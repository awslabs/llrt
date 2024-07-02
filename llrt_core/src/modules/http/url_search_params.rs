// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::utils::class::IteratorDef;
use rquickjs::atom::PredefinedAtom;
use rquickjs::class::Trace;
use rquickjs::function::Opt;
use rquickjs::{
    Array, Class, Coerced, Ctx, Exception, FromJs, Function, IntoJs, Null, Object, Result, Symbol,
    Value,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use url::Url;

/// Represents `URLSearchParams` in the JavaScript context
///
/// <https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams>
///
/// # Examples
///
/// ```rust,ignore
/// // This is JavaScript
/// const params = new URLSearchParams();
/// params.set("foo", "bar");
/// ```
#[derive(Clone, Trace)]
#[rquickjs::class]
pub struct URLSearchParams {
    // URL and URLSearchParams work together to manipulate URLs, so using a
    // reference counter (Rc) allows them to have shared ownership of the
    // undering Url, and a RefCell allows interior mutability.
    #[qjs(skip_trace)]
    pub url: Rc<RefCell<Url>>,
}

// URLSearchParams is designed to operate directly on the underlying Url to
// avoid maintaining derived state that can get out of sync. When it's used
// independently, it still needs a valid URL (http://example.com), but this
// doesn't have any effect on using URLSearchParams with URL as the params are
// stringified when added to a URL.
//
// ```js
// const params = new URLSearchParams("foo=bar");
// const url = new URL("http://github.com");
// url.search = params; // This works as expected
// ```
#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> URLSearchParams {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_string() {
                let string: String = Coerced::from_js(&ctx, init)?.0;
                return Ok(Self::from_str(string));
            } else if init.is_array() {
                return Self::from_array(&ctx, init.into_array().unwrap());
            } else if init.is_object() {
                return Self::from_object(&ctx, init.into_object().unwrap());
            }
        }
        let url: Url = "http://example.com".parse().unwrap();

        Ok(URLSearchParams {
            url: Rc::new(RefCell::new(url)),
        })
    }

    //
    // Properties
    //

    #[qjs(get)]
    pub fn size(&self) -> usize {
        self.url.borrow().query_pairs().count()
    }

    //
    // Instance methods
    //

    pub fn append(&mut self, key: Coerced<String>, value: Coerced<String>) {
        self.url
            .borrow_mut()
            .query_pairs_mut()
            .append_pair(key.as_str(), value.as_str());
    }

    pub fn delete(&mut self, ctx: Ctx<'js>, key: Coerced<String>, value: Opt<Value<'js>>) {
        let key = key.0;
        let value = value.into_inner();

        let new_pairs: Vec<_> = self
            .url
            .borrow()
            .query_pairs()
            .filter(|(k, v)| {
                if let Some(value) = value.clone() {
                    if !value.is_undefined() {
                        let value: Result<Coerced<String>> = Coerced::from_js(&ctx, value);
                        if let Ok(value) = value {
                            return !(*k == key && *v == *value);
                        }
                    }
                }
                *k != key
            })
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        if !new_pairs.is_empty() {
            self.url
                .borrow_mut()
                .query_pairs_mut()
                .clear()
                .extend_pairs(new_pairs);
        } else {
            self.url.borrow_mut().set_query(None);
        }
    }

    pub fn entries(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    pub fn for_each(&self, callback: Function<'js>) -> Result<()> {
        self.url
            .borrow()
            .query_pairs()
            .into_owned()
            .try_for_each(|(k, v)| callback.call((v, k)))?;
        Ok(())
    }

    pub fn get(&mut self, ctx: Ctx<'js>, key: String) -> Result<Value<'js>> {
        match self
            .url
            .borrow()
            .query_pairs()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
        {
            Some(value) => value.into_js(&ctx),
            None => Null.into_js(&ctx),
        }
    }

    pub fn get_all(&mut self, key: String) -> Vec<String> {
        self.url
            .borrow()
            .query_pairs()
            .filter_map(|(k, v)| if k == key { Some(v.to_string()) } else { None })
            .collect()
    }

    pub fn has(&self, ctx: Ctx<'js>, key: Coerced<String>, value: Opt<Value<'js>>) -> bool {
        let key = key.0;
        let value = value.into_inner();
        self.url.borrow().query_pairs().any(|(k, v)| {
            if let Some(value) = value.clone() {
                if !value.is_undefined() {
                    let value: Result<Coerced<String>> = Coerced::from_js(&ctx, value);
                    if let Ok(value) = value {
                        return *k == key && *v == *value;
                    }
                }
            }

            *k == key
        })
    }

    pub fn keys(&mut self) -> Vec<String> {
        self.url
            .borrow()
            .query_pairs()
            .map(|(k, _)| k.to_string())
            .collect()
    }

    pub fn set(&mut self, key: Coerced<String>, value: Coerced<String>) {
        let key = key.0;
        let value = value.0;

        // Use a HashSet just to filter duplicates
        let mut uniques = HashSet::new();
        let mut new_query_pairs: Vec<(String, String)> = Vec::new();

        for (k, v) in self.url.borrow().query_pairs() {
            // Update the value for an existing key
            let value = if k == key {
                value.clone()
            } else {
                v.to_string()
            };

            let query_pair = (k.to_string(), value);
            if uniques.insert(query_pair.clone()) {
                new_query_pairs.push(query_pair);
            }
        }

        // Append a new key/value pair
        let query_pair = (key, value);
        if uniques.insert(query_pair.clone()) {
            new_query_pairs.push(query_pair);
        }

        self.url
            .borrow_mut()
            .query_pairs_mut()
            .clear()
            .extend_pairs(new_query_pairs);
    }

    pub fn sort(&mut self) {
        let mut new_pairs: Vec<(String, String)> =
            self.url.borrow().query_pairs().into_owned().collect();
        new_pairs.sort_by(|(a, _), (b, _)| a.cmp(b));

        if new_pairs.is_empty() {
            self.url.borrow_mut().set_query(None);
        } else {
            self.url
                .borrow_mut()
                .query_pairs_mut()
                .clear()
                .extend_pairs(new_pairs);
        }
    }

    pub fn to_string(&self) -> String {
        fn escape(value: &str) -> String {
            url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
        }

        self.url
            .borrow()
            .query_pairs()
            .map(|(key, value)| {
                [
                    escape(key.as_ref()),
                    "=".to_string(),
                    escape(value.as_ref()),
                ]
                .join("")
            })
            .collect::<Vec<String>>()
            .join("&")
    }

    pub fn values(&mut self) -> Vec<String> {
        self.url
            .borrow()
            .query_pairs()
            .map(|(_, v)| v.to_string())
            .collect()
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator(&self, ctx: Ctx<'js>) -> Result<Class<'js, URLSearchParamsIter>> {
        Class::instance(
            ctx,
            URLSearchParamsIter {
                index: 0,
                params: self.clone(),
            },
        )
    }
}

impl<'js> URLSearchParams {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(query: String) -> Self {
        let query = query.strip_prefix('?').unwrap_or(query.as_str());
        let url = "http://example.com"
            .parse::<Url>()
            .unwrap()
            .join(("?".to_string() + query).as_str())
            .unwrap();
        Self {
            url: Rc::new(RefCell::new(url)),
        }
    }

    pub fn from_url(url: &Rc<RefCell<Url>>) -> Self {
        Self {
            url: Rc::clone(url),
        }
    }

    pub fn from_array(ctx: &Ctx<'js>, array: Array) -> Result<Self> {
        let mut url: Url = "http://example.com".parse().unwrap();
        let query_pairs: Vec<(String, String)> = array
            .into_iter()
            .map(|value| {
                if let Ok(value) = value {
                    if let Some(pair) = value.as_array() {
                        if pair.len() == 2 {
                            if let Ok(key) = pair.get::<Coerced<String>>(0) {
                                if let Ok(value) = pair.get::<Coerced<String>>(1) {
                                    return Ok((key.to_string(), value.to_string()));
                                }
                            }
                        }
                    }
                };
                Err(Exception::throw_type(
                    ctx,
                    "Invalid tuple: Each query pair must be an iterable [name, value] tuple",
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .collect();

        url.query_pairs_mut().extend_pairs(query_pairs);

        Ok(Self {
            url: Rc::new(RefCell::new(url)),
        })
    }

    pub fn from_object(ctx: &Ctx<'js>, object: Object<'js>) -> Result<Self> {
        let iterator = Symbol::iterator(ctx.clone());
        if object.contains_key(iterator)? {
            let array_object: Object = ctx.globals().get(PredefinedAtom::Array)?;
            let array_from: Function = array_object.get(PredefinedAtom::From)?;
            let query_pairs: Array = array_from.call((object,))?;
            return Self::from_array(ctx, query_pairs);
        }

        let mut url: Url = "http://example.com".parse().unwrap();
        let query_pairs: Vec<(String, String)> = object
            .keys::<Value<'js>>()
            .map(|key| {
                let key = key?;
                let key_string: String = Coerced::from_js(ctx, key.clone())?.0;
                let value: String = object.get::<_, Coerced<String>>(key)?.0;
                Ok((key_string, value))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .collect();

        url.query_pairs_mut().extend_pairs(query_pairs);

        Ok(Self {
            url: Rc::new(RefCell::new(url)),
        })
    }
}

#[derive(Trace)]
#[rquickjs::class]
pub struct URLSearchParamsIter {
    params: URLSearchParams,
    index: u32,
}

#[rquickjs::methods]
impl<'js> URLSearchParamsIter {
    pub fn next(&mut self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        let value = (*self.params.url.borrow())
            .query_pairs()
            .nth(self.index as _)
            .map(|(k, v)| vec![k.to_string(), v.to_string()]);

        if let Some(value) = value {
            obj.set("done", false)?;
            obj.set("value", value)?;
        } else {
            obj.set("done", true)?;
        }

        self.index += 1;

        Ok(obj)
    }
}

impl<'js> IteratorDef<'js> for URLSearchParams {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        let array = Array::new(ctx.clone())?;
        for (idx, (key, value)) in self.url.borrow().query_pairs().into_owned().enumerate() {
            let entry = Array::new(ctx.clone())?;
            entry.set(0, key)?;
            entry.set(1, value)?;
            array.set(idx, entry)?;
        }
        Ok(array)
    }
}
