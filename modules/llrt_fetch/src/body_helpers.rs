// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{borrow::Cow, sync::RwLock};

use llrt_buffer::Blob;
use llrt_stream_web::{
    readable_byte_stream_controller_close_stream, readable_byte_stream_controller_enqueue_bytes,
    readable_stream_default_controller_close_stream, utils::promise::PromisePrimordials,
    CancelAlgorithm, PullAlgorithm, ReadableStream, ReadableStreamControllerClass,
};
use llrt_utils::bytes::ObjectBytes;
use llrt_utils::primordials::Primordial;
use rquickjs::{
    prelude::This, Class, Ctx, Exception, Function, Object, Promise, Result, TypedArray, Value,
};

/// Creates a ReadableStream from a body value (string, Blob, ArrayBuffer, etc.)
pub(crate) fn create_body_value_stream<'js>(
    ctx: &Ctx<'js>,
    body_value: Value<'js>,
) -> Result<Value<'js>> {
    let pull = PullAlgorithm::from_fn_once(
        move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
            let ctrl = match controller {
                ReadableStreamControllerClass::ReadableStreamByteController(c) => c,
                _ => return Err(Exception::throw_type(&ctx, "Expected byte controller")),
            };

            // Collect bytes from whatever the body value is.
            let bytes: Vec<u8> = if let Ok(ta) = TypedArray::<u8>::from_value(body_value.clone()) {
                ta.as_bytes().unwrap_or(&[]).to_vec()
            } else if let Some(blob) = body_value.as_object().and_then(Class::<Blob>::from_object) {
                blob.borrow().get_bytes()
            } else {
                ObjectBytes::from(&ctx, &body_value)?
                    .as_bytes(&ctx)?
                    .to_vec()
            };

            if !bytes.is_empty() {
                // `new_copy` uses QuickJS-owned storage (no Rust drop
                // callback), which is the only variant that survives being
                // transferred by the byte-controller enqueue path and
                // later GC'd.
                let buf = rquickjs::ArrayBuffer::new_copy(ctx.clone(), &bytes)?;
                readable_byte_stream_controller_enqueue_bytes(ctx.clone(), ctrl.clone(), buf)?;
            }
            readable_byte_stream_controller_close_stream(ctx.clone(), ctrl)?;

            Ok(PromisePrimordials::get(&ctx)?
                .promise_resolved_with_undefined
                .clone())
        },
    );

    // Byte-source stream so `response.body.getReader({mode:'byob'})` works
    // (WPT `response-consume-stream`).
    let stream = ReadableStream::from_byte_pull_algorithm(
        ctx.clone(),
        pull,
        CancelAlgorithm::ReturnPromiseUndefined,
    )?;

    Ok(stream.into_value())
}

/// Creates a ReadableStream that is already closed and locked — used for
/// the `body` getter after the body has been consumed. Holding a reader
/// makes subsequent `getReader()` throw per spec.
pub(crate) fn create_disturbed_stream<'js>(ctx: &Ctx<'js>) -> Result<Value<'js>> {
    let pull = PullAlgorithm::from_fn_once(
        move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
            let ctrl = match controller {
                ReadableStreamControllerClass::ReadableStreamDefaultController(c) => c,
                _ => return Err(Exception::throw_type(&ctx, "Expected default controller")),
            };
            readable_stream_default_controller_close_stream(ctx.clone(), ctrl)?;
            Ok(PromisePrimordials::get(&ctx)?
                .promise_resolved_with_undefined
                .clone())
        },
    );
    let stream = ReadableStream::from_pull_algorithm(
        ctx.clone(),
        pull,
        CancelAlgorithm::ReturnPromiseUndefined,
    )?;
    stream.borrow_mut().disturbed = true;
    // Lock by acquiring a reader so subsequent `getReader()` throws per spec.
    llrt_stream_web::lock_readable_stream(ctx.clone(), stream.clone())?;
    Ok(stream.into_value())
}

/// Collects all data from a ReadableStream into a Vec<u8>
pub(crate) async fn collect_readable_stream<'js>(
    stream: &Class<'js, ReadableStream<'js>>,
) -> Result<Vec<u8>> {
    // Fast path: if the stream is already closed with all its chunks buffered
    // (e.g. created with a synchronous `start()` that enqueues + closes), drain
    // them directly from the controller's queue. Bypasses JS Promise machinery
    // so user code patching `Object.prototype.then` can't intercept the bytes
    // (WPT `response-stream-with-broken-then`).
    if let Some(chunks) = llrt_stream_web::try_sync_drain_closed_stream(stream) {
        let mut result = Vec::new();
        for chunk in chunks {
            if let Ok(typed_array) = TypedArray::<u8>::from_value(chunk.clone()) {
                if let Some(bytes) = typed_array.as_bytes() {
                    result.extend_from_slice(bytes);
                }
            } else {
                return Err(Exception::throw_type(
                    stream.ctx(),
                    "Response body stream chunk must be a Uint8Array",
                ));
            }
        }
        return Ok(result);
    }

    let mut result = Vec::new();

    let get_reader: Function = stream.get("getReader")?;
    let reader: Object = get_reader.call((This(stream.clone()),))?;
    let read_fn: Function = reader.get("read")?;

    loop {
        let promise: Promise = read_fn.call((This(reader.clone()),))?;
        let read_result: Object = promise.into_future().await?;

        let done: bool = read_result.get("done").unwrap_or(true);
        if done {
            break;
        }

        let value: Value = read_result.get("value")?;
        if let Ok(typed_array) = TypedArray::<u8>::from_value(value.clone()) {
            if let Some(bytes) = typed_array.as_bytes() {
                result.extend_from_slice(bytes);
            }
        } else {
            return Err(Exception::throw_type(
                stream.ctx(),
                "Response body stream chunk must be a Uint8Array",
            ));
        }
    }

    Ok(result)
}

pub(crate) fn strip_bom<'a>(bytes: impl Into<Cow<'a, [u8]>>) -> Cow<'a, [u8]> {
    let cow = bytes.into();
    if cow.starts_with(&[0xEF, 0xBB, 0xBF]) {
        match cow {
            Cow::Borrowed(bytes) => Cow::Borrowed(&bytes[3..]),
            Cow::Owned(mut bytes) => {
                bytes.drain(0..3);
                Cow::Owned(bytes)
            },
        }
    } else {
        cow
    }
}

/// Returns true if the cached body stream has been disturbed (read from).
pub(crate) fn is_body_stream_disturbed(body_stream: &RwLock<Option<Value<'_>>>) -> bool {
    if let Some(stream_value) = body_stream.read().unwrap().as_ref() {
        if let Some(stream) = stream_value
            .as_object()
            .and_then(Class::<ReadableStream>::from_object)
        {
            return stream.borrow().disturbed;
        }
    }
    false
}

/// Validates that a ReadableStream is not disturbed or locked.
pub(crate) fn validate_stream_usable<'js>(
    ctx: &Ctx<'js>,
    stream: &Class<'js, ReadableStream<'js>>,
    action: &str,
) -> Result<()> {
    let stream_ref = stream.borrow();
    if stream_ref.disturbed {
        return Err(Exception::throw_type(
            ctx,
            &format!("Cannot {} with a disturbed ReadableStream", action),
        ));
    }
    if stream_ref.is_readable_stream_locked() {
        return Err(Exception::throw_type(
            ctx,
            &format!("Cannot {} with a locked ReadableStream", action),
        ));
    }
    Ok(())
}
