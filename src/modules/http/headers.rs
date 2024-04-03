// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::BTreeMap;

use hyper::HeaderMap;
use rquickjs::{
    atom::PredefinedAtom, methods, prelude::Opt, Array, Coerced, Ctx, FromJs, Function, Result,
    Value,
};

use crate::utils::{
    class::IteratorDef,
    object::{array_to_btree_map, map_to_entries},
    result::ResultExt,
};

#[derive(Clone, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Headers {
    #[qjs(skip_trace)]
    headers: BTreeMap<String, String>,
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

        Ok(Self {
            headers: BTreeMap::default(),
        })
    }

    pub fn append(&mut self, key: String, value: String) {
        let key = key.to_lowercase();

        self.headers
            .entry(key)
            .and_modify(|header| *header = format!("{}, {}", header, &value))
            .or_insert_with(|| value);
    }

    pub fn get(&mut self, key: String) -> Option<String> {
        self.headers.get(&key.to_lowercase()).map(|v| v.to_owned())
    }

    pub fn has(&mut self, key: String) -> bool {
        self.headers.contains_key(&key.to_lowercase())
    }

    pub fn set(&mut self, key: String, value: String) {
        self.headers.insert(key.to_lowercase(), value);
    }

    pub fn delete(&mut self, key: String) {
        self.headers.remove(&key.to_lowercase());
    }

    pub fn keys(&mut self) -> Vec<String> {
        self.headers.keys().cloned().collect::<Vec<String>>()
    }

    pub fn values(&mut self) -> Vec<String> {
        self.headers.values().cloned().collect::<Vec<String>>()
    }

    pub fn entries<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    pub fn for_each(&self, callback: Function<'_>) -> Result<()> {
        for header in self.headers.iter() {
            callback.call((header.1, header.0))?
        }
        Ok(())
    }
}

impl Headers {
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.headers.iter()
    }

    pub fn from_http_headers(ctx: &Ctx<'_>, header_map: &HeaderMap) -> Result<Self> {
        let mut headers = BTreeMap::default();

        for (n, v) in header_map.iter() {
            headers.insert(
                n.to_string(),
                v.to_owned().to_str().or_throw(ctx)?.to_string(),
            );
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
        let headers: BTreeMap<String, String> = map
            .into_iter()
            .map(|(key, value)| (key.to_lowercase(), value.to_string()))
            .collect();
        Self { headers }
    }
}

impl<'js> IteratorDef<'js> for Headers {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        map_to_entries(&ctx, self.headers.clone())
    }
}
