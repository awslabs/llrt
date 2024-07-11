// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{ArrayBuffer, Ctx, Error, FromJs, IntoJs, Object, Result, TypedArray, Value};

#[cfg(feature = "buffer")]
use crate::buffer::Buffer;

pub struct ArrayBufferView<'js> {
    value: Value<'js>,
    buffer: ArrayBuffer<'js>,
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
            return Ok(ArrayBufferView {
                value,
                buffer: array_buffer,
            });
        }

        if let Ok(int8_array) = TypedArray::<i8>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: int8_array.arraybuffer()?,
            });
        }

        if let Ok(uint8_array) = TypedArray::<u8>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: uint8_array.arraybuffer()?,
            });
        }

        if let Ok(int16_array) = TypedArray::<i16>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: int16_array.arraybuffer()?,
            });
        }

        if let Ok(uint16_array) = TypedArray::<u16>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: uint16_array.arraybuffer()?,
            });
        }

        if let Ok(int32_array) = TypedArray::<i32>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: int32_array.arraybuffer()?,
            });
        }

        if let Ok(uint32_array) = TypedArray::<u32>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: uint32_array.arraybuffer()?,
            });
        }

        if let Ok(float32_array) = TypedArray::<f32>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: float32_array.arraybuffer()?,
            });
        }

        if let Ok(float64_array) = TypedArray::<f64>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: float64_array.arraybuffer()?,
            });
        }

        if let Ok(bigint64_array) = TypedArray::<i64>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: bigint64_array.arraybuffer()?,
            });
        }

        if let Ok(biguint64_array) = TypedArray::<u64>::from_object(obj.clone()) {
            return Ok(ArrayBufferView {
                value,
                buffer: biguint64_array.arraybuffer()?,
            });
        }

        if let Ok(array_buffer) = obj.get::<_, ArrayBuffer>("buffer") {
            return Ok(ArrayBufferView {
                value,
                buffer: array_buffer,
            });
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
        self.buffer.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.buffer.as_bytes()
    }

    /// Mutable buffer for the view.
    ///
    /// # Safety
    /// This is only safe if you have a lock on the runtime.
    /// Do not pass it directly to other threads.
    pub unsafe fn as_bytes_mut(&self) -> Option<&mut [u8]> {
        let raw = self.buffer.as_raw()?;
        Some(std::slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len))
    }
}
