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

mod body;
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
