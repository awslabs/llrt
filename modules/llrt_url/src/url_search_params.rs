// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use llrt_utils::{
    class::IteratorDef,
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, Array, Class, Coerced, Ctx, Exception,
    FromJs, Function, IntoJs, Null, Object, Result, Symbol, Value,
};
use url::Url;

use super::convert_trailing_space;

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
#[derive(Clone, Trace, rquickjs::JsLifetime)]
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
                return Self::from_array(&ctx, unsafe { init.into_array().unwrap_unchecked() });
            } else if init.is_object() {
                return Self::from_object(&ctx, unsafe { init.into_object().unwrap_unchecked() });
            }
        }
        let url: Url = unsafe { "http://example.com".parse().unwrap_unchecked() };

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

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(URLSearchParams)
    }

    //
    // Instance methods
    //

    pub fn append(&mut self, key: Coerced<String>, value: Coerced<String>) {
        convert_trailing_space(&mut self.url.borrow_mut());

        self.url
            .borrow_mut()
            .query_pairs_mut()
            .append_pair(key.as_str(), value.as_str());
    }

    pub fn delete(&mut self, ctx: Ctx<'js>, key: Coerced<String>, value: Opt<Value<'js>>) {
        convert_trailing_space(&mut self.url.borrow_mut());

        let key = key.0;

        let value = get_coerced_string_value(&ctx, value);

        let new_pairs: Vec<_> = self
            .url
            .borrow()
            .query_pairs()
            .filter(|(k, v)| {
                if let Some(value) = value.as_ref() {
                    return !(*k == key && *v == *value);
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
        let value = get_coerced_string_value(&ctx, value);
        let key = key.0;
        self.url.borrow().query_pairs().any(|(k, v)| {
            if let Some(value) = value.as_ref() {
                return *k == key && *v == *value;
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
        convert_trailing_space(&mut self.url.borrow_mut());

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
        // The Url create doesn't properly encode query params for all edge
        // cases, so we need to construct the query string by percent-encoding
        // each key/value
        // TODO: This should probably be fixed in the Url crate
        let url = self.url.borrow();
        url.query_pairs().fold(
            String::with_capacity(url.query().map_or(0, |q| q.len())),
            |mut acc, (key, value)| {
                if !acc.is_empty() {
                    acc.push('&');
                }
                url::form_urlencoded::byte_serialize(key.as_bytes()).for_each(|b| acc.push_str(b));
                acc.push('=');
                url::form_urlencoded::byte_serialize(value.as_bytes())
                    .for_each(|b| acc.push_str(b));
                acc
            },
        )
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
        let query = if !query.starts_with('?') {
            ["?", &query].concat()
        } else {
            query
        };
        let url = unsafe {
            "http://example.com"
                .parse::<Url>()
                .unwrap_unchecked()
                .join(&query)
                .unwrap_unchecked()
        };
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
            let query_pairs: Array = BasePrimordials::get(ctx)?
                .function_array_from
                .call((object,))?;
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

#[derive(Trace, rquickjs::JsLifetime)]
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

fn get_coerced_string_value<'js>(ctx: &Ctx<'js>, value: Opt<Value<'js>>) -> Option<String> {
    if let Some(value) = value.0 {
        if !value.is_undefined() {
            if let Ok(value) = Coerced::<String>::from_js(ctx, value) {
                return Some(value.0);
            }
        }
    };
    None
}

#[cfg(test)]
mod tests {
    use llrt_test::test_sync_with;
    use rquickjs::{CatchResultExt, Class};

    use super::*;

    #[tokio::test]
    async fn test_basic() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.append('b', '4');
                params.append('c', 8);
                params.delete('a');
                params.delete('b', '2');
                params.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "b=4&c=8");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_iterate() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                let res = [];
                for (const [name, value] of params) {
                    res.push(`${name}=${value}`);
                }
                res.join('&')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=1&b=2&a=3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_iterate_entries() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                let res = [];
                for (const [name, value] of params.entries()) {
                    res.push(`${name}=${value}`);
                }
                res.join('&')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=1&b=2&a=3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_iterate_keys() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                let res = [];
                for (const name of params.keys()) {
                    res.push(name);
                }
                res.join('&')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a&b&a");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_iterate_values() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                let res = [];
                for (const name of params.values()) {
                    res.push(name);
                }
                res.join('&')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "1&2&3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_new_string() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams('a=1&b=2&a=3');
                params.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=1&b=2&a=3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_new_string_url() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams('https://google.com?a=1&b=2&a=3');
                params.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "https%3A%2F%2Fgoogle.com%3Fa=1&b=2&a=3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_new_object() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams({'a': 1, 'b': 2});
                params.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=1&b=2");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_new_array() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams([['a', 1], ['b', 2], ['a', 3]]);
                params.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=1&b=2&a=3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_new_iterator() {
        test_sync_with(|ctx| {
            BasePrimordials::init(&ctx)?;
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                const params2 = new URLSearchParams(params.entries());
                params2.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=1&b=2&a=3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_size() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<usize, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.size
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, 3);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_set() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.set('a', '4');
                params.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=4&b=2");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_get() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.get('a')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "1");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_get_missing() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<bool, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.get('c') === null
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert!(result);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_get_all() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.getAll('a').join('&')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "1&3");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_get_all_missing() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.getAll('c').join('&')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_has() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<bool, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.has('b')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert!(result);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_has_value() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<bool, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.has('b', 5)
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert!(!result);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_has_not() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<bool, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '1');
                params.append('b', '2');
                params.append('a', '3');
                params.has('c')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert!(!result);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_sort() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '3');
                params.append('b', '2');
                params.append('a', '1');
                params.sort();
                params.toString()
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=3&a=1&b=2");
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_for_each() {
        test_sync_with(|ctx| {
            Class::<URLSearchParams>::define(&ctx.globals()).unwrap();
            let result = ctx
                .eval::<String, _>(
                    r#"
                const params = new URLSearchParams();
                params.append('a', '3');
                params.append('b', '2');
                params.append('a', '1');
                let res = [];
                params.forEach((value, name) => {
                    res.push(`${name}=${value}`);
                });
                res.join('&')
            "#,
                )
                .catch(&ctx)
                .unwrap();
            assert_eq!(result, "a=3&b=2&a=1");
            Ok(())
        })
        .await
    }
}
