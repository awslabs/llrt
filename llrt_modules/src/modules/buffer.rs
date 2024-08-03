// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{
    bytes::{
        get_array_buffer_bytes, get_array_bytes, get_bytes, get_coerced_string_bytes,
        get_start_end_indexes, get_string_bytes, obj_to_array_buffer,
    },
    encoding::{bytes_from_b64, bytes_to_b64_string, Encoder},
    module::export_default,
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, Opt},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, This},
    Array, ArrayBuffer, Coerced, Ctx, Exception, IntoJs, Object, Result, TypedArray, Value,
};

use crate::module_info::ModuleInfo;

pub struct Buffer(pub Vec<u8>);

impl<'js> IntoJs<'js> for Buffer {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let array_buffer = ArrayBuffer::new(ctx.clone(), self.0)?;
        Self::from_array_buffer(ctx, array_buffer)
    }
}

impl<'js> Buffer {
    pub fn alloc(length: usize) -> Self {
        Self(vec![0; length])
    }

    pub fn to_string(&self, ctx: &Ctx<'js>, encoding: &str) -> Result<String> {
        Encoder::from_str(encoding)
            .and_then(|enc| enc.encode_to_string(self.0.as_ref(), true))
            .or_throw(ctx)
    }

    fn from_array_buffer(ctx: &Ctx<'js>, buffer: ArrayBuffer<'js>) -> Result<Value<'js>> {
        let constructor: Constructor = ctx.globals().get(stringify!(Buffer))?;
        constructor.construct((buffer,))
    }

    fn from_array_buffer_offset_length(
        ctx: &Ctx<'js>,
        array_buffer: ArrayBuffer<'js>,
        offset: usize,
        length: usize,
    ) -> Result<Value<'js>> {
        let constructor: Constructor = ctx.globals().get(stringify!(Buffer))?;
        constructor.construct((array_buffer, offset, length))
    }

    fn from_encoding(
        ctx: &Ctx<'js>,
        mut bytes: Vec<u8>,
        encoding: Option<String>,
    ) -> Result<Value<'js>> {
        if let Some(encoding) = encoding {
            let encoder = Encoder::from_str(&encoding).or_throw(ctx)?;
            bytes = encoder.decode(bytes).or_throw(ctx)?;
        }
        Buffer(bytes).into_js(ctx)
    }
}

fn byte_length<'js>(ctx: Ctx<'js>, value: Value<'js>, encoding: Opt<String>) -> Result<usize> {
    //slow path
    if let Some(encoding) = encoding.0 {
        let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
        let bytes = get_bytes(&ctx, value)?;
        return Ok(encoder.decode(bytes).or_throw(&ctx)?.len());
    }
    //fast path
    if let Some(val) = value.as_string() {
        return Ok(val.to_string()?.len());
    }

    if value.is_array() {
        let array = value.as_array().unwrap();

        for val in array.iter::<u8>() {
            val.or_throw_msg(&ctx, "array value is not u8")?;
        }

        return Ok(array.len());
    }

    if let Some(obj) = value.as_object() {
        if let Some((_, source_length, _)) = obj_to_array_buffer(obj)? {
            return Ok(source_length);
        }
    }

    Err(Exception::throw_message(
        &ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or string",
    ))
}

fn to_string(this: This<Object<'_>>, ctx: Ctx, encoding: Opt<String>) -> Result<String> {
    let typed_array = TypedArray::<u8>::from_object(this.0)?;
    let bytes: &[u8] = typed_array.as_ref();
    let encoding = encoding.0.unwrap_or_else(|| String::from("utf-8"));
    let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
    encoder.encode_to_string(bytes, true).or_throw(&ctx)
}

fn alloc<'js>(
    ctx: Ctx<'js>,
    length: usize,
    fill: Opt<Value<'js>>,
    encoding: Opt<String>,
) -> Result<Value<'js>> {
    if let Some(value) = fill.0 {
        if let Some(value) = value.as_string() {
            let string = value.to_string()?;

            if let Some(encoding) = encoding.0 {
                let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
                let bytes = encoder.decode_from_string(string).or_throw(&ctx)?;
                return alloc_byte_ref(&ctx, &bytes, length);
            }

            let byte_ref = string.as_bytes();

            return alloc_byte_ref(&ctx, byte_ref, length);
        }
        if let Some(value) = value.as_int() {
            let bytes = vec![value as u8; length];
            return Buffer(bytes).into_js(&ctx);
        }
        if let Some(obj) = value.as_object() {
            if let Some((array_buffer, source_length, offset)) = obj_to_array_buffer(obj)? {
                let bytes: &[u8] = array_buffer.as_ref();
                return alloc_byte_ref(&ctx, &bytes[offset..offset + source_length], length);
            }
        }
    }

    Buffer(vec![0; length]).into_js(&ctx)
}

fn alloc_byte_ref<'js>(ctx: &Ctx<'js>, byte_ref: &[u8], length: usize) -> Result<Value<'js>> {
    let mut bytes = vec![0; length];
    let byte_ref_length = byte_ref.len();
    for i in 0..length {
        bytes[i] = byte_ref[i % byte_ref_length];
    }
    Buffer(bytes).into_js(ctx)
}

fn concat<'js>(ctx: Ctx<'js>, list: Array<'js>, max_length: Opt<usize>) -> Result<Value<'js>> {
    let mut bytes = Vec::new();
    let mut total_length = 0;
    let mut length;
    for value in list.iter::<Object>() {
        let typed_array = TypedArray::<u8>::from_object(value?)?;
        let bytes_ref: &[u8] = typed_array.as_ref();

        length = bytes_ref.len();

        if length == 0 {
            continue;
        }

        if let Some(max_length) = max_length.0 {
            total_length += length;
            if total_length > max_length {
                let diff = max_length - (total_length - length);
                bytes.extend_from_slice(&bytes_ref[0..diff]);
                break;
            }
        }
        bytes.extend_from_slice(bytes_ref);
    }

    Buffer(bytes).into_js(&ctx)
}

fn from<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    offset_or_encoding: Opt<Value<'js>>,
    length: Opt<usize>,
) -> Result<Value<'js>> {
    let mut encoding: Option<String> = None;
    let mut offset = 0;

    if let Some(offset_or_encoding) = offset_or_encoding.0 {
        if offset_or_encoding.is_string() {
            encoding = Some(offset_or_encoding.get()?);
        } else if offset_or_encoding.is_number() {
            offset = offset_or_encoding.get()?;
        }
    }

    if let Some(bytes) = get_string_bytes(&value, offset, length.0)? {
        return Buffer::from_encoding(&ctx, bytes, encoding)?.into_js(&ctx);
    }
    if let Some(bytes) = get_array_bytes(&ctx, &value, offset, length.0)? {
        return Buffer::from_encoding(&ctx, bytes, encoding)?.into_js(&ctx);
    }

    if let Some(obj) = value.as_object() {
        if let Some((array_buffer, source_length, source_offset)) = obj_to_array_buffer(obj)? {
            let (start, end) = get_start_end_indexes(source_length, length.0, offset);

            //buffers from buffer should be copied
            if obj
                .get::<_, Option<String>>(PredefinedAtom::Meta)?
                .as_deref()
                == Some(stringify!(Buffer))
                || encoding.is_some()
            {
                let bytes = get_array_buffer_bytes(
                    array_buffer,
                    start + source_offset,
                    end - source_offset,
                );
                return Buffer::from_encoding(&ctx, bytes, encoding)?.into_js(&ctx);
            } else {
                return Buffer::from_array_buffer_offset_length(
                    &ctx,
                    array_buffer,
                    start + source_offset,
                    end - start,
                );
            }
        }
    }

    if let Some(bytes) = get_coerced_string_bytes(&value, offset, length.0) {
        return Buffer::from_encoding(&ctx, bytes, encoding)?.into_js(&ctx);
    }

    Err(Exception::throw_message(
        &ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or interpretable as string",
    ))
}

fn set_prototype<'js>(ctx: &Ctx<'js>, constructor: Object<'js>) -> Result<()> {
    let _ = &constructor.set(PredefinedAtom::From, Func::from(from))?;
    let _ = &constructor.set(stringify!(alloc), Func::from(alloc))?;
    let _ = &constructor.set(stringify!(concat), Func::from(concat))?;
    let _ = &constructor.set("byteLength", Func::from(byte_length))?;

    let prototype: &Object = &constructor.get(PredefinedAtom::Prototype)?;
    prototype.set(PredefinedAtom::ToString, Func::from(to_string))?;
    //not assessable from js
    prototype.prop(PredefinedAtom::Meta, stringify!(Buffer))?;

    ctx.globals().set(stringify!(Buffer), constructor)?;

    Ok(())
}

pub fn atob(ctx: Ctx<'_>, encoded_value: Coerced<String>) -> Result<rquickjs::String<'_>> {
    let vec = bytes_from_b64(encoded_value.as_bytes()).or_throw(&ctx)?;
    // SAFETY: QuickJS will replace invalid characters with U+FFFD
    let str = unsafe { String::from_utf8_unchecked(vec) };
    rquickjs::String::from_str(ctx, &str)
}

pub fn btoa(value: Coerced<String>) -> String {
    bytes_to_b64_string(value.as_bytes())
}

pub fn init<'js>(ctx: &Ctx<'js>) -> Result<()> {
    // Buffer
    let buffer = ctx.eval::<Object<'js>, &str>(concat!(
        "class ",
        stringify!(Buffer),
        " extends Uint8Array {}\n",
        stringify!(Buffer),
    ))?;
    set_prototype(ctx, buffer)?;

    // Conversion
    let globals = ctx.globals();
    globals.set("atob", Func::from(atob))?;
    globals.set("btoa", Func::from(btoa))?;

    Ok(())
}

pub struct BufferModule;

impl ModuleDef for BufferModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(Buffer))?;
        declare.declare("atob")?;
        declare.declare("btoa")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let buf: Constructor = globals.get(stringify!(Buffer))?;

        export_default(ctx, exports, |default| {
            default.set(stringify!(Buffer), buf)?;
            default.set("atob", Func::from(atob))?;
            default.set("btoa", Func::from(btoa))?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<BufferModule> for ModuleInfo<BufferModule> {
    fn from(val: BufferModule) -> Self {
        ModuleInfo {
            name: "buffer",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{call_test, test_async_with, ModuleEvaluator};

    #[tokio::test]
    async fn test_atob() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let data = "aGVsbG8gd29ybGQ=".to_string();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { atob } from 'buffer';

                        export async function test(data) {
                            return atob(data);
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, (data,)).await;
                assert_eq!(result, "hello world");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_atob_invalid_utf8() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let data = "aGVsbG/Ad29ybGQ=".to_string();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { atob } from 'buffer';

                        export async function test(data) {
                            return atob(data);
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, (data,)).await;
                assert_eq!(result, "hello�world");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_btoa() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let data = "hello world".to_string();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { btoa } from 'buffer';

                        export async function test(data) {
                            return btoa(data);
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, (data,)).await;
                assert_eq!(result, "aGVsbG8gd29ybGQ=");
            })
        })
        .await;
    }
}
