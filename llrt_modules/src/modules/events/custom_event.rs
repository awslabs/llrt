// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{prelude::Opt, Ctx, IntoJs, Null, Result, Value};

use llrt_utils::object::ObjectExt;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct CustomEvent<'js> {
    event_type: String,
    detail: Option<Value<'js>>,
}

#[rquickjs::methods]
impl<'js> CustomEvent<'js> {
    #[qjs(constructor)]
    pub fn new(_ctx: Ctx<'js>, event_type: String, options: Opt<Value<'js>>) -> Result<Self> {
        let mut detail = None;
        if let Some(options) = options.0 {
            if let Some(opt) = options.get_optional("detail")? {
                detail = opt;
            }
        }
        Ok(Self { event_type, detail })
    }

    #[qjs(get)]
    pub fn detail(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(detail) = &self.detail {
            return Ok(detail.clone());
        }
        Null.into_js(&ctx)
    }

    #[qjs(get, rename = "type")]
    pub fn event_type(&self) -> String {
        self.event_type.clone()
    }
}
