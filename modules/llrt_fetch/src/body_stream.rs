// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Body stream adapters for converting Hyper bodies to ReadableStream
//!
//! This module provides utilities to create JavaScript ReadableStream objects
//! from hyper's Incoming body type, enabling streaming of fetch response bodies.

use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;

use bytes::{Bytes, BytesMut};
use http_body_util::BodyExt;
use hyper::body::Body;
use llrt_stream_web::ReadableStreamClass;
use llrt_utils::bytes::ObjectBytes;
use rquickjs::{
    function::Constructor, prelude::This, Class, Ctx, Function, Object, Promise, Result,
    TypedArray, Value,
};

use crate::Blob;
use llrt_utils::mc_oneshot;
use tokio::sync::mpsc;

/// Supported content encodings for streaming decompression
#[derive(Clone, Copy, Debug)]
pub enum ContentEncoding {
    Gzip,
    Deflate,
    Brotli,
    Zstd,
    Identity,
}

impl ContentEncoding {
    /// Parse content-encoding header value
    pub fn from_header(value: Option<&str>) -> Self {
        match value {
            Some("gzip") => ContentEncoding::Gzip,
            Some("deflate") => ContentEncoding::Deflate,
            Some("br") => ContentEncoding::Brotli,
            Some("zstd") => ContentEncoding::Zstd,
            _ => ContentEncoding::Identity,
        }
    }

    /// Returns true if this encoding requires decompression
    pub fn needs_decompression(&self) -> bool {
        !matches!(self, ContentEncoding::Identity)
    }
}

/// Incremental decompressor state that accumulates compressed data
/// and performs decompression when requested.
///
/// Since llrt_compression decoders take ownership of the input reader,
/// we accumulate data and create a fresh decoder for each decompression
/// attempt.
struct DecompressorState {
    /// Accumulated compressed input
    input_buffer: BytesMut,
    /// Encoding type
    encoding: ContentEncoding,
}

/// Incremental streaming decompressor that outputs decompressed data
/// as compressed chunks arrive.
///
/// This implementation accumulates compressed data and attempts decompression
/// periodically. Since the llrt_compression decoders take ownership of their
/// input reader, we create a fresh decoder for each decompression attempt
/// and track how much output we've already sent.
struct IncrementalDecompressor {
    state: DecompressorState,
    /// How many bytes we've already output (to avoid duplicates)
    bytes_output: usize,
}

impl IncrementalDecompressor {
    /// Create a new incremental decompressor for the given encoding.
    /// Returns None for Identity encoding (no decompression needed).
    fn new(encoding: ContentEncoding) -> Option<Self> {
        if !encoding.needs_decompression() {
            return None;
        }

        Some(Self {
            state: DecompressorState {
                input_buffer: BytesMut::new(),
                encoding,
            },
            bytes_output: 0,
        })
    }

    /// Add more compressed data and return any newly decompressed output available.
    fn decompress_chunk(&mut self, chunk: Bytes) -> std::io::Result<Option<Vec<u8>>> {
        // Add the new compressed data
        self.state.input_buffer.extend_from_slice(&chunk);

        // Try to decompress with current data
        self.try_decompress()
    }

    /// Try to decompress accumulated data and return any new output.
    fn try_decompress(&mut self) -> std::io::Result<Option<Vec<u8>>> {
        // Create a cursor over the accumulated input
        let input = std::io::Cursor::new(self.state.input_buffer.as_ref());

        // Create a fresh decoder
        let mut output = Vec::new();
        let mut temp_buf = [0u8; 8192];

        match self.state.encoding {
            ContentEncoding::Gzip => {
                let mut decoder = llrt_compression::gz::decoder(input);
                loop {
                    match decoder.read(&mut temp_buf) {
                        Ok(0) => break,
                        Ok(n) => output.extend_from_slice(&temp_buf[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            // Not enough data yet - this is expected during streaming
                            break;
                        },
                        Err(e) => return Err(e),
                    }
                }
            },
            ContentEncoding::Deflate => {
                let mut decoder = llrt_compression::zlib::decoder(input);
                loop {
                    match decoder.read(&mut temp_buf) {
                        Ok(0) => break,
                        Ok(n) => output.extend_from_slice(&temp_buf[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            break;
                        },
                        Err(e) => return Err(e),
                    }
                }
            },
            ContentEncoding::Brotli => {
                let buf_input = std::io::BufReader::new(input);
                let mut decoder = llrt_compression::brotli::decoder(buf_input);
                loop {
                    match decoder.read(&mut temp_buf) {
                        Ok(0) => break,
                        Ok(n) => output.extend_from_slice(&temp_buf[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                            break;
                        },
                        Err(e) => return Err(e),
                    }
                }
            },
            ContentEncoding::Zstd => {
                let buf_input = std::io::BufReader::new(input);
                match llrt_compression::zstd::decoder(buf_input) {
                    Ok(mut decoder) => loop {
                        match decoder.read(&mut temp_buf) {
                            Ok(0) => break,
                            Ok(n) => output.extend_from_slice(&temp_buf[..n]),
                            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                                break;
                            },
                            Err(e) => return Err(e),
                        }
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                        // Not enough data to initialize decoder
                    },
                    Err(e) => return Err(e),
                }
            },
            ContentEncoding::Identity => unreachable!(),
        }

        // Only return bytes we haven't output yet
        if output.len() > self.bytes_output {
            let new_bytes = output[self.bytes_output..].to_vec();
            self.bytes_output = output.len();
            Ok(Some(new_bytes))
        } else {
            Ok(None)
        }
    }

    /// Signal that no more input is coming and flush any remaining output.
    fn finish(mut self) -> std::io::Result<Option<Vec<u8>>> {
        // Try one final decompression
        self.try_decompress()
    }
}

/// Message types sent from the body reader task to the stream pull callback
pub(crate) enum BodyChunk {
    /// A data frame containing bytes
    Data(Bytes),
    /// End of stream
    End,
    /// An error occurred
    Error(String),
}

/// State shared between the pull callback closure and the body reader task
struct BodySourceState {
    /// Receiver for body chunks from the async reader task
    /// Wrapped in Option so we can temporarily take it for async operations
    receiver: Option<mpsc::Receiver<BodyChunk>>,
    /// Whether the stream has ended
    ended: bool,
}

/// Creates a ReadableStream from a hyper body.
///
/// This function creates a JavaScript ReadableStream that reads data from a hyper
/// Body implementation. It spawns a background task that reads frames from the body
/// and sends them through a channel to the JavaScript pull callback.
///
/// # Arguments
/// * `ctx` - The JavaScript context
/// * `body` - The hyper body to stream
/// * `abort_receiver` - Optional abort signal receiver to cancel the stream
/// * `content_encoding` - Content encoding for decompression (pass ContentEncoding::Identity for no decompression)
///
/// # Returns
/// A JavaScript ReadableStream class instance
pub(crate) fn create_body_stream<'js, B>(
    ctx: &Ctx<'js>,
    body: B,
    abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
    content_encoding: ContentEncoding,
) -> Result<ReadableStreamClass<'js>>
where
    B: Body<Data = Bytes> + Unpin + 'static,
    B::Error: std::fmt::Display,
{
    // Create a channel for sending body chunks to the stream
    let (tx, rx) = mpsc::channel::<BodyChunk>(4);

    // Spawn a background task to read the body
    ctx.spawn(async move {
        let mut body = body;

        if let Some(mut decompressor) = IncrementalDecompressor::new(content_encoding) {
            // With decompression - decompress incrementally as chunks arrive
            if let Some(abort_rx) = abort_receiver {
                // With abort signal
                loop {
                    tokio::select! {
                        frame_result = body.frame() => {
                            match frame_result {
                                Some(Ok(frame)) => {
                                    if let Ok(data) = frame.into_data() {
                                        // Decompress this chunk and send any available output
                                        match decompressor.decompress_chunk(data) {
                                            Ok(Some(decompressed)) => {
                                                if tx.send(BodyChunk::Data(Bytes::from(decompressed))).await.is_err() {
                                                    return;
                                                }
                                            },
                                            Ok(None) => {
                                                // No output yet, continue reading
                                            },
                                            Err(e) => {
                                                let _ = tx.send(BodyChunk::Error(format!("Decompression error: {}", e))).await;
                                                return;
                                            },
                                        }
                                    }
                                },
                                Some(Err(e)) => {
                                    let _ = tx.send(BodyChunk::Error(e.to_string())).await;
                                    return;
                                },
                                None => {
                                    // End of stream - flush any remaining decompressed data
                                    match decompressor.finish() {
                                        Ok(Some(remaining)) => {
                                            let _ = tx.send(BodyChunk::Data(Bytes::from(remaining))).await;
                                        },
                                        Ok(None) => {},
                                        Err(e) => {
                                            let _ = tx.send(BodyChunk::Error(format!("Decompression error: {}", e))).await;
                                            return;
                                        },
                                    }
                                    let _ = tx.send(BodyChunk::End).await;
                                    return;
                                },
                            }
                        }
                        _ = abort_rx.recv() => {
                            let _ = tx.send(BodyChunk::Error("AbortError: The operation was aborted".to_string())).await;
                            return;
                        }
                    }
                }
            } else {
                // Without abort signal
                loop {
                    match body.frame().await {
                        Some(Ok(frame)) => {
                            if let Ok(data) = frame.into_data() {
                                // Decompress this chunk and send any available output
                                match decompressor.decompress_chunk(data) {
                                    Ok(Some(decompressed)) => {
                                        if tx.send(BodyChunk::Data(Bytes::from(decompressed))).await.is_err() {
                                            return;
                                        }
                                    },
                                    Ok(None) => {
                                        // No output yet, continue reading
                                    },
                                    Err(e) => {
                                        let _ = tx.send(BodyChunk::Error(format!("Decompression error: {}", e))).await;
                                        return;
                                    },
                                }
                            }
                        },
                        Some(Err(e)) => {
                            let _ = tx.send(BodyChunk::Error(e.to_string())).await;
                            return;
                        },
                        None => {
                            // End of stream - flush any remaining decompressed data
                            match decompressor.finish() {
                                Ok(Some(remaining)) => {
                                    let _ = tx.send(BodyChunk::Data(Bytes::from(remaining))).await;
                                },
                                Ok(None) => {},
                                Err(e) => {
                                    let _ = tx.send(BodyChunk::Error(format!("Decompression error: {}", e))).await;
                                    return;
                                },
                            }
                            let _ = tx.send(BodyChunk::End).await;
                            return;
                        },
                    }
                }
            }
        } else {
            // No decompression needed - stream chunks directly
            if let Some(abort_rx) = abort_receiver {
                // With abort signal - use select to handle both body reads and abort
                loop {
                    tokio::select! {
                        frame_result = body.frame() => {
                            match frame_result {
                                Some(Ok(frame)) => {
                                    if let Ok(data) = frame.into_data() {
                                        if tx.send(BodyChunk::Data(data)).await.is_err() {
                                            break;
                                        }
                                    }
                                },
                                Some(Err(e)) => {
                                    let _ = tx.send(BodyChunk::Error(e.to_string())).await;
                                    break;
                                },
                                None => {
                                    let _ = tx.send(BodyChunk::End).await;
                                    break;
                                },
                            }
                        }
                        _ = abort_rx.recv() => {
                            let _ = tx.send(BodyChunk::Error("AbortError: The operation was aborted".to_string())).await;
                            break;
                        }
                    }
                }
            } else {
                // Without abort signal - simple loop
                loop {
                    match body.frame().await {
                        Some(Ok(frame)) => {
                            if let Ok(data) = frame.into_data() {
                                if tx.send(BodyChunk::Data(data)).await.is_err() {
                                    break;
                                }
                            }
                        },
                        Some(Err(e)) => {
                            let _ = tx.send(BodyChunk::Error(e.to_string())).await;
                            break;
                        },
                        None => {
                            let _ = tx.send(BodyChunk::End).await;
                            break;
                        },
                    }
                }
            }
        }
    });

    // Create the underlying source object for ReadableStream
    let source = create_underlying_source(ctx, rx)?;

    // Get the global ReadableStream constructor
    let globals = ctx.globals();
    let readable_stream_ctor: Constructor = globals.get("ReadableStream")?;

    // Create the ReadableStream with the underlying source
    let stream: ReadableStreamClass = readable_stream_ctor.construct((source,))?;

    Ok(stream)
}

/// Creates the underlying source object for the ReadableStream.
///
/// The underlying source has a `pull` callback that returns a Promise.
/// When called, it reads the next chunk from the channel and enqueues it
/// to the stream controller.
///
/// This creates a byte stream (type: "bytes") which uses ReadableByteStreamController
/// for efficient binary data handling per the WHATWG Streams spec.
fn create_underlying_source<'js>(
    ctx: &Ctx<'js>,
    receiver: mpsc::Receiver<BodyChunk>,
) -> Result<Object<'js>> {
    let source = Object::new(ctx.clone())?;

    // Set type to "bytes" for ReadableByteStreamController
    // This enables efficient binary streaming with BYOB reader support
    source.set("type", "bytes")?;

    // Create shared state
    let state = Rc::new(RefCell::new(BodySourceState {
        receiver: Some(receiver),
        ended: false,
    }));

    // Create pull callback
    let state_for_pull = state.clone();
    let pull = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, controller: Value<'js>| -> Result<Promise<'js>> {
            let state = state_for_pull.clone();

            // Create a promise that will be resolved when we get the next chunk
            let (promise, resolve, reject) = Promise::new(&ctx)?;

            let controller = controller.clone();
            let ctx_clone = ctx.clone();

            ctx.spawn(async move {
                let ctx = ctx_clone;

                // Take the receiver out of the RefCell to avoid holding borrow across await
                let mut receiver = {
                    let mut state_mut = state.borrow_mut();
                    if state_mut.ended {
                        return;
                    }
                    match state_mut.receiver.take() {
                        Some(r) => r,
                        None => return, // Already taken by another pull
                    }
                };

                // Now we can await without holding the RefCell borrow
                let chunk = receiver.recv().await;

                // Put the receiver back
                {
                    let mut state_mut = state.borrow_mut();
                    state_mut.receiver = Some(receiver);
                }

                match chunk {
                    Some(BodyChunk::Data(data)) => {
                        // Create Uint8Array from the bytes using ArrayBuffer::new_copy
                        // to avoid ownership issues with TypedArray::new that cause
                        // null pointer panics during ArrayBuffer finalization.
                        if let Ok(array_buffer) =
                            rquickjs::ArrayBuffer::new_copy(ctx.clone(), &data)
                        {
                            let globals = ctx.globals();
                            if let Ok(uint8_ctor) = globals.get::<_, Constructor>("Uint8Array") {
                                if let Ok(typed_array) =
                                    uint8_ctor.construct::<_, Value>((array_buffer,))
                                {
                                    if let Some(controller_obj) = controller.as_object() {
                                        if let Ok(enqueue) =
                                            controller_obj.get::<_, Function>("enqueue")
                                        {
                                            let _ = enqueue.call::<_, ()>((
                                                This(controller_obj.clone()),
                                                typed_array,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        let _ = resolve.call::<_, ()>(());
                    },
                    Some(BodyChunk::End) => {
                        state.borrow_mut().ended = true;
                        // Close the stream
                        if let Some(controller_obj) = controller.as_object() {
                            if let Ok(close) = controller_obj.get::<_, Function>("close") {
                                let _ = close.call::<_, ()>((This(controller_obj.clone()),));
                            }
                        }
                        let _ = resolve.call::<_, ()>(());
                    },
                    Some(BodyChunk::Error(msg)) => {
                        state.borrow_mut().ended = true;
                        // Error the stream
                        if let Some(controller_obj) = controller.as_object() {
                            if let Ok(error) = controller_obj.get::<_, Function>("error") {
                                let _ = error.call::<_, ()>((This(controller_obj.clone()), msg));
                            }
                        }
                        let _ = reject.call::<_, ()>(());
                    },
                    None => {
                        // Channel closed unexpectedly
                        state.borrow_mut().ended = true;
                        if let Some(controller_obj) = controller.as_object() {
                            if let Ok(close) = controller_obj.get::<_, Function>("close") {
                                let _ = close.call::<_, ()>((This(controller_obj.clone()),));
                            }
                        }
                        let _ = resolve.call::<_, ()>(());
                    },
                }
            });

            Ok(promise)
        },
    )?;
    source.set("pull", pull)?;

    // Create cancel callback
    let cancel = Function::new(ctx.clone(), move |_ctx: Ctx<'js>, _reason: Value<'js>| {
        // The receiver will be dropped when state is dropped,
        // which will cause the sender task to terminate
        // No explicit action needed here
    })?;
    source.set("cancel", cancel)?;

    Ok(source)
}

/// Creates a ReadableStream from a JavaScript value (string, ArrayBuffer, Blob, etc.)
///
/// This is used for Request.body when the body is a provided value rather than
/// an incoming stream.
///
/// This creates a byte stream (type: "bytes") which uses ReadableByteStreamController
/// for efficient binary data handling per the WHATWG Streams spec.
pub(crate) fn create_value_stream<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Value<'js>> {
    // If the value is null or undefined, return null
    if value.is_null() || value.is_undefined() {
        return Ok(Value::new_null(ctx.clone()));
    }

    // Extract bytes from value BEFORE creating the closure to avoid
    // holding JS values across callback boundaries which can cause
    // memory issues when the callback is invoked.
    let raw_bytes: Option<Vec<u8>> =
        if let Some(blob) = value.as_object().and_then(Class::<Blob>::from_object) {
            // Handle Blob by getting its bytes directly
            let blob = blob.borrow();
            Some(blob.get_bytes())
        } else {
            // Try to convert other types via ObjectBytes
            ObjectBytes::from(ctx, &value)
                .ok()
                .and_then(|bytes| bytes.as_bytes(ctx).ok().map(|b| b.to_vec()))
        };

    // Create an underlying source that yields all data at once
    let source = Object::new(ctx.clone())?;

    // Set type to "bytes" for ReadableByteStreamController
    // This enables efficient binary streaming with BYOB reader support
    source.set("type", "bytes")?;

    // Store bytes in a RefCell so the closure can consume them
    let bytes_cell = Rc::new(RefCell::new(raw_bytes));

    let start = Function::new(ctx.clone(), move |ctx: Ctx<'js>, controller: Value<'js>| {
        // Take bytes from the cell (only happens once)
        if let Some(raw_bytes) = bytes_cell.borrow_mut().take() {
            // Use ArrayBuffer::new_copy to avoid ownership issues with TypedArray::new
            // The TypedArray::new approach was causing null pointer panics during
            // ArrayBuffer finalization due to ownership/lifetime issues.
            if let Ok(array_buffer) = rquickjs::ArrayBuffer::new_copy(ctx.clone(), &raw_bytes) {
                // Create Uint8Array from ArrayBuffer via JS constructor
                let globals = ctx.globals();
                if let Ok(uint8_ctor) = globals.get::<_, Constructor>("Uint8Array") {
                    if let Ok(typed_array) = uint8_ctor.construct::<_, Value>((array_buffer,)) {
                        if let Some(controller_obj) = controller.as_object() {
                            if let Ok(enqueue) = controller_obj.get::<_, Function>("enqueue") {
                                let _ = enqueue
                                    .call::<_, ()>((This(controller_obj.clone()), typed_array));
                            }
                        }
                    }
                }
            }
        }

        // Close the stream
        if let Some(controller_obj) = controller.as_object() {
            if let Ok(close) = controller_obj.get::<_, Function>("close") {
                let _ = close.call::<_, ()>((This(controller_obj.clone()),));
            }
        }

        Ok::<_, rquickjs::Error>(())
    })?;
    source.set("start", start)?;

    // Get the global ReadableStream constructor
    let globals = ctx.globals();
    let readable_stream_ctor: Constructor = globals.get("ReadableStream")?;

    // Create the ReadableStream
    let stream: Value = readable_stream_ctor.construct((source,))?;

    Ok(stream)
}

/// Reads all bytes from a ReadableStream.
///
/// This function consumes the stream by reading all chunks until completion.
/// Used internally when a Request with ReadableStream body needs to be sent via fetch.
pub(crate) async fn read_all_bytes_from_stream<'js>(
    _ctx: &Ctx<'js>,
    stream: ReadableStreamClass<'js>,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();

    // Get the reader from the stream
    let stream_value = stream.into_value();
    let stream_obj = stream_value
        .as_object()
        .ok_or_else(|| rquickjs::Error::new_from_js("stream", "expected ReadableStream object"))?;
    let get_reader: Function = stream_obj.get("getReader")?;
    let reader: Object = get_reader.call((This(stream_obj.clone()),))?;
    let read_fn: Function = reader.get("read")?;

    loop {
        // Call reader.read() which returns a Promise
        let read_promise: Promise = read_fn.call((This(reader.clone()),))?;
        let result: Object = read_promise.into_future().await?;

        let done: bool = result.get("done")?;
        if done {
            break;
        }

        let value: Option<TypedArray<u8>> = result.get("value")?;
        if let Some(chunk) = value {
            let chunk_bytes = chunk
                .as_bytes()
                .ok_or_else(|| rquickjs::Error::new_from_js("value", "detached buffer"))?;
            bytes.extend_from_slice(chunk_bytes);
        }
    }

    // Release the reader lock
    let release_lock: Function = reader.get("releaseLock")?;
    let _ = release_lock.call::<_, ()>((This(reader),));

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http_body_util::Full;
    use llrt_test::test_async_with;
    use rquickjs::CatchResultExt;

    #[tokio::test]
    async fn test_create_body_stream() {
        test_async_with(|ctx| {
            llrt_stream_web::init(&ctx).unwrap();
            Box::pin(async move {
                let run = async {
                    let body = Full::new(Bytes::from("Hello, World!"));
                    let stream = create_body_stream(&ctx, body, None, ContentEncoding::Identity)?;

                    // Verify it's a ReadableStream
                    assert!(!stream.is_null());

                    Ok::<_, rquickjs::Error>(())
                };
                run.await.catch(&ctx).unwrap();
            })
        })
        .await;
    }
}
