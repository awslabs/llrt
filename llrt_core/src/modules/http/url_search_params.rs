// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Coerced, Ctx, Function, IntoJs, Null, Object,
    Result, Symbol, Value,
};

use crate::utils::class::IteratorDef;

type Params = Vec<(String, String)>;

#[derive(Clone, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct URLSearchParams {
    #[qjs(skip_trace)]
    params: Params,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl URLSearchParams {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_string() {
                let string: String = init.into_string().unwrap().to_string().unwrap();
                return Ok(Self::from_str(&string));
            } else if init.is_array() {
                let array = init.into_array().unwrap();
                return Ok(Self::from_array(array));
            } else if init.is_object() {
                let obj = init.into_object().unwrap();

                let iterator = Symbol::iterator(ctx.clone());

                if obj.contains_key(iterator)? {
                    let array_object: Object = ctx.globals().get(PredefinedAtom::Array)?;
                    let array_from: Function = array_object.get(PredefinedAtom::From)?;
                    let value: Value = array_from.call((obj,))?;
                    let array = value.into_array().unwrap();
                    return Ok(Self::from_array(array));
                }

                let keys = obj.keys::<String>();
                let key_len = keys.len();

                let mut params = Vec::with_capacity(key_len);

                for key in keys {
                    let key = key?;
                    let val = obj.get::<_, Coerced<String>>(&key)?;
                    params.push((key, val.to_string()))
                }
                return Ok(Self { params });
            }
        }

        Ok(URLSearchParams { params: Vec::new() })
    }

    #[qjs(get)]
    pub fn size(&mut self) -> usize {
        self.params.len()
    }

    pub fn append(&mut self, key: String, value: String) {
        self.params.push((key, value));
    }

    pub fn get<'js>(&mut self, ctx: Ctx<'js>, key: String) -> Result<Value<'js>> {
        self.params
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.into_js(&ctx))
            .unwrap_or_else(|| Null.into_js(&ctx))
    }

    pub fn get_all(&mut self, key: String) -> Vec<String> {
        self.params
            .iter()
            .filter_map(|(k, v)| if k == &key { Some(v.clone()) } else { None })
            .collect()
    }

    pub fn sort(&mut self) {
        self.params.sort_by(|(a, _), (b, _)| a.cmp(b));
    }

    pub fn has(&mut self, key: String) -> bool {
        self.params.iter().any(|(k, _)| k == &key.to_lowercase())
    }

    #[allow(unused_assignments)] //clippy bug?
    pub fn set(&mut self, key: String, value: String) {
        let mut modified = false;
        let mut same = false;
        self.params.retain_mut(move |(k, v)| {
            same = k == &key;
            if !modified && same {
                modified = true;
                v.clone_from(&value);
                return modified;
            }

            !same
        });
    }

    pub fn delete(&mut self, key: String) {
        if let Some(pos) = self
            .params
            .iter()
            .position(|(k, _)| k == &key.to_lowercase())
        {
            self.params.remove(pos);
        }
    }

    pub fn to_string(&self) -> String {
        let length = self.params.len();
        if length == 0 {
            return String::from("");
        }

        fn escape(value: &str) -> String {
            url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
        }

        let mut string = String::with_capacity(self.params.len() * 2);
        for (i, (key, value)) in self.params.iter().enumerate() {
            string.push_str(&escape(key));
            if !value.is_empty() {
                string.push('=');
                string.push_str(&escape(value));
            }

            if i < length - 1 {
                string.push('&');
            }
        }
        string
    }

    pub fn keys(&mut self) -> Vec<String> {
        self.params.iter().map(|(k, _)| k.clone()).collect()
    }

    pub fn values(&mut self) -> Vec<String> {
        self.params.iter().map(|(_, v)| v.clone()).collect()
    }

    pub fn entries<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    pub fn for_each(&self, callback: Function<'_>) -> Result<()> {
        for param in self.params.iter() {
            callback.call((param.1.clone(), param.0.clone()))?
        }
        Ok(())
    }
}

impl URLSearchParams {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(query: &str) -> Self {
        let params = Self::parse_query_string(query);
        Self { params }
    }

    fn from_array(array: Array) -> Self {
        let mut params: Params = Vec::with_capacity(array.len());

        for value in array.into_iter().flatten() {
            if let Some(entry) = value.as_array() {
                if let Ok(key) = entry.get::<Coerced<String>>(0) {
                    let key = key.to_string();
                    if let Ok(value) = entry.get::<Coerced<String>>(1) {
                        params.push((key, value.to_string()));
                    }
                }
            }
        }
        Self { params }
    }

    fn parse_query_string(query_string: &str) -> Params {
        let query = match query_string.strip_prefix('?') {
            Some(q) => q,
            None => query_string,
        };

        let params = url::form_urlencoded::parse(query.as_bytes())
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        params
    }
}

impl<'js> IteratorDef<'js> for URLSearchParams {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        let array = Array::new(ctx.clone())?;
        for (idx, (key, value)) in self.params.iter().enumerate() {
            let entry = Array::new(ctx.clone())?;
            entry.set(0, key)?;
            entry.set(1, value)?;
            array.set(idx, entry)?;
        }
        Ok(array)
    }
}
