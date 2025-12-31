// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{mem::MaybeUninit, slice};

use llrt_encoding::{bytes_from_b64, bytes_to_b64_string, Encoder};
use llrt_utils::{
    bytes::{
        get_array_bytes, get_coerced_string_bytes, get_start_end_indexes, get_string_bytes,
        ObjectBytes,
    },
    error_messages::{ERROR_MSG_ARRAY_BUFFER_DETACHED, ERROR_MSG_NOT_ARRAY_BUFFER},
    primordials::Primordial,
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, Opt},
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

    // This conversion is safe because MaybeUninit has the same memory layout as u8, meaning the underlying bytes are identical.
    // Since Vec<MaybeUninit> and Vec share the same memory representation, a simple reinterpretation of the pointer is valid.
    // Additionally, Vec::from_raw_parts correctly reconstructs the vector using the original length and capacity, ensuring that memory ownership remains consistent.
    // The call to std::mem::forget(vec) prevents the original Vec<MaybeUninit> from being dropped, avoiding double frees or memory corruption.
    // However, this conversion is only safe if all elements of MaybeUninit are properly initialized.
    // If any uninitialized values exist, reading them as u8 would lead to undefined behavior.
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

    // WARN: This is currently bugged for encodings that are not utf8 since we first
    // convert to utf8 and then decode using the encoding.
    // See https://github.com/quickjs-ng/quickjs/issues/992
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
    let view = TypedArray::<u8>::from_object(this.0.clone())?;

    let array_buffer = view.arraybuffer()?;
    let view_offset = this.0.get::<_, isize>("byteOffset")?;
    let view_length = this.0.get::<_, isize>("byteLength")?;

    let start_index = start.map_or(0, |s| {
        if s < 0 {
            (view_length + s).max(0)
        } else {
            s.min(view_length)
        }
    });

    let end_index = end.map_or(view_length, |e| {
        if e < 0 {
            (view_length + e).max(0)
        } else {
            e.min(view_length)
        }
    });

    let length = (end_index - start_index).max(0) as usize;
    let new_offset = (view_offset + start_index).max(0) as usize;

    Buffer::from_array_buffer_offset_length(&ctx, array_buffer, new_offset, length)
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
    let mut offset = 0;
    let mut length = len;
    let mut encoding = "utf8".to_owned();

    if let Some(v1) = args.0.first() {
        if let Some(s) = v1.as_string() {
            return Ok((0, len, s.to_string()?));
        }
        offset = v1.as_int().unwrap_or(0) as usize;
    }

    if let Some(v2) = args.0.get(1) {
        if let Some(s) = v2.as_string() {
            return Ok((offset, len - offset, s.to_string()?));
        }
        length = v2
            .as_int()
            .map_or(len - offset, |l| (l as usize).min(len - offset));
    }

    if let Some(v3) = args.0.get(2) {
        if let Some(s) = v3.as_string() {
            encoding = s.to_string()?;
        }
    }

    Ok((offset, length, encoding))
}

fn safe_byte_slice(s: &str, end: usize) -> (&[u8], usize) {
    let bytes = s.as_bytes();

    if bytes.len() <= end {
        return (bytes, bytes.len());
    }

    let valid_end = s
        .char_indices()
        .map(|(i, _)| i)
        .rfind(|&i| i <= end)
        .unwrap_or(0);

    (&bytes[0..valid_end], valid_end)
}

#[derive(Clone, Copy)]
pub enum Endian {
    Little,
    Big,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NumberKind {
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Float32,
    Float64,
    BigInt,
}

impl NumberKind {
    pub fn bits(&self) -> u8 {
        match self {
            NumberKind::Int8 => 8,
            NumberKind::UInt8 => 8,
            NumberKind::Int16 => 16,
            NumberKind::UInt16 => 16,
            NumberKind::Int32 => 32,
            NumberKind::UInt32 => 32,
            NumberKind::Float32 => 32,
            NumberKind::Float64 => 64,
            NumberKind::BigInt => 64,
        }
    }

    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            NumberKind::Int8 | NumberKind::Int16 | NumberKind::Int32
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn write_buf<'js>(
    this: &This<Object<'js>>,
    ctx: &Ctx<'js>,
    value: &Value<'js>,
    offset: &Opt<usize>,
    endian: Endian,
    kind: NumberKind,
) -> Result<usize> {
    let offset = offset.0.unwrap_or_default();

    // Extract and convert value
    let (byte_count, bytes) = match kind {
        NumberKind::BigInt => {
            let Some(bigint) = value.as_big_int() else {
                return Err(Exception::throw_type(ctx, "Expected BigInt"));
            };
            let (byte_count, val) = (8, bigint.clone().to_i64().or_throw(ctx)? as u64);
            (byte_count, endian_bytes(val, endian))
        },
        NumberKind::Float32 => {
            let Some(float_val) = value.as_float() else {
                return Err(Exception::throw_type(ctx, "Expected number"));
            };
            match endian {
                Endian::Big => (4, (float_val as f32).to_bits().to_be_bytes().to_vec()),
                Endian::Little => (4, (float_val as f32).to_bits().to_le_bytes().to_vec()),
            }
        },
        NumberKind::Float64 => {
            let Some(float_val) = value.as_float() else {
                return Err(Exception::throw_type(ctx, "Expected number"));
            };
            match endian {
                Endian::Big => (8, float_val.to_bits().to_be_bytes().to_vec()),
                Endian::Little => (8, float_val.to_bits().to_le_bytes().to_vec()),
            }
        },
        NumberKind::Int8
        | NumberKind::UInt8
        | NumberKind::Int16
        | NumberKind::UInt16
        | NumberKind::Int32
        | NumberKind::UInt32 => {
            let Some(int_val) = value.as_number() else {
                return Err(Exception::throw_type(ctx, "Expected number"));
            };
            let int_val = int_val as i64;
            let bit_mask = (1i64 << kind.bits()) - 1;
            let max_val = if kind.is_signed() {
                (1i64 << (kind.bits() - 1)) - 1
            } else {
                bit_mask
            };
            let min_val = if kind.is_signed() { -max_val - 1 } else { 0 };

            if int_val < min_val || int_val > max_val {
                return Err(Exception::throw_range(ctx, "Value out of range"));
            }

            let masked = int_val & bit_mask;
            (
                (kind.bits() / 8) as usize,
                shifted_bytes(masked as u64, kind.bits(), endian),
            )
        },
    };

    if offset >= this.0.len() || offset + byte_count > this.0.len() {
        return Err(Exception::throw_range(
            ctx,
            "The specified offset is out of range",
        ));
    }

    let target = ObjectBytes::from(ctx, this.0.as_inner())?;
    let mut writable_length = 0;

    if let Some((array_buffer, _, _)) = target.get_array_buffer()? {
        let raw = array_buffer
            .as_raw()
            .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            .or_throw(ctx)?;

        let target_bytes = unsafe { slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len) };

        writable_length = offset + bytes.len();
        target_bytes[offset..writable_length].copy_from_slice(&bytes);
    }

    Ok(writable_length)
}

fn read_buf<'js>(
    this: &This<Object<'js>>,
    ctx: &Ctx<'js>,
    offset: &Opt<usize>,
    endian: Endian,
    kind: NumberKind,
) -> Result<Value<'js>> {
    // Retrieve the array buffer
    let target = ObjectBytes::from(&ctx, this.0.as_inner())?;
    let Some((array_buffer, _, _)) = target.get_array_buffer()? else {
        return Err(Exception::throw_message(ctx, ERROR_MSG_NOT_ARRAY_BUFFER));
    };
    let raw = array_buffer
        .as_raw()
        .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
        .or_throw(ctx)?;
    let target_bytes = unsafe { slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len) };

    // Enforce the bounds
    let start = offset.0.unwrap_or_default();
    let end = start + (kind.bits() / 8) as usize;
    if end > raw.len {
        return Err(Exception::throw_range(
            ctx,
            "The value of \"offset\" is out of range",
        ));
    }

    let bytes = &target_bytes[start..end];

    let value = match kind {
        NumberKind::BigInt => {
            let value = match endian {
                Endian::Big => i64::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => i64::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_big_int(ctx.clone(), value)
        },
        NumberKind::Float32 => {
            let value = match endian {
                Endian::Big => f32::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => f32::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_float(ctx.clone(), value as f64)
        },
        NumberKind::Float64 => {
            let value = match endian {
                Endian::Big => f64::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => f64::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_float(ctx.clone(), value)
        },
        NumberKind::Int8 => {
            let value = match endian {
                Endian::Big => i8::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => i8::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_int(ctx.clone(), value as i32)
        },
        NumberKind::UInt8 => {
            let value = match endian {
                Endian::Big => u8::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => u8::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_int(ctx.clone(), value as i32)
        },
        NumberKind::Int16 => {
            let value = match endian {
                Endian::Big => i16::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => i16::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_int(ctx.clone(), value as i32)
        },
        NumberKind::UInt16 => {
            let value = match endian {
                Endian::Big => u16::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => u16::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_int(ctx.clone(), value as i32)
        },
        NumberKind::Int32 => {
            let value = match endian {
                Endian::Big => i32::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => i32::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_int(ctx.clone(), value)
        },
        NumberKind::UInt32 => {
            let value = match endian {
                Endian::Big => u32::from_be_bytes(bytes.try_into().unwrap()),
                Endian::Little => u32::from_le_bytes(bytes.try_into().unwrap()),
            };
            Value::new_float(ctx.clone(), value as f64)
        },
    };
    Ok(value)
}

// Pure mathematical byte generation
fn endian_bytes(mut val: u64, endian: Endian) -> Vec<u8> {
    let mut bytes = vec![0u8; 8];

    #[allow(clippy::needless_range_loop)]
    for i in 0..8 {
        bytes[i] = match endian {
            Endian::Big => (val >> (56 - i * 8)) as u8,
            Endian::Little => (val >> (i * 8)) as u8,
        };
        // Clear processed bits
        match endian {
            Endian::Big => val &= !(0xFF << ((7 - i) * 8)),
            Endian::Little => val &= !(0xFF << (i * 8)),
        }
    }
    bytes
}

fn shifted_bytes(mut val: u64, bits: u8, endian: Endian) -> Vec<u8> {
    let byte_count = (bits / 8) as usize;
    let mut bytes = vec![0u8; byte_count];

    #[allow(clippy::needless_range_loop)]
    for i in 0..byte_count {
        let shift = match endian {
            Endian::Big => (byte_count - 1 - i) * 8,
            Endian::Little => i * 8,
        };
        bytes[i] = (val >> shift) as u8;
        val &= !(0xFF << shift); // Clear processed bits
    }
    bytes
}

pub(crate) fn set_prototype<'js>(ctx: &Ctx<'js>, constructor: Object<'js>) -> Result<()> {
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
    prototype.set(
        "writeBigInt64BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::BigInt)),
    )?;
    prototype.set(
        "writeBigUint64BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::BigInt)),
    )?;
    prototype.set(
        "writeBigInt64LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::BigInt)),
    )?;
    prototype.set(
        "writeBigUint64LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::BigInt)),
    )?;
    prototype.set(
        "writeDoubleBE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::Float64)),
    )?;
    prototype.set(
        "writeDoubleLE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::Float64)),
    )?;
    prototype.set(
        "writeFloatBE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::Float32)),
    )?;
    prototype.set(
        "writeFloatLE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::Float32)),
    )?;
    prototype.set(
        "writeInt8",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::Int8)),
    )?;
    prototype.set(
        "writeInt16BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::Int16)),
    )?;
    prototype.set(
        "writeInt16LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::Int16)),
    )?;
    prototype.set(
        "writeInt32BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::Int32)),
    )?;
    prototype.set(
        "writeInt32LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::Int32)),
    )?;
    prototype.set(
        "writeUInt8",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::UInt8)),
    )?;
    prototype.set(
        "writeUint8",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::UInt8)),
    )?;
    prototype.set(
        "writeUInt16BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::UInt16)),
    )?;
    prototype.set(
        "writeUint16BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::UInt16)),
    )?;
    prototype.set(
        "writeUInt16LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::UInt16)),
    )?;
    prototype.set(
        "writeUint16LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::UInt16)),
    )?;
    prototype.set(
        "writeUInt32BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::UInt32)),
    )?;
    prototype.set(
        "writeUint32BE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Big, NumberKind::UInt32)),
    )?;
    prototype.set(
        "writeUInt32LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::UInt32)),
    )?;
    prototype.set(
        "writeUint32LE",
        Func::from(|t, c, v, o| write_buf(&t, &c, &v, &o, Endian::Little, NumberKind::UInt32)),
    )?;
    prototype.set(
        "readBigInt64BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::BigInt)),
    )?;
    prototype.set(
        "readBigUint64BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::BigInt)),
    )?;
    prototype.set(
        "readBigInt64LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::BigInt)),
    )?;
    prototype.set(
        "readBigUint64LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::BigInt)),
    )?;
    prototype.set(
        "readDoubleBE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::Float64)),
    )?;
    prototype.set(
        "readDoubleLE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::Float64)),
    )?;
    prototype.set(
        "readFloatBE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::Float32)),
    )?;
    prototype.set(
        "readFloatLE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::Float32)),
    )?;
    prototype.set(
        "readInt8",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::Int8)),
    )?;
    prototype.set(
        "readInt16BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::Int16)),
    )?;
    prototype.set(
        "readInt16LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::Int16)),
    )?;
    prototype.set(
        "readInt32BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::Int32)),
    )?;
    prototype.set(
        "readInt32LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::Int32)),
    )?;
    prototype.set(
        "readUInt8",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::UInt8)),
    )?;
    prototype.set(
        "readUint8",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::UInt8)),
    )?;
    prototype.set(
        "readUInt16BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::UInt16)),
    )?;
    prototype.set(
        "readUint16BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::UInt16)),
    )?;
    prototype.set(
        "readUInt16LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::UInt16)),
    )?;
    prototype.set(
        "readUint16LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::UInt16)),
    )?;
    prototype.set(
        "readUInt32BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::UInt32)),
    )?;
    prototype.set(
        "readUint32BE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Big, NumberKind::UInt32)),
    )?;
    prototype.set(
        "readUInt32LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::UInt32)),
    )?;
    prototype.set(
        "readUint32LE",
        Func::from(|t, c, o| read_buf(&t, &c, &o, Endian::Little, NumberKind::UInt32)),
    )?;
    //not assessable from js
    prototype.prop(PredefinedAtom::Meta, stringify!(Buffer))?;

    ctx.globals().set(stringify!(Buffer), constructor)?;

    Ok(())
}

pub fn atob(ctx: Ctx<'_>, encoded_value: Coerced<String>) -> Result<rquickjs::String<'_>> {
    let vec = bytes_from_b64(encoded_value.as_bytes()).or_throw(&ctx)?;
    // Convert bytes to Latin-1 string where each byte becomes a character with that code point.
    // This matches the WHATWG spec: atob returns a "binary string" where each character's
    // code point is 0-255, directly representing one byte of data.
    let str: String = vec.iter().map(|&b| b as char).collect();
    rquickjs::String::from_str(ctx, &str)
}

pub fn btoa(ctx: Ctx<'_>, value: Coerced<String>) -> Result<String> {
    // Per WHATWG spec, btoa() treats input as a "binary string" where each character
    // must have a code point 0-255. Characters > 255 cause InvalidCharacterError.
    let s: &str = value.as_str();

    // Fast path: ASCII is a 1:1 mapping to bytes 0-127 (SIMD optimized)
    if s.is_ascii() {
        return Ok(bytes_to_b64_string(s.as_bytes()));
    }

    // Slow path: Check for Latin-1 (0-255)
    let bytes: Vec<u8> = s
        .chars()
        .map(|c| {
            let code_point = c as u32;
            if code_point > 255 {
                Err(Exception::throw_message(
                    &ctx,
                    "Invalid character: btoa() argument contains character with code point > 255",
                ))
            } else {
                Ok(code_point as u8)
            }
        })
        .collect::<Result<Vec<u8>>>()?;
    Ok(bytes_to_b64_string(&bytes))
}

#[cfg(test)]
mod tests {
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use crate::BufferModule;

    #[tokio::test]
    async fn test_atob() {
        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
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
    async fn test_atob_high_bytes() {
        // Test that atob correctly decodes bytes 128-255 as Latin-1 characters
        // (each byte becomes a character with that code point)
        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { atob } from 'buffer';

                        export async function test() {
                            // Test individual high-byte values
                            // gA== decodes to byte 0x80 (128)
                            // /w== decodes to byte 0xFF (255)
                            const test128 = atob("gA==");
                            const test255 = atob("/w==");

                            // Each decoded byte should become a character
                            // with that exact code point
                            if (test128.charCodeAt(0) !== 128) {
                                return `byte 128 failed: got ${test128.charCodeAt(0)}`;
                            }
                            if (test255.charCodeAt(0) !== 255) {
                                return `byte 255 failed: got ${test255.charCodeAt(0)}`;
                            }

                            // Test all bytes 128-255 to ensure none are corrupted
                            // Create base64 for bytes 128-255 and verify roundtrip
                            const highBytes = new Uint8Array(128);
                            for (let i = 0; i < 128; i++) {
                                highBytes[i] = 128 + i;
                            }
                            const base64 = Buffer.from(highBytes).toString("base64");
                            const decoded = atob(base64);

                            for (let i = 0; i < 128; i++) {
                                const expected = 128 + i;
                                const actual = decoded.charCodeAt(i);
                                if (actual !== expected) {
                                    return `byte ${expected} failed: got ${actual}`;
                                }
                            }

                            return "ok";
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "ok");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_btoa() {
        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
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
    async fn test_btoa_high_bytes() {
        // Test that btoa correctly encodes Latin-1 characters (code points 128-255)
        // as single bytes per WHATWG spec (not UTF-8 encoded)
        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<BufferModule>(ctx.clone(), "buffer")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { btoa, atob } from 'buffer';

                        export async function test() {
                            // Test byte 255 (0xFF): should encode as single byte
                            // btoa(String.fromCharCode(255)) should give "/w==" not "w78="
                            const char255 = String.fromCharCode(255);
                            const encoded255 = btoa(char255);
                            if (encoded255 !== "/w==") {
                                return `byte 255 encoding failed: got ${encoded255}, expected /w==`;
                            }

                            // Test byte 128 (0x80): should encode as single byte
                            const char128 = String.fromCharCode(128);
                            const encoded128 = btoa(char128);
                            if (encoded128 !== "gA==") {
                                return `byte 128 encoding failed: got ${encoded128}, expected gA==`;
                            }

                            // Test roundtrip for all bytes 0-255
                            for (let i = 0; i <= 255; i++) {
                                const char = String.fromCharCode(i);
                                const encoded = btoa(char);
                                const decoded = atob(encoded);
                                if (decoded.charCodeAt(0) !== i || decoded.length !== 1) {
                                    return `roundtrip failed for byte ${i}`;
                                }
                            }

                            // Test that characters > 255 throw
                            try {
                                btoa("€"); // U+20AC
                                return "btoa should have thrown for euro sign";
                            } catch (e) {
                                // Expected
                            }

                            return "ok";
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "ok");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_subarray() {
        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
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
                crate::init(&ctx).unwrap();
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
                crate::init(&ctx).unwrap();
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

    #[tokio::test]
    async fn test_read_int_32_be() {
        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
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
                            const buf = Buffer.from([1, 2, 3, 4, 0, 0, 0, 0]);
                            return buf.readInt32BE();
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<i32, _>(&ctx, &module, (data,)).await;
                assert_eq!(result, 0x01020304);
            })
        })
        .await;
    }
}
