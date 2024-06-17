// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{Ctx, Result, TypedArray, Value};

#[derive(rquickjs::class::Trace)]
#[rquickjs::class]
pub struct TextEncoder {}

#[rquickjs::methods]
impl TextEncoder {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[qjs(get)]
    fn encoding(&self) -> String {
        "utf-8".to_string()
    }

    pub fn encode<'js>(&self, ctx: Ctx<'js>, string: String) -> Result<Value<'js>> {
        TypedArray::new(ctx, string.as_bytes()).map(|m| m.into_value())
    }
}
