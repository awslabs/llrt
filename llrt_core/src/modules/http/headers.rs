// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

use hyper::HeaderMap;
use rquickjs::{
    atom::PredefinedAtom, methods, prelude::Opt, Array, Coerced, Ctx, FromJs, Function, Result,
    Value,
};

use crate::utils::{class::IteratorDef, object::map_to_entries};

const HEADERS_KEY_COOKIE: &str = "cookie";
const HEADERS_KEY_SET_COOKIE: &str = "set-cookie";

#[derive(Clone, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Headers {
    #[qjs(skip_trace)]
    headers: Vec<(String, String)>,
}

#[methods(rename_all = "camelCase")]
impl Headers {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_array() {
                let array = init.into_array().unwrap();
                let headers = Self::array_to_headers(array)?;
                return Ok(Self { headers });
            } else if init.is_object() {
                return Self::from_value(&ctx, init);
            }
        }
        Ok(Self {
            headers: Vec::new(),
        })
    }

    pub fn append(&mut self, key: String, value: String) {
        let key = key.to_lowercase();
        if key == HEADERS_KEY_SET_COOKIE {
            return self.headers.push((key, value));
        }
        if let Some((_, existing_value)) = self.headers.iter_mut().find(|(k, _)| k == &key) {
            match key.as_str() {
                HEADERS_KEY_COOKIE => existing_value.push_str("; "),
                _ => existing_value.push_str(", "),
            }
            existing_value.push_str(&value);
        } else {
            self.headers.push((key, value));
        }
    }

    pub fn get(&self, key: String) -> Option<String> {
        let key = key.to_lowercase();
        if key == HEADERS_KEY_SET_COOKIE {
            let result: Vec<String> = self
                .headers
                .iter()
                .filter_map(|(k, v)| if k == &key { Some(v.clone()) } else { None })
                .collect();
            return if result.is_empty() {
                None
            } else {
                Some(result.join(", "))
            };
        }
        self.headers
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.clone())
    }

    pub fn get_set_cookie(&self) -> Vec<String> {
        self.headers
            .iter()
            .filter_map(|(k, v)| {
                if k == HEADERS_KEY_SET_COOKIE {
                    Some(v.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn has(&self, key: String) -> bool {
        let key = key.to_lowercase();
        self.headers.iter().any(|(k, _)| k == &key)
    }

    pub fn set(&mut self, key: String, value: String) {
        let key = key.to_lowercase();
        if key == HEADERS_KEY_SET_COOKIE
            && self.headers.iter().filter(|(k, _)| k == &key).count() > 1
        {
            self.headers.retain(|(k, _)| k != &key);
        }
        if let Some((_, existing_value)) = self.headers.iter_mut().find(|(k, _)| k == &key) {
            *existing_value = value;
        } else {
            self.headers.push((key, value));
        }
    }

    pub fn delete(&mut self, key: String) {
        let key = key.to_lowercase();
        self.headers.retain(|(k, _)| k != &key);
    }

    pub fn keys(&self) -> Vec<String> {
        self.headers.iter().map(|(k, _)| k.clone()).collect()
    }

    pub fn values(&self) -> Vec<String> {
        self.headers.iter().map(|(_, v)| v.clone()).collect()
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
            callback.call((v.clone(), k.clone()))?;
        }
        Ok(())
    }
}

impl Headers {
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.headers.iter().map(|(k, v)| (k, v))
    }

    pub fn from_http_headers(header_map: &HeaderMap) -> Result<Self> {
        let mut headers = Vec::new();
        for (n, v) in header_map.iter() {
            headers.push((
                n.to_string(),
                String::from_utf8_lossy(v.as_bytes()).to_string(),
            ));
        }
        Ok(Self { headers })
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
        let headers = map
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v.to_string()))
            .collect();
        Self { headers }
    }

    fn array_to_headers(array: Array<'_>) -> Result<Vec<(String, String)>> {
        let mut vec = Vec::new();
        for entry in array.into_iter().flatten() {
            if let Some(array_entry) = entry.as_array() {
                let key = array_entry.get::<String>(0)?.to_lowercase();
                let value = array_entry.get::<String>(1)?;
                vec.push((key, value));
            }
        }
        Ok(vec)
    }
}

impl<'js> IteratorDef<'js> for Headers {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        map_to_entries(&ctx, self.headers.clone())
    }
}
