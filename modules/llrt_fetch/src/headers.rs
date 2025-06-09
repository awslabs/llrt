// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::BTreeMap, rc::Rc};

use hyper::HeaderMap;
use llrt_utils::{
    class::{CustomInspect, IteratorDef},
    object::map_to_entries,
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, methods, prelude::Opt, Array, Coerced, Ctx, Exception,
    FromJs, Function, IntoJs, JsLifetime, Null, Object, Result, Symbol, Value,
};

const HEADERS_KEY_COOKIE: &str = "cookie";
const HEADERS_KEY_SET_COOKIE: &str = "set-cookie";

type ImmutableString = Rc<str>;

// https://fetch.spec.whatwg.org/#concept-headers-guard
#[derive(Clone, Default, PartialEq)]
pub enum HeadersGuard {
    #[default]
    None,
    Request,
    RequestNoCors,
    Response,
    Immutable,
}

#[derive(Clone, Default, Trace, JsLifetime)]
#[rquickjs::class]
pub struct Headers {
    #[qjs(skip_trace)]
    headers: Vec<(ImmutableString, ImmutableString)>,
    #[qjs(skip_trace)]
    guard: HeadersGuard,
}

#[methods(rename_all = "camelCase")]
impl Headers {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_array() {
                let array = unsafe { init.into_array().unwrap_unchecked() };
                let headers = Self::array_to_headers(&ctx, array)?;
                return Ok(Self {
                    headers,
                    guard: HeadersGuard::None,
                });
            } else if init.is_null() || init.is_number() {
                return Err(Exception::throw_type(&ctx, "Invalid argument"));
            } else if init.is_object() {
                if let Some(obj) = init.as_object() {
                    if obj.contains_key(Symbol::iterator(ctx.clone()))? {
                        let array: Array = BasePrimordials::get(&ctx)?
                            .function_array_from
                            .call((init,))?;
                        return Ok(Self {
                            headers: Self::array_to_headers(&ctx, array)?,
                            guard: HeadersGuard::None,
                        });
                    }
                }
                return Self::from_value(&ctx, init, HeadersGuard::None);
            }
        }
        Ok(Self {
            headers: Vec::new(),
            guard: HeadersGuard::None,
        })
    }

    pub fn append<'js>(&mut self, ctx: Ctx<'js>, key: String, value: Value<'js>) -> Result<()> {
        let key: ImmutableString = key.to_lowercase().into();
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        let mut value = coerce_to_string(&ctx, value)?;
        normalize_header_value_inplace(&ctx, &mut value)?;
        if self.guard == HeadersGuard::RequestNoCors {
            let val = value.split(',').next().unwrap_or("").trim();
            if !is_cors_safelisted_request_header(&key, val) {
                return Ok(()); // silently ignore disallowed header
            }
            if self.headers.iter().any(|(k, _)| k == &key) {
                return Ok(()); // silently ignore same header
            }
            value = val.into();
        };
        if !is_http_header_value(&value) {
            return Err(Exception::throw_type(&ctx, "Invalid value of key"));
        }

        let str_key = key.as_ref();
        if str_key == HEADERS_KEY_SET_COOKIE {
            return {
                self.headers.push((key, value.into()));
                Ok(())
            };
        }
        if let Some((_, existing_value)) = self.headers.iter_mut().find(|(k, _)| k == &key) {
            let mut new_value = String::with_capacity(existing_value.len() + 2 + value.len());
            new_value.push_str(existing_value);
            match str_key {
                HEADERS_KEY_COOKIE => new_value.push_str("; "),
                _ => new_value.push_str(", "),
            }
            new_value.push_str(&value);
            *existing_value = new_value.into();
        } else {
            self.headers.push((key, value.into()));
        }
        Ok(())
    }

    pub fn get<'js>(&self, ctx: Ctx<'js>, key: String) -> Result<Value<'js>> {
        let key: ImmutableString = key.to_lowercase().into();
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        if key.as_ref() == HEADERS_KEY_SET_COOKIE {
            let result: Vec<&str> = self
                .headers
                .iter()
                .filter_map(|(k, v)| if k == &key { Some(v.as_ref()) } else { None })
                .collect();
            return if result.is_empty() {
                Null.into_js(&ctx)
            } else {
                result.join(", ").into_js(&ctx)
            };
        }
        self.headers
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.as_ref().into_js(&ctx))
            .unwrap_or_else(|| Null.into_js(&ctx))
    }

    pub fn get_set_cookie(&self) -> Vec<&str> {
        self.headers
            .iter()
            .filter_map(|(k, v)| {
                if k.as_ref() == HEADERS_KEY_SET_COOKIE {
                    Some(v.as_ref())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn has<'js>(&self, ctx: Ctx<'js>, key: String) -> Result<bool> {
        let key: ImmutableString = key.to_lowercase().into();
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        Ok(self.headers.iter().any(|(k, _)| k == &key))
    }

    pub fn set<'js>(&mut self, ctx: Ctx<'js>, key: String, value: Value<'js>) -> Result<()> {
        let key: ImmutableString = key.to_lowercase().into();
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        let mut value = coerce_to_string(&ctx, value)?;
        normalize_header_value_inplace(&ctx, &mut value)?;
        if self.guard == HeadersGuard::RequestNoCors {
            let val = value.split(',').next().unwrap_or("").trim();
            if !is_cors_safelisted_request_header(&key, val) {
                return Ok(()); // silently ignore disallowed header
            }
            value = val.into();
        }
        if !is_http_header_value(&value) {
            return Err(Exception::throw_type(&ctx, "Invalid value of key"));
        }

        if key.as_ref() == HEADERS_KEY_SET_COOKIE {
            self.headers.retain(|(k, _)| k != &key);
            self.headers.push((key, value.into()));
        } else {
            match self.headers.iter_mut().find(|(k, _)| k == &key) {
                Some((_, existing_value)) => *existing_value = value.into(),
                None => self.headers.push((key, value.into())),
            }
        }
        Ok(())
    }

    pub fn delete<'js>(&mut self, ctx: Ctx<'js>, key: String) -> Result<()> {
        let key: ImmutableString = key.to_lowercase().into();
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        self.headers.retain(|(k, _)| k != &key);
        Ok(())
    }

    pub fn keys(&self) -> Vec<&str> {
        self.headers.iter().map(|(k, _)| k.as_ref()).collect()
    }

    pub fn values(&self) -> Vec<&str> {
        self.headers.iter().map(|(_, v)| v.as_ref()).collect()
    }

    pub fn entries<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    pub fn for_each(&self, callback: Function<'_>) -> Result<()> {
        for (k, v) in &self.headers {
            () = callback.call((v.as_ref(), k.as_ref()))?;
        }
        Ok(())
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(Headers)
    }
}

impl Headers {
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter().map(|(k, v)| (k.as_ref(), v.as_ref()))
    }

    pub fn from_http_headers(header_map: &HeaderMap, guard: HeadersGuard) -> Result<Self> {
        let mut headers = Vec::new();
        for (n, v) in header_map.iter() {
            headers.push((
                n.as_str().into(),
                String::from_utf8_lossy(v.as_bytes()).into(),
            ));
        }
        Ok(Self { headers, guard })
    }

    pub fn from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>, guard: HeadersGuard) -> Result<Self> {
        if value.is_object() {
            let headers_obj = unsafe { value.as_object().unwrap_unchecked() };
            return if headers_obj.instance_of::<Headers>() {
                Headers::from_js(ctx, value)
            } else {
                let map: BTreeMap<String, Coerced<String>> = value.get().unwrap_or_default();
                return Ok(Self::from_map(ctx, map, guard));
            };
        }
        Ok(Self {
            headers: vec![],
            guard,
        })
    }

    pub fn from_map(
        ctx: &Ctx<'_>,
        map: BTreeMap<String, Coerced<String>>,
        guard: HeadersGuard,
    ) -> Self {
        let headers = map
            .into_iter()
            .filter_map(|(k, v)| {
                if !is_http_header_name(&k) {
                    return None;
                }
                let mut value = v.0;
                let _ = normalize_header_value_inplace(ctx, &mut value);
                Some((k.to_lowercase().into(), value.into()))
            })
            .collect::<Vec<(Rc<str>, Rc<str>)>>();

        Self { headers, guard }
    }

    fn array_to_headers<'js>(
        ctx: &Ctx<'js>,
        array: Array<'js>,
    ) -> Result<Vec<(ImmutableString, ImmutableString)>> {
        let mut vec: Vec<(ImmutableString, ImmutableString)> = Vec::new();

        for entry in array.into_iter().flatten() {
            if let Some(array_entry) = entry.as_array() {
                if array_entry.len() % 2 != 0 {
                    return Err(Exception::throw_type(ctx, "Header arrays are not paired"));
                }

                let raw_key = array_entry.get::<String>(0)?.to_lowercase();
                let key: Rc<str> = ImmutableString::from(raw_key.clone());
                if !is_http_header_name(&key) {
                    return Err(Exception::throw_type(ctx, "Invalid key"));
                }

                let raw_value = array_entry.get::<Value>(1)?;
                let value: ImmutableString = coerce_to_string(ctx, raw_value)?.into();
                if !is_http_header_value(&value) {
                    return Err(Exception::throw_type(ctx, "Invalid value of key"));
                }

                if raw_key == HEADERS_KEY_SET_COOKIE {
                    vec.push((key, value));
                    continue;
                }

                if let Some((_, existing_value)) = vec.iter_mut().find(|(k, _)| *k == key) {
                    let mut new_value = existing_value.to_string();

                    match raw_key.as_str() {
                        HEADERS_KEY_COOKIE => new_value.push_str("; "),
                        _ => new_value.push_str(", "),
                    }

                    new_value.push_str(&value);
                    *existing_value = ImmutableString::from(new_value);
                } else {
                    vec.push((key, value));
                }
            }
        }

        vec.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(vec)
    }
}

impl<'js> IteratorDef<'js> for Headers {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        map_to_entries(
            &ctx,
            self.headers.iter().map(|(k, v)| (k.as_ref(), v.as_ref())),
        )
    }
}

impl<'js> CustomInspect<'js> for Headers {
    fn custom_inspect(&self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        let obj = Object::new(ctx)?;
        for (k, v) in self.headers.iter() {
            obj.set(k.as_ref(), v.as_ref())?;
        }

        Ok(obj)
    }
}

fn coerce_to_string<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<String> {
    if value.is_null() {
        Ok("null".into())
    } else if value.is_undefined() {
        Ok("undefined".into())
    } else if let Some(v) = value.as_bool() {
        Ok(v.to_string())
    } else if let Some(v) = value.as_number() {
        Ok(v.to_string())
    } else if let Some(s) = value.as_string() {
        s.to_string()
    } else {
        // fallback: try JSON.stringify or [object Object]
        let base_primordials = BasePrimordials::get(ctx)?;
        base_primordials.constructor_string.construct((value,))
    }
}

// 3.2.6.  Field Value Components
// https://datatracker.ietf.org/doc/html/rfc7230#section-3.2.6
fn is_http_header_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    name.bytes().all(|b| {
        matches!(b,
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' |
            b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~' |
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
        )
    })
}

fn is_http_header_value(value: &str) -> bool {
    value.chars().all(|c| {
        c == '\t'                // HTAB
        || c == ' '              // SP
        || (('\u{21}'..='\u{7E}').contains(&c)) // VCHAR range
        || c == '\u{0C}'         // Form Feed
        || c == '\u{00A0}' // NBSP
    })
}

fn normalize_header_value_inplace(ctx: &Ctx<'_>, text: &mut String) -> Result<()> {
    let mut input = std::mem::take(text).into_bytes();
    let mut read_idx = 0;
    let mut write_idx = 0;

    // Skip leading SP or HTAB
    while read_idx < input.len() && (input[read_idx] == b' ' || input[read_idx] == b'\t') {
        read_idx += 1;
    }

    // Store the last whitespace byte if any (space or tab)
    let mut pending_whitespace: Option<u8> = None;

    while read_idx < input.len() {
        match input[read_idx] {
            // obs-fold: CRLF followed by SP or HTAB
            b'\r'
                if read_idx + 2 < input.len()
                    && input[read_idx + 1] == b'\n'
                    && (input[read_idx + 2] == b' ' || input[read_idx + 2] == b'\t') =>
            {
                pending_whitespace = Some(input[read_idx + 2]);
                read_idx += 3;
            },
            b'\r' | b'\n' => {
                // skip bare CR or LF
                read_idx += 1;
            },
            b' ' | b'\t' => {
                // keep the last whitespace char to write later (collapse multiple)
                pending_whitespace = Some(input[read_idx]);
                read_idx += 1;
            },
            byte => {
                // write pending whitespace if any
                if let Some(ws) = pending_whitespace.take() {
                    if write_idx > 0 {
                        input[write_idx] = ws;
                        write_idx += 1;
                    }
                }
                input[write_idx] = byte;
                write_idx += 1;
                read_idx += 1;
            },
        }
    }

    // Trim trailing SP or HTAB
    while write_idx > 0 && (input[write_idx - 1] == b' ' || input[write_idx - 1] == b'\t') {
        write_idx -= 1;
    }

    input.truncate(write_idx);
    *text = String::from_utf8(input).or_throw(ctx)?;
    Ok(())
}

// https://fetch.spec.whatwg.org/#cors-safelisted-request-header
pub fn is_cors_safelisted_request_header(key: &str, value: &str) -> bool {
    if value.len() > 128 {
        return false;
    }

    match key.to_ascii_lowercase().as_str() {
        "accept" => !contains_cors_unsafe_request_header_byte(value),
        "accept-language" | "content-language" => is_cors_safelisted_field_value(value),
        "content-type" => {
            if contains_cors_unsafe_request_header_byte(value) {
                return false;
            }
            let mime_type = value.split(';').next().unwrap_or("").trim();
            matches!(
                mime_type.to_ascii_lowercase().as_str(),
                "application/x-www-form-urlencoded" | "multipart/form-data" | "text/plain" | ""
            )
        },
        _ => false,
    }
}

// https://fetch.spec.whatwg.org/#cors-unsafe-request-header-byte
pub fn contains_cors_unsafe_request_header_byte(value: &str) -> bool {
    for byte in value.bytes() {
        match byte {
            // Control characters except for HT (0x09)
            0x00..=0x08 | 0x0A..=0x1F => return true,

            // byte is 0x22 ("), 0x28 (left parenthesis), 0x29 (right parenthesis), 0x3A (:), 0x3C (<),
            // 0x3E (>), 0x3F (?), 0x40 (@), 0x5B ([), 0x5C (\), 0x5D (]), 0x7B ({), 0x7D (}), or 0x7F DEL.
            0x22 | 0x28 | 0x29 | 0x3A | 0x3C | 0x3E | 0x3F | 0x40 | 0x5B | 0x5C | 0x5D | 0x7B
            | 0x7F | 0x7D => return true,

            _ => {}, // Allowed byte
        }
    }
    false
}

pub fn is_cors_safelisted_field_value(value: &str) -> bool {
    value.bytes().all(|b| match b {
        0x30..=0x39 | // 0-9
        0x41..=0x5A | // A-Z
        0x61..=0x7A | // a-z
        0x20 | 0x2A | 0x2C | 0x2D | 0x2E | 0x3B | 0x3D => true, // allowed symbols
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use llrt_test::test_async_with;

    use super::*;

    #[tokio::test]
    async fn test_get_header() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                let mut headers = Headers::new(ctx.clone(), Opt(None)).unwrap();
                headers
                    .set(
                        ctx.clone(),
                        "Content-Type".into(),
                        "application/json".into_js(&ctx).unwrap(),
                    )
                    .unwrap();

                let append_headers = [
                    ("set-cookie", "cookie1=value1"),
                    ("set-cookie", "cookie2=value2"),
                    ("Accept-Encoding", "deflate"),
                    ("Accept-Encoding", "gzip"),
                ];
                for (key, value) in append_headers {
                    headers
                        .append(ctx.clone(), key.into(), value.into_js(&ctx).unwrap())
                        .unwrap();
                }

                let get_headers = [
                    ("Content-Type", "application/json"),
                    ("set-cookie", "cookie1=value1, cookie2=value2"),
                    ("Accept-Encoding", "deflate, gzip"),
                ];
                for (key, expected) in get_headers {
                    assert_eq!(
                        headers
                            .get(ctx.clone(), key.into())
                            .unwrap()
                            .as_string()
                            .unwrap()
                            .to_string()
                            .unwrap(),
                        expected
                    );
                }
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_normalize_header_value_inplace() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                // https://github.com/web-platform-tests/wpt/blob/master/fetch/api/headers/headers-normalize.any.js
                let expectations = [
                    (" space ", "space"),
                    ("\ttab\t", "tab"),
                    (" spaceAndTab\t", "spaceAndTab"),
                    ("\r\n newLine", "newLine"),
                    ("newLine\r\n ", "newLine"),
                    ("\r\n\tnewLine", "newLine"),
                    ("\t\u{000C}\tnewLine\n", "\u{000C}\tnewLine"), //  \f = \u{000C}
                    ("newLine\u{00A0}", "newLine\u{00A0}"),   // \u{00A0} = NBSP
                ];
                for (input, expected) in expectations {
                    let mut value = input.to_string();
                    super::normalize_header_value_inplace(&ctx, &mut value).unwrap();
                    assert_eq!(
                        value,
                        expected,
                        "normalize_header_value_inplace failed: input = {:?}, expected = {:?}, got = {:?}",
                        input,
                        expected,
                        value
                    );
                }
            })
        })
        .await;
    }
}
