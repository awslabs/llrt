// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_buffer::Blob;
use llrt_http::HTTP_CLIENT;
use llrt_utils::{
    class::CustomInspectExtension,
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{Class, Ctx, Result};
use std::borrow::Cow;

pub use self::security::{get_allow_list, get_deny_list, set_allow_list, set_deny_list};
use self::{form_data::FormData, headers::Headers, request::Request, response::Response};
use llrt_stream_web::{
    readable_stream_default_controller_close_stream,
    readable_stream_default_controller_enqueue_value, CancelAlgorithm, PromisePrimordials,
    PullAlgorithm, ReadableStream, ReadableStreamControllerClass,
    ReadableStreamDefaultControllerClass,
};
use llrt_utils::bytes::ObjectBytes;
use rquickjs::{Exception, TypedArray};
use std::{cell::RefCell, rc::Rc};

mod decompress;
pub mod fetch;
pub mod form_data;
pub mod headers;
pub mod request;
pub mod response;
mod security;

const MIME_TYPE_FORM_URLENCODED: &str = "application/x-www-form-urlencoded;charset=UTF-8";
const MIME_TYPE_TEXT: &str = "text/plain;charset=UTF-8";
const MIME_TYPE_JSON: &str = "application/json;charset=UTF-8";
const MIME_TYPE_FORM_DATA: &str = "multipart/form-data; boundary=";
const MIME_TYPE_OCTET_STREAM: &str = "application/octet-stream";

/// Tee a ReadableStream for cloning. Returns (branch1, branch2) or error if stream is disturbed/locked.
pub(crate) fn tee_stream_for_clone<'js>(
    ctx: &Ctx<'js>,
    stream: &Class<'js, llrt_stream_web::ReadableStream<'js>>,
    entity: &str,
) -> Result<(
    Class<'js, llrt_stream_web::ReadableStream<'js>>,
    Class<'js, llrt_stream_web::ReadableStream<'js>>,
)> {
    use rquickjs::Exception;

    {
        let stream_ref = stream.borrow();
        if stream_ref.disturbed {
            return Err(Exception::throw_type(
                ctx,
                &format!("Cannot clone {} with disturbed body", entity),
            ));
        }
        if stream_ref.is_readable_stream_locked() {
            return Err(Exception::throw_type(
                ctx,
                &format!("Cannot clone {} with locked body", entity),
            ));
        }
    }

    llrt_stream_web::tee_readable_stream(ctx.clone(), stream.clone())
}

/// Creates a ReadableStream from a body value (string, Blob, ArrayBuffer, etc.)
pub(crate) fn create_body_value_stream<'js>(
    ctx: &Ctx<'js>,
    body_value: rquickjs::Value<'js>,
) -> Result<rquickjs::Value<'js>> {
    let body_data: Rc<RefCell<Option<rquickjs::Value<'js>>>> =
        Rc::new(RefCell::new(Some(body_value)));

    let pull = PullAlgorithm::from_fn(
        move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
            let body_data = body_data.clone();

            let ctrl_class: ReadableStreamDefaultControllerClass = match controller {
                ReadableStreamControllerClass::ReadableStreamDefaultController(c) => c,
                _ => return Err(Exception::throw_type(&ctx, "Expected default controller")),
            };

            let data = body_data.borrow_mut().take();

            if let Some(value) = data {
                let bytes =
                    if let Some(blob) = value.as_object().and_then(Class::<Blob>::from_object) {
                        blob.borrow().get_bytes()
                    } else {
                        ObjectBytes::from(&ctx, &value)?.as_bytes(&ctx)?.to_vec()
                    };

                let array = TypedArray::<u8>::new(ctx.clone(), bytes)?;
                readable_stream_default_controller_enqueue_value(
                    ctx.clone(),
                    ctrl_class.clone(),
                    array.into_value(),
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

pub(crate) fn strip_bom<'a>(bytes: impl Into<Cow<'a, [u8]>>) -> Cow<'a, [u8]> {
    let cow = bytes.into();
    if cow.starts_with(&[0xEF, 0xBB, 0xBF]) {
        match cow {
            Cow::Borrowed(bytes) => Cow::Borrowed(&bytes[3..]),
            Cow::Owned(mut bytes) => {
                bytes.drain(0..3); //memmove instead of copy
                Cow::Owned(bytes)
            },
        }
    } else {
        cow
    }
}

/// Collects all data from a ReadableStream into a Vec<u8>
pub(crate) async fn collect_readable_stream<'js>(
    stream: &rquickjs::Class<'js, llrt_stream_web::ReadableStream<'js>>,
) -> Result<Vec<u8>> {
    use rquickjs::function::This;
    use rquickjs::{Function, Object, Promise, Value};

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
        if let Ok(typed_array) = rquickjs::TypedArray::<u8>::from_value(value) {
            if let Some(bytes) = typed_array.as_bytes() {
                result.extend_from_slice(bytes);
            }
        }
    }

    Ok(result)
}

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    BasePrimordials::init(ctx)?;

    //init eagerly
    fetch::init(HTTP_CLIENT.as_ref().or_throw(ctx)?.clone(), &globals)?;

    Class::<FormData>::define(&globals)?;

    Class::<Request>::define(&globals)?;
    Class::<Response>::define(&globals)?;
    Class::<Headers>::define_with_custom_inspect(&globals)?;

    Ok(())
}
