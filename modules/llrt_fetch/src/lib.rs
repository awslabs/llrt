// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub use self::security::{get_allow_list, get_deny_list, set_allow_list, set_deny_list};
use self::{
    form_data::{FormData, FormDataIter},
    headers::{Headers, HeadersIter},
    request::Request,
    response::Response,
};
use llrt_buffer::Blob;
use llrt_http::HTTP_CLIENT;
use llrt_utils::{
    class::{CustomInspectExtension, WebIdlIteratorExtension},
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

#[derive(Clone, Copy)]
pub(crate) enum IteratorKind {
    Keys,
    Values,
    Entries,
}

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    BasePrimordials::init(ctx)?;

    //init eagerly
    fetch::init(HTTP_CLIENT.as_ref().or_throw(ctx)?.clone(), &globals)?;

    Class::<Request>::define(&globals)?;
    Class::<Response>::define(&globals)?;
    Class::<Headers>::define_with_custom_inspect(&globals)?;
    Class::<FormData>::define_with_custom_inspect(&globals)?;

    Class::<HeadersIter>::define_as_webidl_iterator(&globals, stringify!(HeadersIter))?;
    Class::<FormDataIter>::define_as_webidl_iterator(&globals, stringify!(FormDataIter))?;

    Ok(())
}
