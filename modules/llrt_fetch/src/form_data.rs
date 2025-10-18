// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use llrt_buffer::{Blob, File};
use llrt_utils::{class::IteratorDef, object::map_to_entries, result::ResultExt};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, prelude::Opt, Array, Class, Coerced, Ctx, Exception,
    FromJs, Function, IntoJs, JsLifetime, Result, Value,
};

#[derive(Clone)]
enum FormValue {
    Text(String),
    File(File),
    Blob(Blob),
}

impl<'js> IntoJs<'js> for FormValue {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self {
            FormValue::Text(s) => s.into_js(ctx),
            FormValue::File(f) => f.clone().into_js(ctx),
            FormValue::Blob(b) => b.clone().into_js(ctx),
        }
    }
}

#[derive(Clone, Trace, JsLifetime)]
#[rquickjs::class]
pub struct FormData {
    #[qjs(skip_trace)]
    entries: Arc<Mutex<Vec<(String, FormValue)>>>,
}

impl<'js> IteratorDef<'js> for FormData {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        let entries = self.entries.lock().or_throw(&ctx)?;
        map_to_entries(&ctx, entries.clone())
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> FormData {
    #[qjs(constructor)]
    pub fn new(_form: Opt<Value<'js>>, _submitter: Opt<Value<'js>>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn append(&self, ctx: Ctx<'js>, name: String, value: Value<'js>) -> Result<()> {
        let mut entries = self.entries.lock().or_throw(&ctx)?;

        if let Some(obj) = value.clone().into_object() {
            if let Some(f) = Class::<File>::from_object(&obj) {
                let file = f.borrow().to_owned();
                entries.push((name, FormValue::File(file)));
                return Ok(());
            }
            if let Some(b) = Class::<Blob>::from_object(&obj) {
                let blob = b.borrow().to_owned();
                entries.push((name, FormValue::Blob(blob)));
                return Ok(());
            }
        }

        if let Some(s) = value.as_string() {
            let str = s.to_string().or_throw(&ctx)?;
            entries.push((name, FormValue::Text(str)));
            return Ok(());
        }

        Err(Exception::throw_type(&ctx, "Invalid FormData value type"))
    }

    pub fn get(&self, ctx: Ctx<'js>, name: String) -> Result<Option<Value<'js>>> {
        let entries = self.entries.lock().or_throw(&ctx)?;
        for (k, v) in entries.iter().rev() {
            if *k == name {
                return Ok(v.clone().into_js(&ctx).ok());
            }
        }
        Ok(None)
    }

    pub fn get_all(&self, ctx: Ctx<'js>, name: String) -> Result<Vec<Value<'js>>> {
        let entries = self.entries.lock().or_throw(&ctx)?;

        Ok(entries
            .iter()
            .filter(|(k, _)| *k == name)
            .filter_map(|(_, v)| v.clone().into_js(&ctx).ok())
            .collect())
    }

    pub fn has(&self, ctx: Ctx<'js>, name: String) -> Result<bool> {
        let entries = self.entries.lock().or_throw(&ctx)?;

        Ok(entries.iter().any(|(n, _)| n == &name))
    }

    pub fn set(&self, ctx: Ctx<'js>, name: String, value: Value<'js>) -> Result<()> {
        let mut entries = self.entries.lock().or_throw(&ctx)?;
        entries.retain(|(k, _)| *k != name);

        if let Some(obj) = value.clone().into_object() {
            if let Some(f) = Class::<File>::from_object(&obj) {
                let file = f.borrow().to_owned();
                entries.push((name, FormValue::File(file)));
                return Ok(());
            }
            if let Some(b) = Class::<Blob>::from_object(&obj) {
                let blob = b.borrow().to_owned();
                entries.push((name, FormValue::Blob(blob)));
                return Ok(());
            }
        }

        if let Ok(s) = value.try_into_string() {
            let string = s.to_string().or_throw(&ctx)?;
            entries.push((name, FormValue::Text(string)));
            return Ok(());
        }

        Err(Exception::throw_type(&ctx, "Invalid FormData value type"))
    }

    pub fn delete(&self, ctx: Ctx<'js>, name: String) -> Result<()> {
        let mut entries = self.entries.lock().or_throw(&ctx)?;

        entries.retain(|(k, _)| *k != name);
        Ok(())
    }

    pub fn keys(&self, ctx: Ctx<'js>) -> Result<Vec<String>> {
        let entries = self.entries.lock().or_throw(&ctx)?;

        Ok(entries.iter().map(|(k, _)| k.clone()).collect())
    }

    pub fn values(&self, ctx: Ctx<'js>) -> Result<Vec<Value<'js>>> {
        let ctx2 = ctx.clone();
        let entries = self.entries.lock().or_throw(&ctx)?;

        Ok(entries
            .iter()
            .filter_map(|(_, v)| v.clone().into_js(&ctx2).ok())
            .collect())
    }

    pub fn entries(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    #[qjs(rename = PredefinedAtom::SymbolIterator)]
    pub fn iterator(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.js_iterator(ctx)
    }

    pub fn for_each(&self, ctx: Ctx<'js>, callback: Function<'js>) -> Result<()> {
        let entries = self.entries.lock().or_throw(&ctx)?;

        for (name, value) in entries.iter() {
            let val = value.clone().into_js(&ctx)?;
            () = callback.call((val, name.clone()))?;
        }

        Ok(())
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(FormData)
    }
}

impl FormData {
    #[allow(private_interfaces)]
    pub fn iter<'js>(&self, ctx: &Ctx<'js>) -> Result<impl Iterator<Item = (String, FormValue)>> {
        let entries = self.entries.lock().or_throw(ctx)?;
        let entries = entries.clone();

        Ok(entries.into_iter())
    }

    pub fn from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.is_object() {
            let form_data_obj = value.as_object().or_throw(ctx)?;
            return if form_data_obj.instance_of::<FormData>() {
                FormData::from_js(ctx, value)
            } else {
                let map: BTreeMap<String, Coerced<String>> = value.get().unwrap_or_default();
                return Ok(Self::from_map(map));
            };
        }

        Ok(Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn from_map(map: BTreeMap<String, Coerced<String>>) -> Self {
        let entries: Vec<(String, FormValue)> = map
            .into_iter()
            .map(|(name, value)| (name, FormValue::Text(value.to_string())))
            .collect();

        Self {
            entries: Arc::new(Mutex::new(entries)),
        }
    }
}
