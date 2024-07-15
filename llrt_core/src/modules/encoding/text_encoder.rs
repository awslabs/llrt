// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::obj_to_array_buffer, result::ResultExt};
use rquickjs::{Ctx, Object, Result, TypedArray, Value};

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
    fn encoding(&self) -> &str {
        "utf-8"
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

            let bytes_len = bytes.len();
            written = string
                .chars()
                .take_while(|ch| (written + ch.len_utf8()) <= bytes_len)
                .map(|ch| ch.len_utf8())
                .sum();
            bytes[..written].copy_from_slice(&string.as_bytes()[..written]);
            read = string[..written].chars().map(|ch| ch.len_utf16()).sum();
        }

        let obj = Object::new(ctx)?;
        obj.set("read", read)?;
        obj.set("written", written)?;
        Ok(obj)
    }
}
