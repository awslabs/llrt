// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::BTreeMap, rc::Rc};

use hyper::HeaderMap;
use llrt_utils::class::{CustomInspect, IteratorDef};
use rquickjs::{
    atom::PredefinedAtom, methods, prelude::Opt, Array, Coerced, Ctx, FromJs, Function, IntoJs,
    Null, Object, Result, Value,
};

use crate::utils::object::map_to_entries;

const HEADERS_KEY_COOKIE: &str = "cookie";
const HEADERS_KEY_SET_COOKIE: &str = "set-cookie";

type ImmutableString = Rc<str>;

#[derive(Clone, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
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
        let key: ImmutableString = key.to_lowercase().into();
        let str_key = key.as_ref();
        if str_key == HEADERS_KEY_SET_COOKIE {
            return self.headers.push((key, value.into()));
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
    }

    pub fn get<'js>(&self, ctx: Ctx<'js>, key: String) -> Result<Value<'js>> {
        let key: ImmutableString = key.to_lowercase().into();
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

    pub fn has(&self, key: String) -> bool {
        let key = key.to_lowercase().into();
        self.headers.iter().any(|(k, _)| k == &key)
    }

    pub fn set(&mut self, key: String, value: String) {
        let key: ImmutableString = key.to_lowercase().into();
        let value = value.into();

        if key.as_ref() == HEADERS_KEY_SET_COOKIE {
            self.headers.retain(|(k, _)| k != &key);
            self.headers.push((key, value));
        } else {
            match self.headers.iter_mut().find(|(k, _)| k == &key) {
                Some((_, existing_value)) => *existing_value = value,
                None => self.headers.push((key, value)),
            }
        }
    }

    pub fn delete(&mut self, key: String) {
        let key = key.to_lowercase().into();
        self.headers.retain(|(k, _)| k != &key);
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

    fn array_to_headers(array: Array<'_>) -> Result<Vec<(ImmutableString, ImmutableString)>> {
        let mut vec = Vec::new();
        for entry in array.into_iter().flatten() {
            if let Some(array_entry) = entry.as_array() {
                let key = array_entry.get::<String>(0)?.to_lowercase();
                let value = array_entry.get::<String>(1)?;
                vec.push((key.into(), value.into()));
            }
        }
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
