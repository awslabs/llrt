// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::BTreeMap, rc::Rc};

use hyper::HeaderMap;
use llrt_utils::{
    class::{CustomInspect, IteratorDef},
    object::map_to_entries,
};
use rquickjs::{
    atom::PredefinedAtom, methods, prelude::Opt, Array, Coerced, Ctx, Exception, FromJs, Function,
    IntoJs, Null, Object, Result, Value,
};

const HEADERS_KEY_COOKIE: &str = "cookie";
const HEADERS_KEY_SET_COOKIE: &str = "set-cookie";

type ImmutableString = Rc<str>;

#[derive(Clone, Default, rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct Headers {
    #[qjs(skip_trace)]
    headers: Vec<(ImmutableString, ImmutableString)>,
}

#[methods(rename_all = "camelCase")]
impl Headers {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_array() {
                let array = unsafe { init.into_array().unwrap_unchecked() };
                let headers = Self::array_to_headers(&ctx, array)?;
                return Ok(Self { headers });
            } else if init.is_null() || init.is_number() {
                return Err(Exception::throw_type(&ctx, "Invalid argument"));
            } else if init.is_object() {
                return Self::from_value(&ctx, init);
            }
        }
        Ok(Self {
            headers: Vec::new(),
        })
    }

    pub fn append<'js>(&mut self, ctx: Ctx<'js>, key: String, value: String) -> Result<()> {
        let key: ImmutableString = key.to_lowercase().into();
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        let value: Rc<str> = value.into();
        if !is_http_header_value(&value) {
            return Err(Exception::throw_type(&ctx, "Invalid value of key"));
        }

        let str_key = key.as_ref();
        if str_key == HEADERS_KEY_SET_COOKIE {
            return {
                self.headers.push((key, value));
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

    pub fn set<'js>(&mut self, ctx: Ctx<'js>, key: String, value: String) -> Result<()> {
        let key: ImmutableString = key.to_lowercase().into();
        if !is_http_header_name(&key) {
            return Err(Exception::throw_type(&ctx, "Invalid key"));
        }

        let value: Rc<str> = value.into();
        if !is_http_header_value(&value) {
            return Err(Exception::throw_type(&ctx, "Invalid value of key"));
        }

        if key.as_ref() == HEADERS_KEY_SET_COOKIE {
            self.headers.retain(|(k, _)| k != &key);
            self.headers.push((key, value));
        } else {
            match self.headers.iter_mut().find(|(k, _)| k == &key) {
                Some((_, existing_value)) => *existing_value = value,
                None => self.headers.push((key, value)),
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

    pub fn from_http_headers(header_map: &HeaderMap) -> Result<Self> {
        let mut headers = Vec::new();
        for (n, v) in header_map.iter() {
            headers.push((
                n.as_str().into(),
                String::from_utf8_lossy(v.as_bytes()).into(),
            ));
        }
        Ok(Self { headers })
    }

    pub fn from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.is_object() {
            let headers_obj = unsafe { value.as_object().unwrap_unchecked() };
            return if headers_obj.instance_of::<Headers>() {
                Headers::from_js(ctx, value)
            } else {
                let map: BTreeMap<String, Coerced<String>> = value.get().unwrap_or_default();
                return Ok(Self::from_map(map));
            };
        }
        Ok(Headers::default())
    }

    pub fn from_map(map: BTreeMap<String, Coerced<String>>) -> Self {
        let headers = map
            .into_iter()
            .map(|(k, v)| (k.to_lowercase().into(), v.to_string().into()))
            .collect();
        Self { headers }
    }

    fn array_to_headers<'js>(
        ctx: &Ctx<'js>,
        array: Array<'js>,
    ) -> Result<Vec<(ImmutableString, ImmutableString)>> {
        let mut vec: Vec<(ImmutableString, ImmutableString)> = Vec::new();

        for entry in array.into_iter().flatten() {
            if let Some(array_entry) = entry.as_array() {
                if array_entry.clone().into_iter().flatten().count() % 2 != 0 {
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
        Ok("null".to_string())
    } else if let Some(s) = value.as_string() {
        Ok(s.to_string()?)
    } else {
        // fallback: try JSON.stringify or [object Object]
        let global = ctx.globals();
        let string_ctor: Function = global.get("String")?;
        let result: String = string_ctor.call((value,))?;
        Ok(result)
    }
}

fn is_http_header_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // 3.2.6.  Field Value Components
    // https://datatracker.ietf.org/doc/html/rfc7230#section-3.2.6
    name.bytes().all(|b| {
        matches!(b,
            b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' |
            b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~' |
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z'
        )
    })
}

fn is_http_header_value(value: &str) -> bool {
    let bytes = value.as_bytes();

    // Reject leading/trailing SP or HTAB
    if bytes.first().is_some_and(|&b| b == 0x20 || b == 0x09) {
        return false;
    }
    if bytes.last().is_some_and(|&b| b == 0x20 || b == 0x09) {
        return false;
    }

    // Validate chars same as sanitize
    bytes.iter().all(|&b| {
        b == 0x09  // HTAB
        || b == 0x20 // SP
        || (0x21..=0x7E).contains(&b) // VCHAR
        || b == 0x0C // \f
        || b == 0xA0 // NBSP
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
                        "application/json".into(),
                    )
                    .unwrap();
                headers
                    .append(ctx.clone(), "set-cookie".into(), "cookie1=value1".into())
                    .unwrap();
                headers
                    .append(ctx.clone(), "set-cookie".into(), "cookie2=value2".into())
                    .unwrap();
                headers
                    .append(ctx.clone(), "Accept-Encoding".into(), "deflate".into())
                    .unwrap();
                headers
                    .append(ctx.clone(), "Accept-Encoding".into(), "gzip".into())
                    .unwrap();

                assert_eq!(
                    headers
                        .get(ctx.clone(), "Content-Type".into())
                        .unwrap()
                        .as_string()
                        .unwrap()
                        .to_string()
                        .unwrap(),
                    "application/json"
                );
                assert_eq!(
                    headers
                        .get(ctx.clone(), "set-cookie".into())
                        .unwrap()
                        .as_string()
                        .unwrap()
                        .to_string()
                        .unwrap(),
                    "cookie1=value1, cookie2=value2"
                );
                assert_eq!(
                    headers
                        .get(ctx.clone(), "Accept-Encoding".into())
                        .unwrap()
                        .as_string()
                        .unwrap()
                        .to_string()
                        .unwrap(),
                    "deflate, gzip"
                );
            })
        })
        .await;
    }
}
