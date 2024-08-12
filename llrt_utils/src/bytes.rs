// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use rquickjs::{ArrayBuffer, Coerced, Ctx, Exception, IntoJs, Object, Result, TypedArray, Value};

use crate::error_messages::ERROR_MSG_ARRAY_BUFFER_DETACHED;

#[derive(Clone)]
pub enum ObjectBytes<'js> {
    U8Array(TypedArray<'js, u8>),
    I8Array(TypedArray<'js, i8>),
    U16Array(TypedArray<'js, u16>),
    I16Array(TypedArray<'js, i16>),
    U32Array(TypedArray<'js, u32>),
    I32Array(TypedArray<'js, i32>),
    U64Array(TypedArray<'js, u64>),
    I64Array(TypedArray<'js, i64>),
    F32Array(TypedArray<'js, f32>),
    F64Array(TypedArray<'js, f64>),
    DataView(ArrayBuffer<'js>),
    Vec(Vec<u8>),
}

impl<'js> From<ObjectBytes<'js>> for Vec<u8> {
    fn from(value: ObjectBytes<'js>) -> Self {
        value.into_bytes()
    }
}

impl<'a, 'js> From<&'a ObjectBytes<'js>> for &'a [u8] {
    fn from(value: &'a ObjectBytes<'js>) -> Self {
        value.as_bytes()
    }
}

impl<'js> ObjectBytes<'js> {
    pub fn from(ctx: &Ctx<'js>, value: &Value<'js>) -> Result<Self> {
        Self::from_offset(ctx, value, 0, None)
    }

    pub fn from_offset(
        ctx: &Ctx<'js>,
        value: &Value<'js>,
        offset: usize,
        length: Option<usize>,
    ) -> Result<Self> {
        if value.is_undefined() {
            return Ok(ObjectBytes::Vec(vec![]));
        }
        if let Some(bytes) = get_string_bytes(value, offset, length)? {
            return Ok(ObjectBytes::Vec(bytes));
        }
        if let Some(bytes) = get_array_bytes(value, offset, length)? {
            return Ok(ObjectBytes::Vec(bytes));
        }

        if let Some(obj) = value.as_object() {
            if let Some(bytes) = Self::from_array_buffer(obj)? {
                return Ok(bytes);
            }
        }

        if let Some(bytes) = get_coerced_string_bytes(value, offset, length) {
            return Ok(ObjectBytes::Vec(bytes));
        }

        Err(Exception::throw_message(
        ctx,
        "value must be typed DataView, Buffer, ArrayBuffer, Uint8Array or interpretable as string",
    ))
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            ObjectBytes::U8Array(array) => array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED),
            ObjectBytes::I8Array(array) => array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED),
            ObjectBytes::U16Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::I16Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::U32Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::I32Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::U64Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::I64Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::F32Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::F64Array(array) => {
                array.as_bytes().expect(ERROR_MSG_ARRAY_BUFFER_DETACHED)
            },
            ObjectBytes::DataView(array_buffer) => array_buffer
                .as_bytes()
                .expect(ERROR_MSG_ARRAY_BUFFER_DETACHED),
            ObjectBytes::Vec(bytes) => bytes.as_ref(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        if let ObjectBytes::Vec(bytes) = self {
            return bytes;
        }
        self.as_bytes().to_vec()
    }

    pub fn from_array_buffer(obj: &Object<'js>) -> Result<Option<ObjectBytes<'js>>> {
        //most common
        if let Ok(typed_array) = TypedArray::<u8>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::U8Array(typed_array)));
        }
        //second most common
        if let Some(array_buffer) = ArrayBuffer::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::DataView(array_buffer)));
        }

        if let Ok(typed_array) = TypedArray::<i8>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::I8Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<u16>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::U16Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<i16>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::I16Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<u32>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::U32Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<i32>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::I32Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<u64>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::U64Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<i64>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::I64Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<f32>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::F32Array(typed_array)));
        }

        if let Ok(typed_array) = TypedArray::<f64>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::F64Array(typed_array)));
        }

        if let Ok(array_buffer) = obj.get::<_, ArrayBuffer>("buffer") {
            return Ok(Some(ObjectBytes::DataView(array_buffer)));
        }

        Ok(None)
    }

    pub fn get_array_buffer(&self) -> Result<Option<(ArrayBuffer<'js>, usize, usize)>> {
        let buffer = match self {
            ObjectBytes::DataView(array_buffer) => (array_buffer.clone(), array_buffer.len(), 0),
            ObjectBytes::U8Array(typed_array) => {
                let byte_length = typed_array.len();
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::I8Array(typed_array) => {
                let byte_length = typed_array.len();
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::U16Array(typed_array) => {
                let byte_length = typed_array.len() * 2;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::I16Array(typed_array) => {
                let byte_length = typed_array.len() * 2;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::U32Array(typed_array) => {
                let byte_length = typed_array.len() * 4;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::I32Array(typed_array) => {
                let byte_length = typed_array.len() * 4;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::U64Array(typed_array) => {
                let byte_length = typed_array.len() * 8;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::I64Array(typed_array) => {
                let byte_length = typed_array.len() * 8;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::F32Array(typed_array) => {
                let byte_length = typed_array.len() * 4;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            ObjectBytes::F64Array(typed_array) => {
                let byte_length = typed_array.len() * 8;
                (
                    typed_array.arraybuffer()?,
                    byte_length,
                    typed_array.get("byteOffset")?,
                )
            },
            _ => return Ok(None),
        };

        Ok(Some(buffer))
    }
}

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

pub fn get_array_bytes(
    value: &Value<'_>,
    offset: usize,
    length: Option<usize>,
) -> Result<Option<Vec<u8>>> {
    if value.is_array() {
        let array = value.as_array().unwrap();
        let (start, end) = get_start_end_indexes(array.len(), length, offset);
        let size = end - start;
        let mut bytes: Vec<u8> = Vec::with_capacity(size);

        for val in array.iter::<u8>().skip(start).take(size) {
            let val: u8 = val?;
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

fn bytes_from_js_string(string: String, offset: usize, length: Option<usize>) -> Vec<u8> {
    let (start, end) = get_start_end_indexes(string.len(), length, offset);
    string.as_bytes()[start..end].to_vec()
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

pub fn bytes_to_typed_array<'js>(ctx: Ctx<'js>, bytes: &[u8]) -> Result<Value<'js>> {
    TypedArray::<u8>::new(ctx.clone(), bytes).into_js(&ctx)
}
