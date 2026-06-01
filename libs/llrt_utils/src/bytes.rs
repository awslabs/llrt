// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{rc::Rc, slice};

use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    function::Constructor,
    ArrayBuffer, Coerced, Ctx, Error, Exception, FromJs, IntoJs, JsLifetime, Object, Result,
    TypedArray, Value,
};

/// Convert a JS string to a `String`, replacing lone UTF-16 surrogates
/// with U+FFFD per WHATWG USVString. Use when ill-formed strings must
/// not fail.
//
// SAFETY (module-wide): QuickJS only emits valid WTF-8, so any run
// without 0xED is valid strict UTF-8.
pub fn get_lossy_string(string_value: Value) -> Result<String> {
    let js_str = string_value.into_string().ok_or_else(|| Error::FromJs {
        from: "Value",
        to: "JSString",
        message: Some("Value is not a string".into()),
    })?;
    let cstr = js_str.to_cstring()?;
    let bytes = unsafe { slice::from_raw_parts(cstr.as_ptr() as *const u8, cstr.len()) };

    let first = match memchr::memchr(0xED, bytes) {
        None => return Ok(unsafe { String::from_utf8_unchecked(bytes.to_vec()) }),
        Some(idx) => idx,
    };
    let mut result = String::with_capacity(bytes.len());
    result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[..first]) });
    qjs_substitute_into(&bytes[first..], &mut result);
    Ok(result)
}

fn qjs_substitute_into(bytes: &[u8], result: &mut String) {
    let mut start = 0;
    while start < bytes.len() {
        let next_ed = match memchr::memchr(0xED, &bytes[start..]) {
            None => {
                result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..]) });
                return;
            },
            Some(rel) => start + rel,
        };
        if next_ed > start {
            result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[start..next_ed]) });
        }
        if next_ed + 3 > bytes.len() {
            replace_invalid_utf8_and_utf16_into(&bytes[next_ed..], result);
            return;
        }
        let b1 = bytes[next_ed + 1];
        let b2 = bytes[next_ed + 2];
        if (b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 {
            replace_invalid_utf8_and_utf16_into(&bytes[next_ed..], result);
            return;
        }
        if (b1 & 0xE0) == 0xA0 {
            result.push('\u{FFFD}');
        } else {
            result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[next_ed..next_ed + 3]) });
        }
        start = next_ed + 3;
    }
}

#[doc(hidden)]
pub fn replace_invalid_utf8_and_utf16(bytes: &[u8]) -> String {
    let err = match simdutf8::compat::from_utf8(bytes) {
        Ok(s) => return s.to_owned(),
        Err(e) => e,
    };
    let valid_up_to = err.valid_up_to();
    let mut result = String::with_capacity(bytes.len());
    result.push_str(unsafe { std::str::from_utf8_unchecked(&bytes[..valid_up_to]) });
    replace_invalid_utf8_and_utf16_into(&bytes[valid_up_to..], &mut result);
    result
}

fn replace_invalid_utf8_and_utf16_into(bytes: &[u8], result: &mut String) {
    let mut i = 0;

    while i < bytes.len() {
        let current = bytes[i];
        match current {
            0x00..=0x7F => {
                result.push(current as char);
                i += 1;
            },
            0xC0..=0xDF if i + 1 < bytes.len() => {
                let next = bytes[i + 1];
                if (next & 0xC0) == 0x80 {
                    let code_point = ((current as u32 & 0x1F) << 6) | (next as u32 & 0x3F);
                    result.push(char::from_u32(code_point).unwrap_or('\u{FFFD}'));
                    i += 2;
                } else {
                    result.push('\u{FFFD}');
                    i += 1;
                }
            },
            0xE0..=0xEF if i + 2 < bytes.len() => {
                let next1 = bytes[i + 1];
                let next2 = bytes[i + 2];
                if (next1 & 0xC0) == 0x80 && (next2 & 0xC0) == 0x80 {
                    let code_point = ((current as u32 & 0x0F) << 12)
                        | ((next1 as u32 & 0x3F) << 6)
                        | (next2 as u32 & 0x3F);
                    result.push(char::from_u32(code_point).unwrap_or('\u{FFFD}'));
                    i += 3;
                } else {
                    result.push('\u{FFFD}');
                    i += 1;
                }
            },
            0xF0..=0xF7 if i + 3 < bytes.len() => {
                let next1 = bytes[i + 1];
                let next2 = bytes[i + 2];
                let next3 = bytes[i + 3];
                if (next1 & 0xC0) == 0x80 && (next2 & 0xC0) == 0x80 && (next3 & 0xC0) == 0x80 {
                    let code_point = ((current as u32 & 0x07) << 18)
                        | ((next1 as u32 & 0x3F) << 12)
                        | ((next2 as u32 & 0x3F) << 6)
                        | (next3 as u32 & 0x3F);
                    result.push(char::from_u32(code_point).unwrap_or('\u{FFFD}'));
                    i += 4;
                } else {
                    result.push('\u{FFFD}');
                    i += 1;
                }
            },
            _ => {
                result.push('\u{FFFD}');
                i += 1;
            },
        }
    }
}

#[cfg(test)]
mod replace_invalid_utf8_tests {
    use super::replace_invalid_utf8_and_utf16;

    fn cases() -> Vec<(&'static str, Vec<u8>, &'static str)> {
        vec![
            ("empty", vec![], ""),
            ("ascii", b"hello world".to_vec(), "hello world"),
            (
                "ascii_with_control",
                vec![b'a', 0x00, b'b', 0x7f, b'c'],
                "a\u{0}b\u{7f}c",
            ),
            ("two_byte_latin1", vec![0xC3, 0xA9], "\u{00E9}"),
            ("three_byte_cjk", vec![0xE4, 0xB8, 0x96], "\u{4e16}"),
            ("four_byte_emoji", vec![0xF0, 0x9F, 0xA6, 0x80], "\u{1f980}"),
            ("lone_high_surrogate", vec![0xED, 0xA0, 0xBD], "\u{FFFD}"),
            ("lone_low_surrogate", vec![0xED, 0xB0, 0x80], "\u{FFFD}"),
            (
                "surrogate_pair_in_wtf8",
                vec![0xED, 0xA0, 0xBD, 0xED, 0xB2, 0xA9],
                "\u{FFFD}\u{FFFD}",
            ),
            ("stray_continuation", vec![0x80], "\u{FFFD}"),
            ("truncated_two_byte", vec![0xC3], "\u{FFFD}"),
            ("truncated_three_byte", vec![0xE0, 0xA0], "\u{FFFD}\u{FFFD}"),
            (
                "truncated_four_byte",
                vec![0xF0, 0x9F, 0xA6],
                "\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (
                "two_byte_bad_continuation",
                vec![0xC3, 0x20, b'a'],
                "\u{FFFD} a",
            ),
            (
                "three_byte_bad_continuation",
                vec![0xE4, 0xB8, 0x20, b'a'],
                "\u{FFFD}\u{FFFD} a",
            ),
            ("high_byte_above_f7", vec![0xF8, b'a'], "\u{FFFD}a"),
            (
                "mixed_valid_and_invalid",
                {
                    let mut v = b"hello ".to_vec();
                    v.extend_from_slice(&[0xED, 0xA0, 0xBD]);
                    v.extend_from_slice(" world".as_bytes());
                    v
                },
                "hello \u{FFFD} world",
            ),
            (
                "long_ascii",
                b"the quick brown fox jumps over the lazy dog".repeat(20),
                &*Box::leak(
                    "the quick brown fox jumps over the lazy dog"
                        .repeat(20)
                        .into_boxed_str(),
                ),
            ),
        ]
    }

    #[test]
    fn matches_contract() {
        for (name, input, expected) in cases() {
            let got = replace_invalid_utf8_and_utf16(&input);
            assert_eq!(
                got, expected,
                "case `{}`: got {:?}, expected {:?}",
                name, got, expected
            );
        }
    }
}

use crate::{error_messages::ERROR_MSG_ARRAY_BUFFER_DETACHED, result::ResultExt};

#[derive(Clone, PartialEq)]
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
    DataView(ArrayBuffer<'js>, usize, usize), // buffer, offset, length
    Vec(Vec<u8>),
}

// Requires manual implementation because rquickjs hasn't implemented JsLifetime for f32 or f64
unsafe impl<'js> JsLifetime<'js> for ObjectBytes<'js> {
    type Changed<'to> = ObjectBytes<'to>;
}

impl<'js> Trace<'js> for ObjectBytes<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        match self {
            ObjectBytes::U8Array(a) => a.trace(tracer),
            ObjectBytes::I8Array(a) => a.trace(tracer),
            ObjectBytes::U16Array(a) => a.trace(tracer),
            ObjectBytes::I16Array(a) => a.trace(tracer),
            ObjectBytes::U32Array(a) => a.trace(tracer),
            ObjectBytes::I32Array(a) => a.trace(tracer),
            ObjectBytes::U64Array(a) => a.trace(tracer),
            ObjectBytes::I64Array(a) => a.trace(tracer),
            ObjectBytes::F32Array(a) => a.trace(tracer),
            ObjectBytes::F64Array(a) => a.trace(tracer),
            ObjectBytes::DataView(d, _, _) => d.trace(tracer),
            ObjectBytes::Vec(v) => v.trace(tracer),
        }
    }
}

impl<'js> IntoJs<'js> for ObjectBytes<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self {
            ObjectBytes::U8Array(a) => a.into_js(ctx),
            ObjectBytes::I8Array(a) => a.into_js(ctx),
            ObjectBytes::U16Array(a) => a.into_js(ctx),
            ObjectBytes::I16Array(a) => a.into_js(ctx),
            ObjectBytes::U32Array(a) => a.into_js(ctx),
            ObjectBytes::I32Array(a) => a.into_js(ctx),
            ObjectBytes::U64Array(a) => a.into_js(ctx),
            ObjectBytes::I64Array(a) => a.into_js(ctx),
            ObjectBytes::F32Array(a) => a.into_js(ctx),
            ObjectBytes::F64Array(a) => a.into_js(ctx),
            ObjectBytes::DataView(d, _, _) => {
                let ctor: Constructor = ctx.globals().get(PredefinedAtom::DataView)?;
                ctor.construct((d,))
            },
            ObjectBytes::Vec(v) => v.into_js(ctx),
        }
    }
}

impl<'js> TryFrom<ObjectBytes<'js>> for Vec<u8> {
    type Error = Rc<str>;
    fn try_from(value: ObjectBytes<'js>) -> std::result::Result<Self, Self::Error> {
        value.into_bytes_inner()
    }
}

impl<'a, 'js> TryFrom<&'a ObjectBytes<'js>> for &'a [u8] {
    type Error = Rc<str>;
    fn try_from(value: &'a ObjectBytes<'js>) -> std::result::Result<Self, Self::Error> {
        value.as_bytes_inner()
    }
}

impl<'js> FromJs<'js> for ObjectBytes<'js> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        Self::from_offset(ctx, &value, 0, None)
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

    pub fn as_bytes(&self, ctx: &Ctx<'js>) -> Result<&[u8]> {
        self.as_bytes_inner().or_throw(ctx)
    }

    /// Returns the underlying bytes, or `None` if the buffer is detached.
    /// Unlike [`as_bytes`], does not raise a JS exception — useful when the
    /// caller needs to distinguish detachment from other errors (e.g.
    /// WebIDL-style "treat detached BufferSource as empty").
    pub fn as_bytes_opt(&self) -> Option<&[u8]> {
        self.as_bytes_inner().ok()
    }

    fn as_bytes_inner(&self) -> std::result::Result<&[u8], Rc<str>> {
        match self {
            ObjectBytes::U8Array(array) => array.as_bytes(),
            ObjectBytes::I8Array(array) => array.as_bytes(),
            ObjectBytes::U16Array(array) => array.as_bytes(),
            ObjectBytes::I16Array(array) => array.as_bytes(),
            ObjectBytes::U32Array(array) => array.as_bytes(),
            ObjectBytes::I32Array(array) => array.as_bytes(),
            ObjectBytes::U64Array(array) => array.as_bytes(),
            ObjectBytes::I64Array(array) => array.as_bytes(),
            ObjectBytes::F32Array(array) => array.as_bytes(),
            ObjectBytes::F64Array(array) => array.as_bytes(),
            ObjectBytes::DataView(array_buffer, offset, length) => array_buffer
                .as_bytes()
                .map(|b| &b[*offset..*offset + *length]),
            ObjectBytes::Vec(bytes) => Some(bytes.as_ref()),
        }
        .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED.into())
    }

    pub fn into_bytes(self, ctx: &Ctx<'_>) -> Result<Vec<u8>> {
        self.into_bytes_inner().or_throw(ctx)
    }

    fn into_bytes_inner(self) -> std::result::Result<Vec<u8>, Rc<str>> {
        if let ObjectBytes::Vec(bytes) = self {
            return Ok(bytes);
        }
        Ok(self.as_bytes_inner()?.to_vec())
    }

    pub fn from_array_buffer(obj: &Object<'js>) -> Result<Option<ObjectBytes<'js>>> {
        //most common
        if let Ok(typed_array) = TypedArray::<u8>::from_object(obj.clone()) {
            return Ok(Some(ObjectBytes::U8Array(typed_array)));
        }
        //second most common
        if let Some(array_buffer) = ArrayBuffer::from_object(obj.clone()) {
            let len = array_buffer.len();
            return Ok(Some(ObjectBytes::DataView(array_buffer, 0, len)));
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
            let byte_offset: usize = obj.get("byteOffset").unwrap_or(0);
            let byte_length: usize = obj.get("byteLength").unwrap_or_else(|_| array_buffer.len());
            return Ok(Some(ObjectBytes::DataView(
                array_buffer,
                byte_offset,
                byte_length,
            )));
        }

        Ok(None)
    }

    pub fn get_array_buffer(&self) -> Result<Option<(ArrayBuffer<'js>, usize, usize)>> {
        let buffer = match self {
            ObjectBytes::DataView(array_buffer, offset, length) => {
                (array_buffer.clone(), *length, *offset)
            },
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
        return Some(bytes_from_js_string(val.0, offset, length));
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
    if value.is_string() {
        let string = get_lossy_string(value.clone())?;
        return Ok(Some(bytes_from_js_string(string, offset, length)));
    }
    Ok(None)
}

pub fn bytes_to_typed_array<'js>(ctx: Ctx<'js>, bytes: &[u8]) -> Result<Value<'js>> {
    TypedArray::<u8>::new(ctx.clone(), bytes).into_js(&ctx)
}
