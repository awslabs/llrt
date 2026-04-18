// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{borrow::Cow, cell::RefCell, rc::Rc, sync::RwLock};

use llrt_buffer::Blob;
use llrt_stream_web::{
    readable_stream_default_controller_close_stream,
    readable_stream_default_controller_enqueue_value, utils::promise::PromisePrimordials,
    CancelAlgorithm, PullAlgorithm, ReadableStream, ReadableStreamControllerClass,
    ReadableStreamDefaultControllerClass,
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
    let body_data: Rc<RefCell<Option<Value<'js>>>> = Rc::new(RefCell::new(Some(body_value)));

    let pull = PullAlgorithm::from_fn(
        move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
            let body_data = body_data.clone();

            let ctrl_class: ReadableStreamDefaultControllerClass = match controller {
                ReadableStreamControllerClass::ReadableStreamDefaultController(c) => c,
                _ => return Err(Exception::throw_type(&ctx, "Expected default controller")),
            };

            let data = body_data.borrow_mut().take();

            if let Some(value) = data {
                let array = if TypedArray::<u8>::from_value(value.clone()).is_ok() {
                    value
                } else {
                    let bytes = if let Some(blob) =
                        value.as_object().and_then(Class::<Blob>::from_object)
                    {
                        blob.borrow().get_bytes()
                    } else {
                        ObjectBytes::from(&ctx, &value)?.as_bytes(&ctx)?.to_vec()
                    };
                    TypedArray::<u8>::new(ctx.clone(), bytes)?.into_value()
                };

                readable_stream_default_controller_enqueue_value(
                    ctx.clone(),
                    ctrl_class.clone(),
                    array,
                )?;
                readable_stream_default_controller_close_stream(ctx.clone(), ctrl_class)?;
            } else {
                readable_stream_default_controller_close_stream(ctx.clone(), ctrl_class)?;
            }

            let primordials = PromisePrimordials::get(&ctx)?;
            Ok(primordials.promise_resolved_with_undefined.clone())
        },
    );

    let stream = ReadableStream::from_pull_algorithm(
        ctx.clone(),
        pull,
        CancelAlgorithm::ReturnPromiseUndefined,
    )?;

    Ok(stream.into_value())
}

/// Collects all data from a ReadableStream into a Vec<u8>
pub(crate) async fn collect_readable_stream<'js>(
    stream: &Class<'js, ReadableStream<'js>>,
) -> Result<Vec<u8>> {
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
        if let Ok(typed_array) = TypedArray::<u8>::from_value(value) {
            if let Some(bytes) = typed_array.as_bytes() {
                result.extend_from_slice(bytes);
            }
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
