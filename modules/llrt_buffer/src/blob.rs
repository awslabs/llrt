// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::ops::RangeInclusive;

use llrt_stream_web::{
    readable_byte_stream_controller_close_stream,
    readable_byte_stream_controller_enqueue_bytes_borrowed, utils::promise::PromisePrimordials,
    CancelAlgorithm, PullAlgorithm, ReadableStream, ReadableStreamControllerClass,
};
use llrt_utils::{
    array_buffer::shared_array_buffer_view,
    bytes::{get_lossy_string, ObjectBytes},
    primordials::Primordial,
    result::ResultExt,
    string::get_coerced_defined_string,
};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, prelude::This, Array, ArrayBuffer, Class,
    Coerced, Ctx, Exception, FromJs, IntoJs, JsIterator, Result, TypedArray, Value,
};

use super::file::File;

struct ArrayPartsIter<'js> {
    array: Array<'js>,
    index: usize,
}

impl<'js> ArrayPartsIter<'js> {
    fn new(array: Array<'js>) -> Self {
        Self { array, index: 0 }
    }
}

impl<'js> Iterator for ArrayPartsIter<'js> {
    type Item = Result<Value<'js>>;

    fn next(&mut self) -> Option<Self::Item> {
        let len: usize = match self.array.as_object().get("length") {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        if self.index >= len {
            return None;
        }
        let result = self.array.get(self.index);
        self.index += 1;
        Some(result)
    }
}

enum EndingType {
    Native,
    Transparent,
}

#[cfg(windows)]
const LINE_ENDING: &[u8] = b"\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &[u8] = b"\n";

#[rquickjs::class]
#[derive(Trace, Clone, rquickjs::JsLifetime)]
pub struct Blob<'js> {
    /// Bytes live in a JS-owned `ArrayBuffer` so `.arrayBuffer()` / `.bytes()`
    /// / `.stream()` can hand out refcount-bumped views without copying.
    data: ArrayBuffer<'js>,
    mime_type: String,
}

fn normalize_type(mut mime_type: String) -> String {
    static INVALID_RANGE: RangeInclusive<u8> = 0x0020..=0x007E;

    let bytes = unsafe { mime_type.as_bytes_mut() };
    for byte in bytes {
        if !INVALID_RANGE.contains(byte) {
            return String::new();
        }
        byte.make_ascii_lowercase();
    }
    mime_type
}

#[rquickjs::methods]
impl<'js> Blob<'js> {
    #[qjs(constructor)]
    pub fn new(
        ctx: Ctx<'js>,
        this: This<Value<'js>>,
        parts: Opt<Value<'js>>,
        options: Opt<Value<'js>>,
    ) -> Result<Self> {
        if this.as_function().is_none() {
            return Err(Exception::throw_type(
                &ctx,
                "Failed to construct 'Blob': Please use the 'new' operator",
            ));
        }

        Self::from_parts(ctx, parts, options)
    }

    #[qjs(get)]
    pub fn size(&self) -> usize {
        self.data.len()
    }

    #[qjs(get, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    pub async fn text(&self) -> String {
        String::from_utf8_lossy(self.as_bytes()).to_string()
    }

    #[qjs(rename = "arrayBuffer")]
    pub async fn array_buffer(&self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        //should be mutable according to spec, thus copy is required
        ArrayBuffer::new_copy(ctx, self.as_bytes())
    }

    pub async fn bytes(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        //should be mutable according to spec, thus copy is required
        let ab = ArrayBuffer::new_copy(ctx, self.as_bytes())?;
        TypedArray::<u8>::from_arraybuffer(ab).map(|t| t.into_value())
    }

    pub fn slice(
        &self,
        ctx: Ctx<'js>,
        start: Opt<Value<'js>>,
        end: Opt<Value<'js>>,
        content_type: Opt<Value<'js>>,
    ) -> Result<Blob<'js>> {
        let start = start.0.and_then(|v| v.as_number()).map(clamp_long_long);
        let end = end.0.and_then(|v| v.as_number()).map(clamp_long_long);
        Self::slice_blob(self, &ctx, start, end, content_type.0)
    }

    pub fn stream(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let data = self.data.clone();
        let pull = PullAlgorithm::from_fn_once(
            move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
                let ctrl = match controller {
                    ReadableStreamControllerClass::ReadableStreamByteController(c) => c,
                    _ => return Err(Exception::throw_type(&ctx, "Expected byte controller")),
                };
                let len = data.len();
                if len != 0 {
                    let view = shared_array_buffer_view(&ctx, &data, 0, len)?;
                    readable_byte_stream_controller_enqueue_bytes_borrowed(
                        ctx.clone(),
                        ctrl.clone(),
                        view,
                    )?;
                }
                readable_byte_stream_controller_close_stream(ctx.clone(), ctrl)?;
                Ok(PromisePrimordials::get(&ctx)?
                    .promise_resolved_with_undefined
                    .clone())
            },
        );
        // Byte-source stream so callers can use `getReader({ mode: 'byob' })`.
        // Matches spec: Blob.stream() returns a `type: "bytes"` ReadableStream.
        let stream = ReadableStream::from_byte_pull_algorithm(
            ctx,
            pull,
            CancelAlgorithm::ReturnPromiseUndefined,
        )?;
        Ok(stream.into_value())
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(Blob)
    }

    #[qjs(static, rename = PredefinedAtom::SymbolHasInstance)]
    pub fn has_instance(value: Value<'js>) -> bool {
        if let Some(obj) = value.as_object() {
            return obj.instance_of::<Self>() || obj.instance_of::<File>();
        }
        false
    }

    #[qjs(skip)]
    pub fn slice_blob(
        &self,
        ctx: &Ctx<'js>,
        start: Option<isize>,
        end: Option<isize>,
        content_type: Option<Value<'js>>,
    ) -> Result<Blob<'js>> {
        let bytes = self.as_bytes();
        let len = bytes.len();
        let start = start.unwrap_or_default();
        let start = if start < 0 {
            (len as isize + start).max(0) as usize
        } else {
            len.min(start as usize)
        };
        let end = end.unwrap_or(len as isize);
        let end = if end < 0 {
            (len as isize + end).max(0) as usize
        } else {
            len.min(end as usize)
        };
        let data = shared_array_buffer_view(ctx, &self.data, start, end.saturating_sub(start))?;
        let mime_type = get_coerced_defined_string(&content_type);
        let mime_type = mime_type.map(normalize_type).unwrap_or_default();
        Ok(Blob { mime_type, data })
    }
}

impl<'js> Blob<'js> {
    pub fn from_bytes(ctx: &Ctx<'js>, data: Vec<u8>, content_type: Option<String>) -> Result<Self> {
        let mime_type = content_type.map(normalize_type).unwrap_or_default();
        let data = ArrayBuffer::new(ctx.clone(), data)?;
        Ok(Self { mime_type, data })
    }

    pub fn from_parts(
        ctx: Ctx<'js>,
        parts: Opt<Value<'js>>,
        options: Opt<Value<'js>>,
    ) -> Result<Self> {
        let mut endings = EndingType::Transparent;
        let mut mime_type = String::new();

        if let Some(options) = options.0 {
            if let Some(opts) = options.as_object() {
                if let Some(x) = opts.get::<_, Option<Coerced<String>>>("type")? {
                    mime_type = normalize_type(x.to_string());
                }

                if opts.contains_key("endings")? {
                    if let Some(parsed) = parse_endings(&ctx, opts.get("endings")?)? {
                        endings = parsed;
                    }
                }
            }
        }

        let bytes = if let Some(parts) = parts.0 {
            bytes_from_parts(&ctx, parts, endings)?
        } else {
            Vec::new()
        };
        // Transfer Vec ownership to JS — QuickJS calls the drop callback when
        // the ArrayBuffer is GC'd, so no extra Rust-side copy.
        let data = ArrayBuffer::new(ctx, bytes)?;

        Ok(Self { data, mime_type })
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    /// Zero-copy access to the underlying `ArrayBuffer`. Cloning the handle is
    /// cheap (it's a JS-refcount bump); no bytes are copied. Useful for
    /// consumers that want to pass the Blob body on to hyper via
    /// `ObjectBytes::DataView` without the `get_bytes()` allocation.
    pub fn array_buffer_ref(&self) -> ArrayBuffer<'js> {
        self.data.clone()
    }

    /// Borrow the underlying bytes directly. Returns `&[]` if the ArrayBuffer
    /// has been detached (shouldn't happen in normal blob flow).
    pub fn as_bytes(&self) -> &[u8] {
        self.data.as_bytes().unwrap_or(&[])
    }
}

fn bytes_from_parts<'js>(
    ctx: &Ctx<'js>,
    parts: Value<'js>,
    endings: EndingType,
) -> Result<Vec<u8>> {
    if parts.is_undefined() {
        return Ok(Vec::new());
    }

    if let Some(array) = parts.clone().into_array() {
        return process_parts(ctx, ArrayPartsIter::new(array), endings);
    }

    process_parts(ctx, JsIterator::from_js(ctx, parts)?, endings)
}

fn process_parts<'js, I>(ctx: &Ctx<'js>, iter: I, endings: EndingType) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = Result<Value<'js>>>,
{
    let mut data = Vec::new();
    for elem in iter {
        let elem = elem?;
        if let Some(arr) = elem.as_array() {
            let string = array_to_string(arr)?;
            data.extend_from_slice(string.as_bytes());
            continue;
        }
        if let Some(object) = elem.as_object() {
            if let Some(x) = Class::<Blob>::from_object(object) {
                data.extend_from_slice(x.borrow().as_bytes());
                continue;
            }
            if let Some(x) = Class::<File>::from_object(object) {
                let file = x.borrow();
                let end = Some(file.size().try_into().or_throw(ctx)?);
                let mime_type = Some(file.mime_type().into_js(ctx)?);
                let sub = file.slice(ctx.clone(), Opt(Some(0)), Opt(end), Opt(mime_type))?;
                data.extend_from_slice(sub.as_bytes());
                continue;
            }
            if let Ok(x) = ObjectBytes::from(ctx, object) {
                data.extend_from_slice(x.as_bytes(ctx).map_err(|_| {
                    Exception::throw_type(ctx, "Cannot create a blob with detached buffer")
                })?);
                continue;
            }
            if let Some(x) = ArrayBuffer::from_object(object.clone()) {
                data.extend_from_slice(x.as_bytes().ok_or_else(|| {
                    Exception::throw_type(ctx, "Cannot create a blob with detached buffer")
                })?);
                continue;
            }
        }

        let string = if elem.is_string() {
            get_lossy_string(elem)?
        } else {
            Coerced::<String>::from_js(ctx, elem)?.0
        };
        if let EndingType::Transparent = endings {
            data.extend_from_slice(string.as_bytes());
        } else {
            let len = string.len();
            data.reserve(len);

            let bytes = string.as_bytes();
            let mut iter = bytes.iter();

            let mut start = 0usize;
            let mut i = 0usize;
            let line_ending_is_n = LINE_ENDING[0] == b'\n';

            while let Some(byte) = iter.next() {
                if byte == &b'\r' {
                    if let Some(next_byte) = iter.next() {
                        data.extend(&bytes[start..i]);
                        i += 1;
                        start = i + 1;
                        if next_byte != &b'\n' {
                            data.extend([b'\r', *next_byte]);
                        } else {
                            data.extend(LINE_ENDING);
                        }
                    }
                } else if byte == &b'\n' && !line_ending_is_n {
                    data.extend(&bytes[start..i]);
                    data.extend(LINE_ENDING);
                    start = i + 1;
                };
                i += 1;
            }

            if start < len {
                data.extend(&bytes[start..len]);
            }
        }
    }
    Ok(data)
}

fn parse_endings<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Option<EndingType>> {
    if value.is_undefined() {
        return Ok(None);
    }
    let endings = match Coerced::<String>::from_js(ctx, value)?.0.as_str() {
        "transparent" => Some(EndingType::Transparent),
        "native" => Some(EndingType::Native),
        _ => {
            return Err(Exception::throw_type(
                ctx,
                r#"expected 'endings' to be either 'transparent' or 'native'"#,
            ));
        },
    };
    Ok(endings)
}

fn array_to_string(array: &Array) -> Result<String> {
    let mut itoa_buffer = itoa::Buffer::new();
    let mut ryu_buffer = ryu::Buffer::new();

    let parts = array
        .clone()
        .into_iter()
        .map(|value| {
            let value = value?;
            if let Some(string) = value.as_string() {
                Ok(string.to_string()?)
            } else if let Some(number) = value.as_int() {
                Ok(itoa_buffer.format(number).to_string())
            } else if let Some(number) = value.as_float() {
                Ok(ryu_buffer.format(number).to_string())
            } else {
                Ok(String::new())
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(parts.join(","))
}

fn clamp_long_long(value: f64) -> isize {
    if value.is_nan() {
        return 0;
    }
    let rounded = value.round_ties_even();
    rounded.clamp(isize::MIN as f64, isize::MAX as f64) as isize
}
