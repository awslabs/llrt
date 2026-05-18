// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub use self::security::{get_allow_list, get_deny_list, set_allow_list, set_deny_list};
use self::{
    form_data::FormData,
    headers::{Headers, HeadersIter},
    request::Request,
    response::Response,
};
use llrt_buffer::Blob;
use llrt_http::HTTP_CLIENT;
use llrt_utils::{
    class::CustomInspectExtension,
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{Class, Ctx, Result};

pub(crate) mod body_helpers;
pub mod fetch;
pub mod form_data;
pub mod headers;
pub(crate) mod integrity;
pub mod request;
pub mod response;
mod security;
pub mod utils;

const MIME_TYPE_FORM_URLENCODED: &str = "application/x-www-form-urlencoded;charset=UTF-8";
const MIME_TYPE_TEXT: &str = "text/plain;charset=UTF-8";
const MIME_TYPE_JSON_STATIC: &str = "application/json";
const MIME_TYPE_FORM_DATA: &str = "multipart/form-data; boundary=";
const MIME_TYPE_OCTET_STREAM: &str = "application/octet-stream";

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    BasePrimordials::init(ctx)?;

    //init eagerly
    fetch::init(HTTP_CLIENT.as_ref().or_throw(ctx)?.clone(), &globals)?;

    Class::<FormData>::define(&globals)?;

    Class::<Request>::define(&globals)?;
    Class::<Response>::define(&globals)?;
    Class::<Headers>::define_with_custom_inspect(&globals)?;

    // Set up HeadersIter prototype chain:
    // iter -> HeadersIter.prototype -> %IteratorPrototype%
    // And make `next` enumerable on the prototype per WebIDL.
    Class::<HeadersIter>::define(&globals)?;
    if let Some(proto) = Class::<HeadersIter>::prototype(ctx)? {
        // Get %IteratorPrototype% via [][Symbol.iterator]().__proto__.__proto__
        let array_iter_proto: rquickjs::Object =
            ctx.eval("Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()))")?;
        proto.set_prototype(Some(&array_iter_proto))?;

        // Make `next` enumerable via defineProperty (rquickjs's `prop()` helper
        // doesn't carry HAS_ENUMERABLE, so the enumerable flag is ignored).
        globals.set("__headersIterProto", proto.clone())?;
        let _ = ctx.eval::<(), _>(
            r#"(() => {
                const p = globalThis.__headersIterProto;
                const v = p.next;
                Object.defineProperty(p, 'next', {
                    value: v, writable: true, enumerable: true, configurable: true
                });
            })()"#,
        );
        globals.remove("__headersIterProto")?;
    }
    // Remove HeadersIter from globals (it's an internal class)
    globals.remove("HeadersIter")?;

    Ok(())
}
