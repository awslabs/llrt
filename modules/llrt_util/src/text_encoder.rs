// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::result::ResultExt;
use rquickjs::{
    atom::PredefinedAtom, function::Opt, Ctx, Exception, Object, Result, TypedArray, Value,
};

#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct TextEncoder {}

impl Default for TextEncoder {
    fn default() -> Self {
        Self::new()
    }
}

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

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(TextEncoder)
    }

    pub fn encode<'js>(&self, ctx: Ctx<'js>, string: Opt<Value<'js>>) -> Result<Value<'js>> {
        if let Some(string) = string.0 {
            if let Some(string) = string.as_string() {
                let string = string.to_string()?;
                return TypedArray::new(ctx.clone(), string.as_bytes())
                    .map(|m: TypedArray<'_, u8>| m.into_value());
            } else if !string.is_undefined() {
                return Err(Exception::throw_message(
                    &ctx,
                    "The \"string\" argument must be a string.",
                ));
            }
        }

        TypedArray::new(ctx.clone(), []).map(|m: TypedArray<'_, u8>| m.into_value())
    }

    pub fn encode_into<'js>(
        &self,
        ctx: Ctx<'js>,
        src: String,
        dst: Value<'js>,
    ) -> Result<Object<'js>> {
        if let Ok(typed_array) = TypedArray::<u8>::from_value(dst) {
            let dst_length = typed_array.len();
            let dst_offset: usize = typed_array.get("byteOffset")?;
            let array_buffer = typed_array.arraybuffer()?;
            let raw = array_buffer
                .as_raw()
                .ok_or("ArrayBuffer is detached")
                .or_throw(&ctx)?;

            let dst = unsafe {
                std::slice::from_raw_parts_mut(raw.ptr.as_ptr().add(dst_offset), dst_length)
            };

            let mut written = 0;
            let dst_len = dst.len();
            for ch in src.chars() {
                let len = ch.len_utf8();
                if written + len > dst_len {
                    break;
                }
                written += len;
            }
            dst[..written].copy_from_slice(&src.as_bytes()[..written]);
            let read: usize = src[..written].chars().map(char::len_utf16).sum();

            let obj = Object::new(ctx)?;
            obj.set("read", read)?;
            obj.set("written", written)?;
            Ok(obj)
        } else {
            Err(Exception::throw_type(
                &ctx,
                "The \"dest\" argument must be an instance of Uint8Array.",
            ))
        }
    }
}
