use rquickjs::{ArrayBuffer, Ctx, Error, FromJs, IntoJs, Result, TypedArray, Value};

pub struct ArrayBufferView<'js> {
    buffer: ArrayBuffer<'js>,
}

impl<'js> IntoJs<'js> for ArrayBufferView<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        self.buffer.into_js(ctx)
    }
}

impl<'js> FromJs<'js> for ArrayBufferView<'js> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();

        if let Ok(array_buffer) = ArrayBuffer::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: array_buffer,
            });
        }

        if let Ok(int8_array) = TypedArray::<i8>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: int8_array.arraybuffer()?,
            });
        }

        if let Ok(uint8_array) = TypedArray::<u8>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: uint8_array.arraybuffer()?,
            });
        }

        if let Ok(int16_array) = TypedArray::<i16>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: int16_array.arraybuffer()?,
            });
        }

        if let Ok(uint16_array) = TypedArray::<u16>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: uint16_array.arraybuffer()?,
            });
        }

        if let Ok(int32_array) = TypedArray::<i32>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: int32_array.arraybuffer()?,
            });
        }

        if let Ok(uint32_array) = TypedArray::<u32>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: uint32_array.arraybuffer()?,
            });
        }

        if let Ok(float32_array) = TypedArray::<f32>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: float32_array.arraybuffer()?,
            });
        }

        if let Ok(float64_array) = TypedArray::<f64>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: float64_array.arraybuffer()?,
            });
        }

        if let Ok(bigint64_array) = TypedArray::<i64>::from_js(ctx, value.clone()) {
            return Ok(ArrayBufferView {
                buffer: bigint64_array.arraybuffer()?,
            });
        }

        if let Ok(biguint64_array) = TypedArray::<u64>::from_js(ctx, value) {
            return Ok(ArrayBufferView {
                buffer: biguint64_array.arraybuffer()?,
            });
        }

        Err(Error::new_from_js(ty_name, "ArrayBufferView"))
    }
}

impl<'js> ArrayBufferView<'js> {
    pub fn new(ctx: Ctx<'js>, size: usize) -> Result<Self> {
        Ok(ArrayBufferView {
            buffer: ArrayBuffer::new(ctx, vec![0u8; size])?,
        })
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Mutable buffer for the view.
    ///
    /// # Safety
    /// This is only safe if you have a lock on the runtime.
    /// Do not pass it directly to other threads.
    pub unsafe fn buffer_mut(&self) -> Option<&mut [u8]> {
        let raw = self.buffer.as_raw()?;
        Some(std::slice::from_raw_parts_mut(raw.ptr.as_ptr(), raw.len))
    }
}
