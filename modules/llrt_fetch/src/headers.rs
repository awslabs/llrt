// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::{collections::HashSet, rc::Rc};

use hyper::{
    header::{
        ACCEPT, ACCEPT_CHARSET, ACCEPT_ENCODING, ACCEPT_LANGUAGE, ACCESS_CONTROL_REQUEST_HEADERS,
        ACCESS_CONTROL_REQUEST_METHOD, CONNECTION, CONTENT_LANGUAGE, CONTENT_LENGTH, CONTENT_TYPE,
        COOKIE, DATE, EXPECT, HOST, ORIGIN, REFERER, SET_COOKIE, TE, TRAILER, TRANSFER_ENCODING,
        UPGRADE, VIA,
    },
    HeaderMap,
};
use llrt_utils::{
    class::CustomInspect,
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, methods, prelude::Opt, prelude::This, Array, Class, Ctx,
    Exception, Function, IntoJs, JsLifetime, Null, Object, Result, Symbol, Value,
};

type ImmutableString = Rc<str>;

/// ASCII-lowercase the key in place (HTTP headers are ASCII-only) and wrap in `Rc<str>`,
/// saving one allocation vs `key.to_lowercase().into()`.
fn lower_key(mut key: String) -> ImmutableString {
    key.make_ascii_lowercase();
    key.into()
}

// https://fetch.spec.whatwg.org/#concept-headers-guard
#[derive(Clone, Copy, Default, PartialEq)]
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
    pub(crate) guard: HeadersGuard,
}

#[methods(rename_all = "camelCase")]
impl Headers {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_null() || init.is_number() {
                return Err(Exception::throw_type(&ctx, "Invalid argument"));
            }
            return Self::from_value(&ctx, init, HeadersGuard::None);
        }

        Ok(Self {
            headers: Vec::new(),
            guard: HeadersGuard::None,
        })
    }

    pub fn append<'js>(&mut self, ctx: Ctx<'js>, key: String, value: Value<'js>) -> Result<()> {
        let key = lower_key(key);
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }
        if self.guard == HeadersGuard::Immutable {
            return Err(Exception::throw_type(&ctx, "Headers are immutable"));
        }
        if matches!(
            self.guard,
            HeadersGuard::Request | HeadersGuard::RequestNoCors
        ) && is_forbidden_request_header(&key)
        {
            return Ok(());
        }
        if self.guard == HeadersGuard::Response && key.as_ref() == SET_COOKIE.as_str() {
            return Ok(());
        }

        let mut value = coerce_to_string(&ctx, value)?;
        // Reject values containing null bytes or bare CR/LF;
        // `normalize_header_value_inplace` silently strips them, but
        // `header-setcookie` expects a TypeError for such values.
        if value.contains('\0') || has_bare_cr_lf(&value) {
            return Err(Exception::throw_type(&ctx, "Invalid header value"));
        }
        normalize_header_value_inplace(&ctx, &mut value)?;
        // Value-based forbidden header check (must run after value normalisation).
        if matches!(
            self.guard,
            HeadersGuard::Request | HeadersGuard::RequestNoCors
        ) && is_forbidden_method_override(&key, &value)
        {
            return Ok(());
        }
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
        if str_key == SET_COOKIE.as_str() {
            self.headers.push((key, value.into()));
            return Ok(());
        }
        if let Some((_, existing_value)) = self.headers.iter_mut().find(|(k, _)| k == &key) {
            let mut new_value = String::with_capacity(existing_value.len() + 2 + value.len());
            new_value.push_str(existing_value);
            if str_key == COOKIE.as_str() {
                new_value.push_str("; ");
            } else {
                new_value.push_str(", ");
            }
            new_value.push_str(&value);
            *existing_value = new_value.into();
        } else {
            self.headers.push((key, value.into()));
        }
        Ok(())
    }

    pub fn get<'js>(&self, ctx: Ctx<'js>, key: String) -> Result<Value<'js>> {
        let key = lower_key(key);
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        if key.as_ref() == SET_COOKIE.as_str() {
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
                if k.as_ref() == SET_COOKIE.as_str() {
                    Some(v.as_ref())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn has<'js>(&self, ctx: Ctx<'js>, key: String) -> Result<bool> {
        let key = lower_key(key);
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        Ok(self.headers.iter().any(|(k, _)| k == &key))
    }

    pub fn set<'js>(&mut self, ctx: Ctx<'js>, key: String, value: Value<'js>) -> Result<()> {
        let key = lower_key(key);
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }
        if self.guard == HeadersGuard::Immutable {
            return Err(Exception::throw_type(&ctx, "Headers are immutable"));
        }
        if matches!(
            self.guard,
            HeadersGuard::Request | HeadersGuard::RequestNoCors
        ) && is_forbidden_request_header(&key)
        {
            return Ok(());
        }
        if self.guard == HeadersGuard::Response && key.as_ref() == SET_COOKIE.as_str() {
            return Ok(());
        }

        let mut value = coerce_to_string(&ctx, value)?;
        // Reject values containing null bytes or bare CR/LF;
        // `normalize_header_value_inplace` silently strips them, but
        // `header-setcookie` expects a TypeError for such values.
        if value.contains('\0') || has_bare_cr_lf(&value) {
            return Err(Exception::throw_type(&ctx, "Invalid header value"));
        }
        normalize_header_value_inplace(&ctx, &mut value)?;
        // Value-based forbidden header check (must run after value normalisation).
        if matches!(
            self.guard,
            HeadersGuard::Request | HeadersGuard::RequestNoCors
        ) && is_forbidden_method_override(&key, &value)
        {
            return Ok(());
        }
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

        if key.as_ref() == SET_COOKIE.as_str() {
            self.headers.retain(|(k, _)| k != &key);
            self.headers.push((key, value.into()));
        } else {
            match self.headers.iter_mut().find(|(k, _)| k == &key) {
                Some((_, existing_value)) => *existing_value = value.into(),
                None => {
                    self.headers.push((key, value.into()));
                },
            }
        }
        Ok(())
    }

    pub fn delete<'js>(&mut self, ctx: Ctx<'js>, key: String) -> Result<()> {
        let key = lower_key(key);
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }
        if self.guard == HeadersGuard::Immutable {
            return Err(Exception::throw_type(&ctx, "Headers are immutable"));
        }
        if matches!(
            self.guard,
            HeadersGuard::Request | HeadersGuard::RequestNoCors
        ) && is_forbidden_request_header(&key)
        {
            return Ok(());
        }
        if self.guard == HeadersGuard::Response && key.as_ref() == SET_COOKIE.as_str() {
            return Ok(());
        }

        self.headers.retain(|(k, _)| k != &key);
        Ok(())
    }

    pub fn keys<'js>(
        this: This<Class<'js, Headers>>,
        ctx: Ctx<'js>,
    ) -> Result<Class<'js, HeadersIter<'js>>> {
        HeadersIter::create(&ctx, this.0, HeadersIterKind::Keys)
    }

    pub fn values<'js>(
        this: This<Class<'js, Headers>>,
        ctx: Ctx<'js>,
    ) -> Result<Class<'js, HeadersIter<'js>>> {
        HeadersIter::create(&ctx, this.0, HeadersIterKind::Values)
    }

    pub fn entries<'js>(
        this: This<Class<'js, Headers>>,
        ctx: Ctx<'js>,
    ) -> Result<Class<'js, HeadersIter<'js>>> {
        HeadersIter::create(&ctx, this.0, HeadersIterKind::Entries)
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator<'js>(
        this: This<Class<'js, Headers>>,
        ctx: Ctx<'js>,
    ) -> Result<Class<'js, HeadersIter<'js>>> {
        HeadersIter::create(&ctx, this.0, HeadersIterKind::Entries)
    }

    pub fn for_each<'js>(this: This<Class<'js, Headers>>, callback: Function<'js>) -> Result<()> {
        let sorted = this.0.borrow().sorted_entries();
        for (k, v) in &sorted {
            () = callback.call((v.as_ref(), k.as_ref(), this.0.clone()))?;
        }
        Ok(())
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(Headers)
    }
}

impl Headers {
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter().map(|(k, v)| (k.as_ref(), v.as_ref()))
    }

    /// `has()` for Rust callers who already know the key is lowercase ASCII.
    /// Avoids the `String` → `Rc<str>` allocation of the JS-exposed `has`.
    pub(crate) fn contains_lower(&self, key: &str) -> bool {
        self.headers.iter().any(|(k, _)| k.as_ref() == key)
    }

    /// Returns the sorted header list per the Fetch spec.
    /// Non-set-cookie headers are combined and sorted alphabetically.
    /// Set-cookie headers are kept separate in insertion order at their
    /// alphabetical position.
    fn sorted_entries(&self) -> Vec<(ImmutableString, ImmutableString)> {
        let mut result: Vec<(ImmutableString, ImmutableString)> =
            Vec::with_capacity(self.headers.len());
        let mut seen = HashSet::with_capacity(self.headers.len());

        for (k, v) in &self.headers {
            if k.as_ref() == SET_COOKIE.as_str() || seen.insert(k.clone()) {
                result.push((k.clone(), v.clone()));
            }
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    pub fn from_http_headers(header_map: &HeaderMap, guard: HeadersGuard) -> Result<Self> {
        let mut headers = Vec::with_capacity(header_map.keys_len());
        for (n, v) in header_map.iter() {
            headers.push((
                n.as_str().into(),
                String::from_utf8_lossy(v.as_bytes()).into(),
            ));
        }
        Ok(Self { headers, guard })
    }

    pub fn from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>, guard: HeadersGuard) -> Result<Self> {
        if value.is_array() {
            let array = unsafe { value.into_array().unwrap_unchecked() };
            return Self::from_array(ctx, array, guard);
        }

        if let Some(obj) = value.as_object() {
            if obj
                .get::<_, Value>(Symbol::iterator(ctx.clone()))?
                .is_function()
            {
                let array: Array = BasePrimordials::get(ctx)?
                    .function_array_from
                    .call((value,))?;
                return Self::from_array(ctx, array, guard);
            } else {
                // WebIDL record conversion: keys via Reflect.ownKeys (preserves
                // Symbols, unlike rquickjs's Atom-based iterator which stringifies
                // them), then per-key gopd → enumerable check → get. Ordering
                // matters for Proxy traps (WPT headers-record.any.js).
                let (get_own_property_desc_fn, own_keys_fn) = {
                    let primordials = BasePrimordials::get(ctx)?;
                    let get_own_property_desc_fn =
                        &primordials.function_get_own_property_descriptor;
                    let own_keys_fn = &primordials.function_reflect_own_keys;
                    (get_own_property_desc_fn.clone(), own_keys_fn.clone())
                };

                let keys: Array = own_keys_fn.call((obj.clone(),))?;

                let mut headers: Vec<(ImmutableString, ImmutableString)> =
                    Vec::with_capacity(keys.len());
                for i in 0..keys.len() {
                    let key_val: Value = keys.get(i)?;
                    let desc: Value =
                        get_own_property_desc_fn.call((obj.clone(), key_val.clone()))?;
                    let Some(desc) = desc.as_object() else {
                        continue;
                    };
                    if !desc.get::<_, bool>("enumerable").unwrap_or(false) {
                        continue;
                    }
                    if key_val.is_symbol() {
                        return Err(Exception::throw_type(
                            ctx,
                            "Cannot convert a Symbol value to a string",
                        ));
                    }
                    let Some(key) = key_val.as_string().map(|s| s.to_string()).transpose()? else {
                        continue;
                    };
                    let mut k_lower = key;
                    k_lower.make_ascii_lowercase();
                    if !is_http_header_name(&k_lower) {
                        return Err(Exception::throw_type(ctx, "Invalid header name"));
                    }
                    if matches!(guard, HeadersGuard::Request | HeadersGuard::RequestNoCors)
                        && is_forbidden_request_header(&k_lower)
                    {
                        continue;
                    }
                    let raw_value: Value = obj.get(key_val)?;
                    let mut value = coerce_to_string(ctx, raw_value)?;
                    let _ = normalize_header_value_inplace(ctx, &mut value);
                    if !is_http_header_value(&value) {
                        return Err(Exception::throw_type(ctx, "Invalid header value"));
                    }
                    if matches!(guard, HeadersGuard::Request | HeadersGuard::RequestNoCors)
                        && is_forbidden_method_override(&k_lower, &value)
                    {
                        continue;
                    }
                    if guard == HeadersGuard::RequestNoCors
                        && !is_cors_safelisted_request_header(&k_lower, &value)
                    {
                        continue;
                    }
                    headers.push((k_lower.into(), value.into()));
                }
                headers.sort_by(|a, b| a.0.cmp(&b.0));
                return Ok(Self { headers, guard });
            }
        }

        Ok(Self {
            headers: vec![],
            guard,
        })
    }

    fn from_array<'js>(ctx: &Ctx<'js>, array: Array<'js>, guard: HeadersGuard) -> Result<Self> {
        let mut headers: Vec<(ImmutableString, ImmutableString)> = Vec::with_capacity(array.len());

        for entry in array.into_iter().flatten() {
            if let Some(array_entry) = entry.as_array() {
                if array_entry.len() % 2 != 0 {
                    return Err(Exception::throw_type(ctx, "Header arrays are not paired"));
                }

                let mut raw_key = array_entry.get::<String>(0)?;
                raw_key.make_ascii_lowercase();
                if !is_http_header_name(&raw_key) {
                    return Err(Exception::throw_type(ctx, "Invalid key"));
                }
                // Skip forbidden headers
                if matches!(guard, HeadersGuard::Request | HeadersGuard::RequestNoCors)
                    && is_forbidden_request_header(&raw_key)
                {
                    continue;
                }

                let raw_value = array_entry.get::<Value>(1)?;
                let value: ImmutableString = coerce_to_string(ctx, raw_value)?.into();
                if !is_http_header_value(&value) {
                    return Err(Exception::throw_type(ctx, "Invalid value of key"));
                }

                if matches!(guard, HeadersGuard::Request | HeadersGuard::RequestNoCors)
                    && is_forbidden_method_override(&raw_key, &value)
                {
                    continue;
                }

                // Skip non-safelisted headers in no-cors mode
                if guard == HeadersGuard::RequestNoCors
                    && !is_cors_safelisted_request_header(&raw_key, &value)
                {
                    continue;
                }

                if raw_key == SET_COOKIE.as_str() {
                    let key: ImmutableString = raw_key.into();
                    headers.push((key, value));
                    continue;
                }

                if let Some((_, existing_value)) =
                    headers.iter_mut().find(|(k, _)| k.as_ref() == raw_key)
                {
                    let mut new_value = existing_value.to_string();

                    if raw_key.as_str() == COOKIE.as_str() {
                        new_value.push_str("; ");
                    } else {
                        new_value.push_str(", ");
                    }

                    new_value.push_str(&value);
                    *existing_value = ImmutableString::from(new_value);
                } else {
                    let key: ImmutableString = raw_key.into();
                    headers.push((key, value));
                }
            }
        }

        headers.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(Self { headers, guard })
    }
}

#[derive(Clone, Copy)]
enum HeadersIterKind {
    Keys,
    Values,
    Entries,
}

#[derive(Trace, JsLifetime)]
#[rquickjs::class]
pub struct HeadersIter<'js> {
    headers: Class<'js, Headers>,
    #[qjs(skip_trace)]
    index: usize,
    #[qjs(skip_trace)]
    kind: HeadersIterKind,
}

#[rquickjs::methods]
impl<'js> HeadersIter<'js> {
    fn next(&mut self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        // Re-read on every next() — WPT expects the iterator to observe
        // mutations made during iteration (insertions, deletions).
        let sorted = self.headers.borrow().sorted_entries();

        if self.index < sorted.len() {
            let (k, v) = &sorted[self.index];
            self.index += 1;
            obj.set("done", false)?;
            match self.kind {
                HeadersIterKind::Keys => obj.set("value", k.as_ref())?,
                HeadersIterKind::Values => obj.set("value", v.as_ref())?,
                HeadersIterKind::Entries => {
                    let entry = Array::new(ctx)?;
                    entry.set(0, k.as_ref())?;
                    entry.set(1, v.as_ref())?;
                    obj.set("value", entry)?;
                },
            }
        } else {
            obj.set("done", true)?;
        }
        Ok(obj)
    }
}

impl<'js> HeadersIter<'js> {
    fn create(
        ctx: &Ctx<'js>,
        headers: Class<'js, Headers>,
        kind: HeadersIterKind,
    ) -> Result<Class<'js, Self>> {
        Class::instance(
            ctx.clone(),
            Self {
                headers,
                index: 0,
                kind,
            },
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
        // `new String(value)` returns a boxed String object; unwrap to primitive.
        let base_primordials = BasePrimordials::get(ctx)?;
        let obj: Object = base_primordials.constructor_string.construct((value,))?;
        let value_of: Function = obj.get("valueOf")?;
        let prim: Value = value_of.call((This(obj),))?;
        match prim.into_string() {
            Some(s) => s.to_string(),
            None => Ok(String::new()),
        }
    }
}

// 3.2.6.  Field Value Components
// https://datatracker.ietf.org/doc/html/rfc7230#section-3.2.6

fn is_forbidden_request_header(name: &str) -> bool {
    name == ACCEPT_CHARSET
        || name == ACCEPT_ENCODING
        || name == ACCESS_CONTROL_REQUEST_HEADERS
        || name == ACCESS_CONTROL_REQUEST_METHOD
        || name == CONNECTION
        || name == CONTENT_LENGTH
        || name == COOKIE
        || name == "cookie2"
        || name == DATE
        || name == "dnt"
        || name == EXPECT
        || name == HOST
        || name == "keep-alive"
        || name == ORIGIN
        || name == REFERER
        || name == SET_COOKIE
        || name == TE
        || name == TRAILER
        || name == TRANSFER_ENCODING
        || name == UPGRADE
        || name == VIA
        || name.starts_with("proxy-")
        || name.starts_with("sec-")
}

/// Per Fetch spec, `x-http-method`, `x-http-method-override`, `x-method-override`
/// with a value that parses to a forbidden method (CONNECT/TRACE/TRACK, case-
/// insensitive) count as forbidden headers too.
fn is_forbidden_method_override(name: &str, value: &str) -> bool {
    if !matches!(
        name,
        "x-http-method" | "x-http-method-override" | "x-method-override"
    ) {
        return false;
    }
    value
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .any(|tok| {
            matches!(
                tok.to_ascii_uppercase().as_str(),
                "CONNECT" | "TRACE" | "TRACK"
            )
        })
}
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

/// Check for bare CR or LF in the interior of the value (not leading/trailing)
fn has_bare_cr_lf(value: &str) -> bool {
    let trimmed = value.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\n' || c == '\r');
    trimmed.bytes().any(|b| b == b'\n' || b == b'\r')
}
fn is_http_header_value(value: &str) -> bool {
    value.chars().all(|c| {
        let cp = c as u32;
        cp == 0x09                          // HTAB
        || cp == 0x0C                       // Form Feed
        || (0x20..=0x7E).contains(&cp)      // SP + VCHAR
        || (0x80..=0xFF).contains(&cp) // obs-text
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

    if key.eq_ignore_ascii_case(ACCEPT.as_str()) {
        !contains_cors_unsafe_request_header_byte(value)
    } else if key.eq_ignore_ascii_case(ACCEPT_LANGUAGE.as_str())
        || key.eq_ignore_ascii_case(CONTENT_LANGUAGE.as_str())
    {
        is_cors_safelisted_field_value(value)
    } else if key.eq_ignore_ascii_case(CONTENT_TYPE.as_str()) {
        if contains_cors_unsafe_request_header_byte(value) {
            return false;
        }
        let mime_type = value.split(';').next().unwrap_or("").trim();
        [
            "application/x-www-form-urlencoded",
            "multipart/form-data",
            "text/plain",
            "",
        ]
        .iter()
        .any(|c| mime_type.eq_ignore_ascii_case(c))
    } else {
        false
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

    #[tokio::test]
    async fn test_headers_iterators() {
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                let result: bool = ctx
                    .eval(
                        r#"
                        const h = new Headers([['x-first', '1'], ['x-second', '2']]);
                        const iter = h.keys()[Symbol.iterator]();
                        iter.next().value === 'x-first' &&
                        iter.next().value === 'x-second' &&
                        iter.next().done === true
                        "#,
                    )
                    .unwrap();
                assert!(result, "keys() iterator failed");

                let result: bool = ctx
                    .eval(
                        r#"
                        const h2 = new Headers([['x-first', '1'], ['x-second', '2']]);
                        const iter2 = h2.values()[Symbol.iterator]();
                        iter2.next().value === '1' &&
                        iter2.next().value === '2' &&
                        iter2.next().done === true
                        "#,
                    )
                    .unwrap();
                assert!(result, "values() iterator failed");
            })
        })
        .await;
    }
}
