// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::BTreeMap, fmt};

use hyper::HeaderMap;
use rquickjs::{
    atom::PredefinedAtom, methods, prelude::Opt, Array, Coerced, Ctx, FromJs, Function, IntoJs,
    Result, Value,
};

use crate::utils::{
    class::IteratorDef,
    object::{array_to_btree_map, map_to_entries},
};

const HEADERS_KEY_SET_COOKIE: &str = "set-cookie";

#[derive(Clone)]
pub enum HeaderValue {
    Single(String),
    Multiple(Vec<String>),
}

impl fmt::Display for HeaderValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeaderValue::Single(s) => write!(f, "{}", s),
            HeaderValue::Multiple(v) => write!(f, "{}", v.join("")),
        }
    }
}

impl<'js> IntoJs<'js> for HeaderValue {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self {
            HeaderValue::Single(s) => s.into_js(ctx),
            HeaderValue::Multiple(v) => v.join(", ").into_js(ctx),
        }
    }
}

#[derive(Clone, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Headers {
    #[qjs(skip_trace)]
    headers: BTreeMap<String, HeaderValue>,
}

#[methods(rename_all = "camelCase")]
impl Headers {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_array() {
                let array = init.into_array().unwrap();
                let headers = array_to_btree_map(&ctx, array)?;
                return Ok(Self::from_map(headers));
            } else if init.is_object() {
                return Self::from_value(&ctx, init);
            }
        }
        Ok(Headers::default())
    }

    pub fn append(&mut self, key: String, value: String) {
        self.insert_header_value(key, value, true);
    }

    pub fn get(&mut self, key: String) -> Option<String> {
        match self.headers.get(&key.to_lowercase()).map(|v| v.to_owned()) {
            Some(HeaderValue::Single(s)) => Some(s),
            Some(HeaderValue::Multiple(v)) => Some(v.join(", ")),
            _ => None,
        }
    }

    pub fn get_set_cookie(&mut self) -> Vec<String> {
        match self
            .headers
            .get(HEADERS_KEY_SET_COOKIE)
            .map(|v| v.to_owned())
        {
            Some(HeaderValue::Multiple(v)) => v,
            _ => Vec::new(),
        }
    }

    pub fn has(&mut self, key: String) -> bool {
        self.headers.contains_key(&key.to_lowercase())
    }

    pub fn set(&mut self, key: String, value: String) {
        self.insert_header_value(key, value, false);
    }

    pub fn delete(&mut self, key: String) {
        self.headers.remove(&key.to_lowercase());
    }

    pub fn keys(&mut self) -> Vec<String> {
        self.headers.keys().cloned().collect::<Vec<String>>()
    }

    pub fn values(&mut self) -> Vec<HeaderValue> {
        self.headers.values().cloned().collect::<Vec<HeaderValue>>()
    }

    pub fn entries<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    pub fn for_each(&self, callback: Function<'_>) -> Result<()> {
        for (key, value) in &self.headers {
            match value {
                HeaderValue::Single(s) => callback.call((s, key))?,
                HeaderValue::Multiple(v) => callback.call((v, key))?,
            }
        }
        Ok(())
    }
}

impl Headers {
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.headers
            .iter()
            .flat_map(|(k, v)| match v {
                HeaderValue::Single(s) => Some(vec![(k, s)].into_iter()),
                HeaderValue::Multiple(v) => Some(
                    v.iter()
                        .map(move |s| (k, s as &String))
                        .collect::<Vec<_>>()
                        .into_iter(),
                ),
            })
            .flatten()
    }

    pub fn from_http_headers(header_map: &HeaderMap) -> Result<Self> {
        let mut headers = Headers::default();

        for (key, value) in header_map.iter() {
            let mapping_value = String::from_utf8_lossy(value.as_bytes()).to_string();
            headers.insert_header_value(key.to_string(), mapping_value, true);
        }
        Ok(headers)
    }

    pub fn from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.is_object() {
            let headers_obj = value.as_object().unwrap();
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
        let mut headers = Headers::default();

        for (key, value) in map {
            headers.insert_header_value(key, value.to_string(), true);
        }
        headers
    }

    fn insert_header_value(&mut self, key: String, value: String, appending: bool) {
        let key = key.to_lowercase();

        if let Some(header_value) = self.headers.get_mut(&key) {
            match header_value {
                HeaderValue::Single(existing_value) => match appending {
                    true => {
                        *header_value =
                            HeaderValue::Single(format!("{}, {}", existing_value, value))
                    },
                    false => *header_value = HeaderValue::Single(value),
                },
                HeaderValue::Multiple(existing_values) => match appending {
                    true => existing_values.push(value),
                    false => {
                        existing_values.clear();
                        existing_values.push(value);
                    },
                },
            };
        } else {
            match key.as_str() {
                HEADERS_KEY_SET_COOKIE => self
                    .headers
                    .insert(key.to_string(), HeaderValue::Multiple(vec![value])),
                _ => self
                    .headers
                    .insert(key.to_string(), HeaderValue::Single(value)),
            };
        }
    }
}

impl<'js> IteratorDef<'js> for Headers {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        map_to_entries(&ctx, self.headers.clone())
    }
}
