// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::ptr::NonNull;

use rquickjs::{ArrayBuffer, Ctx, Error, FromJs, IntoJs, Object, Result, TypedArray, Value};

#[cfg(feature = "buffer")]
use crate::buffer::Buffer;

pub struct ArrayBufferView<'js> {
    value: Value<'js>,
    buffer: Option<RawArrayBuffer>,
}

struct RawArrayBuffer {
    len: usize,
    ptr: NonNull<u8>,
}

impl RawArrayBuffer {
    pub fn new(len: usize, ptr: NonNull<u8>) -> Self {
        Self { len, ptr }
    }
}

impl<'js> IntoJs<'js> for ArrayBufferView<'js> {
    fn into_js(self, _ctx: &Ctx<'js>) -> Result<Value<'js>> {
        Ok(self.value)
    }
}

impl<'js> FromJs<'js> for ArrayBufferView<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = Object::from_value(value.clone())
            .map_err(|_| Error::new_from_js(ty_name, "ArrayBufferView"))?;

        if let Some(array_buffer) = ArrayBuffer::from_object(obj.clone()) {
            let buffer = array_buffer
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<i8>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<u8>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<i16>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<u16>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<i32>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<u32>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<f32>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<f64>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<i64>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(typed_array) = TypedArray::<u64>::from_object(obj.clone()) {
            let buffer = typed_array
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        if let Ok(array_buffer) = obj.get::<_, ArrayBuffer>("buffer") {
            let buffer = array_buffer
                .as_raw()
                .map(|raw| RawArrayBuffer::new(raw.len, raw.ptr));
            return Ok(ArrayBufferView { value, buffer });
        }

        Err(Error::new_from_js(ty_name, "ArrayBufferView"))
    }
}

impl<'js> ArrayBufferView<'js> {
    #[cfg(feature = "buffer")]
    pub fn from_buffer(ctx: &Ctx<'js>, buffer: Buffer) -> Result<Self> {
        let value = buffer.into_js(ctx)?;
        Self::from_js(ctx, value)
    }

    pub fn len(&self) -> usize {
        self.buffer.as_ref().map(|b| b.len).unwrap_or(0)
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.buffer
            .as_ref()
            .map(|b| unsafe { std::slice::from_raw_parts(b.ptr.as_ptr(), b.len) })
    }

    /// Mutable buffer for the view.
    ///
    /// # Safety
    /// This is only safe if you have a lock on the runtime.
    /// Do not pass it directly to other threads.
    pub fn as_bytes_mut(&self) -> Option<&mut [u8]> {
        self.buffer
            .as_ref()
            .map(|b| unsafe { std::slice::from_raw_parts_mut(b.ptr.as_ptr(), b.len) })
    }
}
