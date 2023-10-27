use std::collections::BTreeMap;

use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Ctx, FromJs, Function, Object, Result, Symbol, Value,
};

use crate::util::{array_to_btree_map, IteratorDef};

#[derive(Clone, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct URLSearchParams {
    #[qjs(skip_trace)]
    params: BTreeMap<String, Vec<String>>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl URLSearchParams {
    #[qjs(constructor)]
    pub fn new<'js>(ctx: Ctx<'js>, init: Opt<Value<'js>>) -> Result<Self> {
        if let Some(init) = init.into_inner() {
            if init.is_string() {
                let string: String = init.get()?;
                return Ok(Self::from_str(&string));
            } else if init.is_array() {
                let array = init.into_array().unwrap();
                let map = array_to_btree_map(&ctx, array)?;
                let params = to_params(map);
                return Ok(Self { params });
            } else if init.is_object() {
                let obj = init.as_object().unwrap();

                let iterator = Symbol::iterator(ctx.clone());

                if obj.contains_key(iterator)? {
                    let array_object: Object = ctx.globals().get(PredefinedAtom::Array)?;
                    let array_from: Function = array_object.get(PredefinedAtom::From)?;
                    let value: Value = array_from.call((init,))?;
                    let array = value.into_array().unwrap();
                    let map = array_to_btree_map(&ctx, array)?;
                    let params = to_params(map);
                    return Ok(Self { params });
                }

                let map = BTreeMap::from_js(&ctx, init.to_owned())?;
                let params = to_params(map);
                return Ok(Self { params });
            }
        }

        Ok(URLSearchParams {
            params: BTreeMap::default(),
        })
    }

    pub fn append(&mut self, key: String, value: String) {
        self.params
            .entry(key)
            .and_modify(|vec| vec.push(value.clone()))
            .or_insert_with(|| vec![value.clone()]);
    }

    pub fn get(&mut self, key: String) -> Option<String> {
        self.params.get(&key).and_then(|v| v.first()).cloned()
    }

    pub fn get_all(&mut self, key: String) -> Vec<String> {
        match self.params.get(&key) {
            Some(values) => values.to_owned(),
            None => vec![],
        }
    }

    pub fn has(&mut self, key: String) -> bool {
        self.params.contains_key(&key)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.params.insert(key.to_lowercase(), vec![value]);
    }

    pub fn delete(&mut self, key: String) {
        self.params.remove(&key.to_lowercase());
    }

    pub fn to_string(&self) -> String {
        let mut string = String::with_capacity(10);
        let length = self.params.len();
        if length == 0 {
            return String::from("");
        }
        for (i, (key, values)) in self.params.iter().enumerate() {
            let values_length = values.len();
            for (j, value) in values.iter().enumerate() {
                string.push_str(&key.replace(' ', "+"));
                string.push('=');
                string.push_str(&value.replace(' ', "+"));

                if j < values_length - 1 {
                    string.push('&');
                }
            }
            if i < length - 1 {
                string.push('&');
            }
        }
        string
    }

    pub fn values(&mut self) -> Vec<String> {
        self.params
            .values()
            .flatten()
            .cloned()
            .collect::<Vec<String>>()
    }

    pub fn entries<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }
}

impl URLSearchParams {
    pub fn from_str(query: &str) -> Self {
        let params = parse_query_string(query);
        Self { params }
    }
}

fn to_params(map: BTreeMap<String, String>) -> BTreeMap<String, Vec<String>> {
    let mut params = BTreeMap::new();
    for (key, value) in map {
        params.insert(key, vec![value]);
    }
    params
}

fn parse_query_string(query_string: &str) -> BTreeMap<String, Vec<String>> {
    let mut query_pairs = BTreeMap::new();
    let query = match query_string.strip_prefix('?') {
        Some(q) => q,
        None => query_string,
    };
    if query.is_empty() {
        return query_pairs;
    }
    for pair in query.split('&') {
        let mut key_value = pair.split('=');
        if let Some(key) = key_value.next() {
            let values = query_pairs.entry(key.to_string()).or_insert_with(Vec::new);
            if let Some(value) = key_value.next() {
                values.push(value.to_string());
            } else {
                values.push("".to_string());
            }
        }
    }
    query_pairs
}

impl<'js> IteratorDef<'js> for URLSearchParams {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        let array = Array::new(ctx.clone())?;
        let mut idx = 0;
        for (key, values) in &self.params {
            for value in values {
                let entry = Array::new(ctx.clone())?;
                entry.set(0, key)?;
                entry.set(1, value)?;
                array.set(idx, entry)?;
                idx += 1;
            }
        }
        Ok(array)
    }
}
