// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{mem::MaybeUninit, slice};

use llrt_encoding::{bytes_from_b64, bytes_to_b64_string, Encoder};
use llrt_utils::{
    bytes::{
        get_array_bytes, get_coerced_string_bytes, get_start_end_indexes, get_string_bytes,
        ObjectBytes,
    },
    error_messages::ERROR_MSG_ARRAY_BUFFER_DETACHED,
    module::{export_default, ModuleInfo},
    primordials::Primordial,
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, Opt},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest, This},
    Array, ArrayBuffer, Coerced, Ctx, Exception, IntoJs, JsLifetime, Object, Result, TypedArray,
    Value,
};

#[derive(JsLifetime)]
pub struct BufferPrimordials<'js> {
    constructor: Constructor<'js>,
}

impl<'js> Primordial<'js> for BufferPrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self>
    where
        Self: Sized,
    {
        let constructor: Constructor = ctx.globals().get(stringify!(Buffer))?;

        Ok(Self { constructor })
    }
}

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
        BufferPrimordials::get(ctx)?
            .constructor
            .construct((buffer,))
    }

    fn from_array_buffer_offset_length(
        ctx: &Ctx<'js>,
        array_buffer: ArrayBuffer<'js>,
        offset: usize,
        length: usize,
    ) -> Result<Value<'js>> {
        BufferPrimordials::get(ctx)?
            .constructor
            .construct((array_buffer, offset, length))
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

// Static Methods
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
            if let Some(ob) = ObjectBytes::from_array_buffer(obj)? {
                let bytes = ob.as_bytes(&ctx)?;
                return alloc_byte_ref(&ctx, bytes, length);
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

fn alloc_unsafe(ctx: Ctx<'_>, size: usize) -> Result<Value<'_>> {
    let mut bytes: Vec<MaybeUninit<u8>> = Vec::with_capacity(size);
    unsafe {
        bytes.set_len(size);
    }

    Buffer(maybeuninit_to_u8(bytes)).into_js(&ctx)
}

fn maybeuninit_to_u8(vec: Vec<MaybeUninit<u8>>) -> Vec<u8> {
    let len = vec.len();
    let capacity = vec.capacity();
    let ptr = vec.as_ptr() as *mut u8;

    std::mem::forget(vec);

    unsafe { Vec::from_raw_parts(ptr, len, capacity) }
}

fn alloc_unsafe_slow(ctx: Ctx<'_>, size: usize) -> Result<Value<'_>> {
    let layout = std::alloc::Layout::array::<u8>(size).or_throw(&ctx)?;

    let bytes = unsafe {
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            return Err(Exception::throw_internal(&ctx, "Memory allocation failed"));
        }
        Vec::from_raw_parts(ptr, size, size)
    };
    Buffer(bytes).into_js(&ctx)
}

fn byte_length<'js>(ctx: Ctx<'js>, value: Value<'js>, encoding: Opt<String>) -> Result<usize> {
    //slow path
    if let Some(encoding) = encoding.0 {
        let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;
        let a = ObjectBytes::from(&ctx, &value)?;
        let bytes = a.as_bytes(&ctx)?;
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
        if let Some(ob) = ObjectBytes::from_array_buffer(obj)? {
            return Ok(ob.as_bytes(&ctx)?.len());
        }
    }

    Err(Exception::throw_message(
        &ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or string",
    ))
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
    if let Some(bytes) = get_array_bytes(&value, offset, length.0)? {
        return Buffer::from_encoding(&ctx, bytes, encoding)?.into_js(&ctx);
    }

    if let Some(obj) = value.as_object() {
        if let Some(ab_bytes) = ObjectBytes::from_array_buffer(obj)? {
            let bytes = ab_bytes.as_bytes(&ctx)?;
            let (start, end) = get_start_end_indexes(bytes.len(), length.0, offset);

            //buffers from buffer should be copied
            if obj
                .get::<_, Option<String>>(PredefinedAtom::Meta)?
                .as_deref()
                == Some(stringify!(Buffer))
                || encoding.is_some()
            {
                let bytes = bytes.into();
                return Buffer::from_encoding(&ctx, bytes, encoding)?.into_js(&ctx);
            } else {
                let (array_buffer, _, source_offset) = ab_bytes.get_array_buffer()?.unwrap(); //we know it's an array buffer
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

fn is_buffer<'js>(ctx: Ctx<'js>, value: Value<'js>) -> Result<bool> {
    if let Some(object) = value.as_object() {
        let constructor = BufferPrimordials::get(&ctx)?;
        return Ok(object.is_instance_of(&constructor.constructor));
    }

    Ok(false)
}

fn is_encoding(value: Value) -> Result<bool> {
    if let Some(js_string) = value.as_string() {
        let std_string = js_string.to_string()?;
        return Ok(Encoder::from_str(std_string.as_str()).is_ok());
    }

    Ok(false)
}

// Prototype Methods
fn copy<'js>(
    this: This<Object<'js>>,
    ctx: Ctx<'js>,
    target: ObjectBytes<'js>,
    args: Rest<usize>,
) -> Result<usize> {
    let mut args_iter = args.0.into_iter();
    let target_start = args_iter.next().unwrap_or_default();
    let source_start = args_iter.next().unwrap_or_default();
    let source_end = args_iter.next().unwrap_or_else(|| this.0.len());

    let mut copyable_length = 0;

    if source_start >= source_end {
        return Ok(copyable_length);
    }

    let source_bytes = ObjectBytes::from(&ctx, this.0.as_inner())?;
    let source_bytes = source_bytes.as_bytes(&ctx)?;

    if let Some((array_buffer, _, _)) = target.get_array_buffer()? {
        let raw = array_buffer
            .as_raw()
            .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            .or_throw(&ctx)?;

        let target_bytes = unsafe { slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len) };

        copyable_length = (source_end - source_start).min(raw.len - target_start);

        target_bytes[target_start..target_start + copyable_length]
            .copy_from_slice(&source_bytes[source_start..source_start + copyable_length]);
    }

    Ok(copyable_length)
}

fn subarray<'js>(
    this: This<Object<'js>>,
    ctx: Ctx<'js>,
    start: Opt<isize>,
    end: Opt<isize>,
) -> Result<Value<'js>> {
    let typed_array = TypedArray::<u8>::from_object(this.0)?;
    let array_buffer = typed_array.arraybuffer()?;
    let ab_length = array_buffer.len() as isize;
    let offset = start.map_or(0, |start| {
        if start < 0 {
            (ab_length + start).max(0) as usize
        } else {
            start.min(ab_length) as usize
        }
    });

    let end_index = end.map_or(ab_length, |end| {
        if end < 0 {
            (ab_length + end).max(0)
        } else {
            end.min(ab_length)
        }
    });

    let length = (end_index as usize).saturating_sub(offset);

    Buffer::from_array_buffer_offset_length(&ctx, array_buffer, offset, length)
}

fn to_string(this: This<Object<'_>>, ctx: Ctx, encoding: Opt<String>) -> Result<String> {
    let typed_array = TypedArray::<u8>::from_object(this.0)?;
    let bytes: &[u8] = typed_array.as_ref();

    let encoder = Encoder::from_optional_str(encoding.as_deref()).or_throw(&ctx)?;
    encoder.encode_to_string(bytes, true).or_throw(&ctx)
}

fn write<'js>(
    this: This<Object<'js>>,
    ctx: Ctx<'js>,
    string: String,
    args: Rest<Value<'js>>,
) -> Result<usize> {
    let (offset, length, encoding) = get_write_parameters(&args, this.0.len())?;

    let target = ObjectBytes::from(&ctx, this.0.as_inner())?;

    let mut writable_length = 0;

    if let Some((array_buffer, _, _)) = target.get_array_buffer()? {
        let raw = array_buffer
            .as_raw()
            .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            .or_throw(&ctx)?;

        let target_bytes = unsafe { slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len) };

        let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;

        if encoder.as_label() == "utf-8" {
            let (source_slice, valid_length) = safe_byte_slice(&string, length.min(string.len()));
            writable_length = valid_length;
            target_bytes[offset..offset + writable_length].copy_from_slice(source_slice);
        } else {
            let decode_bytes = encoder.decode_from_string(string).or_throw(&ctx)?;
            writable_length = length.min(decode_bytes.len());
            target_bytes[offset..offset + writable_length]
                .copy_from_slice(&decode_bytes[..writable_length]);
        };
    }

    Ok(writable_length)
}

fn get_write_parameters(args: &Rest<Value<'_>>, len: usize) -> Result<(usize, usize, String)> {
    let encoding = "utf8".to_owned();

    match (args.0.first(), args.0.get(1), args.0.get(2)) {
        (Some(v1), _, _) if v1.as_string().is_some() => {
            Ok((0, len, v1.as_string().unwrap().to_string()?))
        },

        (Some(v1), Some(v2), _) if v2.as_string().is_some() => {
            let offset = v1.as_int().unwrap_or(0) as usize;
            Ok((offset, len - offset, v2.as_string().unwrap().to_string()?))
        },

        (Some(v1), Some(v2), Some(v3)) => {
            let offset = v1.as_int().unwrap_or(0) as usize;
            Ok((
                offset,
                v2.as_int()
                    .map_or(len - offset, |l| (l as usize).min(len - offset)),
                v3.as_string().map_or(encoding, |s| s.to_string().unwrap()),
            ))
        },

        (Some(v1), Some(v2), None) => {
            let offset = v1.as_int().unwrap_or(0) as usize;
            Ok((
                offset,
                v2.as_int()
                    .map_or(len - offset, |l| (l as usize).min(len - offset)),
                encoding,
            ))
        },

        (Some(v1), None, None) => {
            let offset = v1.as_int().unwrap_or(0) as usize;
            Ok((offset, len - offset, encoding))
        },

        _ => Ok((0, len, encoding)),
    }
}

fn safe_byte_slice(s: &str, end: usize) -> (&[u8], usize) {
    let bytes = s.as_bytes();

    if bytes.len() <= end {
        return (bytes, bytes.len());
    }

    let valid_end = s
        .char_indices()
        .map(|(i, _)| i)
        .filter(|&i| i <= end)
        .last()
        .unwrap_or(0);

    (&bytes[0..valid_end], valid_end)
}

fn write_int32_be<'js>(
    this: This<Object<'js>>,
    ctx: Ctx<'js>,
    value: i32,
    offset: Opt<i32>,
) -> Result<usize> {
    let source_slice = [
        (value >> 24) as u8,
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ];
    let offset = offset.0.unwrap_or_default();
    let buf_length = this.0.len().try_into().unwrap_or(i32::MAX);

    if offset < 0 || offset + source_slice.len() as i32 > buf_length {
        return Err(Exception::throw_range(
            &ctx,
            "The specified offset is out of range",
        ));
    }

    let offset = offset as usize;

    let target = ObjectBytes::from(&ctx, this.0.as_inner())?;

    let mut writable_length = 0;

    if let Some((array_buffer, _, _)) = target.get_array_buffer()? {
        let raw = array_buffer
            .as_raw()
            .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            .or_throw(&ctx)?;

        let target_bytes = unsafe { slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len) };

        writable_length = offset + source_slice.len();

        target_bytes[offset..writable_length].copy_from_slice(&source_slice);
    }

    Ok(writable_length)
}

fn set_prototype<'js>(ctx: &Ctx<'js>, constructor: Object<'js>) -> Result<()> {
    let _ = &constructor.set("alloc", Func::from(alloc))?;
    let _ = &constructor.set("allocUnsafe", Func::from(alloc_unsafe))?;
    let _ = &constructor.set("allocUnsafeSlow", Func::from(alloc_unsafe_slow))?;
    let _ = &constructor.set("byteLength", Func::from(byte_length))?;
    let _ = &constructor.set("concat", Func::from(concat))?;
    let _ = &constructor.set(PredefinedAtom::From, Func::from(from))?;
    let _ = &constructor.set("isBuffer", Func::from(is_buffer))?;
    let _ = &constructor.set("isEncoding", Func::from(is_encoding))?;

    let prototype: &Object = &constructor.get(PredefinedAtom::Prototype)?;
    prototype.set("copy", Func::from(copy))?;
    prototype.set("subarray", Func::from(subarray))?;
    prototype.set(PredefinedAtom::ToString, Func::from(to_string))?;
    prototype.set("write", Func::from(write))?;
    prototype.set("writeInt32BE", Func::from(write_int32_be))?;
    //not assessable from js
    prototype.prop(PredefinedAtom::Meta, stringify!(Buffer))?;

    ctx.globals().set(stringify!(Buffer), constructor)?;

    Ok(())
}

pub fn atob(ctx: Ctx<'_>, encoded_value: Coerced<String>) -> Result<rquickjs::String<'_>> {
    //fine to pass a slice here since we won't copy if not base64
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

    //init primordials
    let _ = BufferPrimordials::get(ctx)?;

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
        declare.declare("constants")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let buf: Constructor = globals.get(stringify!(Buffer))?;

        let constants = Object::new(ctx.clone())?;
        constants.set("MAX_LENGTH", u32::MAX)?; // For QuickJS
        constants.set("MAX_STRING_LENGTH", (1 << 30) - 1)?; // For QuickJS

        export_default(ctx, exports, |default| {
            default.set(stringify!(Buffer), buf)?;
            default.set("atob", Func::from(atob))?;
            default.set("btoa", Func::from(btoa))?;
            default.set("constants", constants)?;
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
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use super::*;

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

    #[tokio::test]
    async fn test_subarray() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let data = "hello world".to_string().into_bytes();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { Buffer } from 'buffer';

                        export async function test(data) {
                            let buffer = Buffer.from(data);
                            let sub = buffer.subarray(6, 11); // "world" part
                            return sub.toString();
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, (data,)).await;
                assert_eq!(result, "world");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_subarray_partial() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let data = "hello world".to_string().into_bytes();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { Buffer } from 'buffer';

                        export async function test(data) {
                            let buffer = Buffer.from(data);
                            let sub = buffer.subarray(0, 5); // "hello" part
                            return sub.toString();
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, (data,)).await;
                assert_eq!(result, "hello");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_subarray_out_of_bounds() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let data = "hello world".to_string().into_bytes();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { Buffer } from 'buffer';

                        export async function test(data) {
                            let buffer = Buffer.from(data);
                            let sub = buffer.subarray(6, 20); // "world" part but goes out of bounds
                            return sub.toString();
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, (data,)).await;
                assert_eq!(result, "world");
            })
        })
        .await;
    }
}
