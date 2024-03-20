// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::ops::RangeInclusive;

use rquickjs::{
    class::Trace, function::Opt, ArrayBuffer, Class, Coerced, Ctx, Exception, FromJs, Object,
    Result, Value,
};

enum EndingType {
    Native,
    Transparent,
}

#[cfg(windows)]
const LINE_ENDING: &[u8] = b"\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &[u8] = b"\n";

#[rquickjs::class]
#[derive(Trace, Clone)]
pub struct Blob {
    #[qjs(skip_trace)]
    data: Vec<u8>,
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
impl Blob {
    #[qjs(constructor)]
    pub fn new<'js>(
        ctx: Ctx<'js>,
        parts: Opt<Value<'js>>,
        options: Opt<Object<'js>>,
    ) -> Result<Self> {
        let mut endings = EndingType::Transparent;
        let mut mime_type = String::new();

        if let Some(opts) = options.0 {
            if let Some(x) = opts.get::<_, Option<Coerced<String>>>("type")? {
                mime_type = normalize_type(x.to_string());
            }
            if let Some(Coerced(endings_opt)) = opts.get::<_, Option<Coerced<String>>>("endings")? {
                if endings_opt == "native" {
                    endings = EndingType::Native;
                } else if endings_opt != "transparent" {
                    return Err(Exception::throw_type(
                        &ctx,
                        r#"expected 'endings' to be either 'transparent' or 'native'"#,
                    ));
                }
            }
        }

        let data = if let Some(parts) = parts.0 {
            bytes_from_parts(&ctx, parts, endings)?
        } else {
            Vec::new()
        };

        Ok(Self { data, mime_type })
    }

    #[qjs(get)]
    pub fn size(&self) -> usize {
        self.data.len()
    }

    #[qjs(get, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    pub async fn text(&mut self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }

    #[qjs(rename = "arrayBuffer")]
    pub async fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        ArrayBuffer::new(ctx, self.data.to_vec())
    }

    pub fn slice(&self, start: Opt<isize>, end: Opt<isize>, content_type: Opt<String>) -> Blob {
        let start = start.0.unwrap_or_default();
        let start = if start < 0 {
            (self.data.len() as isize + start).max(0) as usize
        } else {
            start as usize
        };
        let end = end.0.unwrap_or_default();
        let end = if end < 0 {
            (self.data.len() as isize + end).max(0) as usize
        } else {
            end as usize
        };
        let data = &self.data[start..end];
        let mime_type = content_type.0.map(normalize_type).unwrap_or_default();

        Blob {
            mime_type,
            data: data.to_vec(),
        }
    }
}

impl Blob {
    pub fn from_bytes(data: Vec<u8>, content_type: Option<String>) -> Self {
        let mime_type = content_type.map(normalize_type).unwrap_or_default();
        Self { mime_type, data }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}

fn bytes_from_parts<'js>(
    ctx: &Ctx<'js>,
    parts: Value<'js>,
    endings: EndingType,
) -> Result<Vec<u8>> {
    let array = parts.into_array().ok_or_else(|| {
        Exception::throw_type(
            ctx,
            "Failed to construct 'Blob': The provided value cannot be converted to a sequence.",
        )
    })?;
    let mut data = Vec::new();
    for elem in array.iter::<Value>() {
        let elem = elem?;
        if let Some(object) = elem.as_object() {
            if let Some(x) = Class::<Blob>::from_object(object.clone()) {
                data.extend_from_slice(&x.borrow().data);
                continue;
            }
            if let Some(x) = ArrayBuffer::from_object(object.clone()) {
                data.extend_from_slice(x.as_bytes().ok_or_else(|| {
                    Exception::throw_type(ctx, "Cannot create a blob with detached buffer")
                })?);
                continue;
            }
        }
        let string = Coerced::<String>::from_js(ctx, elem)?.0;
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

            // let bytes = string.as_bytes();

            // let input_reader = std::io::BufReader::new(bytes);
            // for line in input_reader.lines() {
            //     let line = line?;
            //     data.extend_from_slice(line.as_bytes());
            //     data.extend_from_slice(LINE_ENDING);
            // }

            // let len = data.len();

            // match &bytes[string_len - min(string_len, 2)..] {
            //     LINE_ENDING => {}
            //     _ => data.truncate(len - LINE_ENDING.len()),
            // }
        }
    }
    Ok(data)
}
