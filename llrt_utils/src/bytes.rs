// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{ArrayBuffer, Coerced, Ctx, Exception, IntoJs, Object, Result, TypedArray, Value};

use super::result::ResultExt;

pub fn get_start_end_indexes(
    source_len: usize,
    target_len: Option<usize>,
    offset: usize,
) -> (usize, usize) {
    if offset > source_len {
        return (0, 0);
    }

    let target_len = target_len.unwrap_or(source_len - offset);

    if offset + target_len > source_len {
        return (offset, source_len);
    }

    (offset, target_len + offset)
}

pub fn get_bytes_offset_length<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    offset: usize,
    length: Option<usize>,
) -> Result<Vec<u8>> {
    if value.is_undefined() {
        return Ok(vec![]);
    }
    if let Some(bytes) = get_string_bytes(&value, offset, length)? {
        return Ok(bytes);
    }
    if let Some(bytes) = get_array_bytes(ctx, &value, offset, length)? {
        return Ok(bytes);
    }

    if let Some(obj) = value.as_object() {
        if let Some((array_buffer, source_length, source_offset)) = obj_to_array_buffer(obj)? {
            let (start, end) = get_start_end_indexes(source_length, length, offset);
            let bytes: &[u8] = array_buffer.as_ref();
            return Ok(bytes[(start + source_offset)..(end + source_offset)].to_vec());
        }
    }

    if let Some(bytes) = get_coerced_string_bytes(&value, offset, length) {
        return Ok(bytes);
    }

    Err(Exception::throw_message(
        ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or interpretable as string",
    ))
}

pub fn get_array_bytes<'js>(
    ctx: &Ctx<'js>,
    value: &Value<'js>,
    offset: usize,
    length: Option<usize>,
) -> Result<Option<Vec<u8>>> {
    if value.is_array() {
        let array = value.as_array().unwrap();
        let (start, end) = get_start_end_indexes(array.len(), length, offset);
        let size = end - start;
        let mut bytes: Vec<u8> = Vec::with_capacity(size);

        for val in array.iter::<u8>().skip(start).take(size) {
            let val: u8 = val.or_throw_msg(ctx, "array value is not u8")?;
            bytes.push(val);
        }

        return Ok(Some(bytes));
    }
    Ok(None)
}

pub fn get_coerced_string_bytes(
    value: &Value<'_>,
    offset: usize,
    length: Option<usize>,
) -> Option<Vec<u8>> {
    if let Ok(val) = value.get::<Coerced<String>>() {
        let string = val.to_string();
        return Some(bytes_from_js_string(string, offset, length));
    };
    None
}

#[inline]
pub fn get_string_bytes(
    value: &Value<'_>,
    offset: usize,
    length: Option<usize>,
) -> Result<Option<Vec<u8>>> {
    if let Some(val) = value.as_string() {
        let string = val.to_string()?;
        return Ok(Some(bytes_from_js_string(string, offset, length)));
    }
    Ok(None)
}

fn bytes_from_js_string(string: String, offset: usize, length: Option<usize>) -> Vec<u8> {
    let (start, end) = get_start_end_indexes(string.len(), length, offset);
    string.as_bytes()[start..end].to_vec()
}

pub fn obj_to_array_buffer<'js>(
    obj: &Object<'js>,
) -> Result<Option<(ArrayBuffer<'js>, usize, usize)>> {
    //most common
    if let Ok(typed_array) = TypedArray::<u8>::from_object(obj.clone()) {
        let byte_length = typed_array.len();
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }
    //second most common
    if let Some(array_buffer) = ArrayBuffer::from_object(obj.clone()) {
        let byte_length = array_buffer.len();
        return Ok(Some((array_buffer, byte_length, 0)));
    }

    if let Ok(typed_array) = TypedArray::<i8>::from_object(obj.clone()) {
        let byte_length = typed_array.len();
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<u16>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 2;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<i16>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 2;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<u32>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 4;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<i32>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 4;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<u64>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 8;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<i64>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 8;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<f32>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 4;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(typed_array) = TypedArray::<f64>::from_object(obj.clone()) {
        let byte_length = typed_array.len() * 8;
        let offset: usize = typed_array.get("byteOffset")?;
        return Ok(Some((typed_array.arraybuffer()?, byte_length, offset)));
    }

    if let Ok(array_buffer) = obj.get::<_, ArrayBuffer>("buffer") {
        let length = array_buffer.len();
        return Ok(Some((array_buffer, length, 0)));
    }

    Ok(None)
}

pub fn get_array_buffer_bytes(
    array_buffer: ArrayBuffer<'_>,
    start: usize,
    end_end: usize,
) -> Vec<u8> {
    let bytes: &[u8] = array_buffer.as_ref();
    bytes[start..end_end].to_vec()
}

pub fn get_bytes<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Vec<u8>> {
    get_bytes_offset_length(ctx, value, 0, None)
}

pub fn bytes_to_typed_array<'js>(ctx: Ctx<'js>, bytes: &[u8]) -> Result<Value<'js>> {
    TypedArray::<u8>::new(ctx.clone(), bytes).into_js(&ctx)
}
