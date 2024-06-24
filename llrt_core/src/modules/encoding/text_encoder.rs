// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{Ctx, Object, Result, TypedArray, Value};

use crate::utils::{object::obj_to_array_buffer, result::ResultExt};

#[derive(rquickjs::class::Trace)]
#[rquickjs::class]
pub struct TextEncoder {}

#[rquickjs::methods(rename_all = "camelCase")]
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

    pub fn encode_into<'js>(
        &self,
        ctx: Ctx<'js>,
        string: String,
        obj: Object<'js>,
    ) -> Result<Object<'js>> {
        let mut read = 0;
        let mut written = 0;

        if let Some((array_buffer, source_length, source_offset)) = obj_to_array_buffer(&obj)? {
            let raw = array_buffer
                .as_raw()
                .ok_or("ArrayBuffer is detached")
                .or_throw(&ctx)?;

            let bytes = unsafe {
                std::slice::from_raw_parts_mut(raw.ptr.as_ptr().add(source_offset), source_length)
            };

            let mut enc = encoding_rs::UTF_8.new_encoder();
            (_, _, written, _) = enc.encode_from_utf8(string.as_str(), bytes, false);
            read = string[..written]
                .chars()
                .fold(0, |acc, ch| acc + ch.len_utf16());
        }

        let obj = Object::new(ctx)?;
        obj.set("read", read)?;
        obj.set("written", written)?;
        Ok(obj)
    }
}
