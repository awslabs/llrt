// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, io::Write, rc::Rc};

use hyper::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use llrt_buffer::{Blob, File};
use llrt_utils::{class::IteratorDef, object::map_to_entries, result::ResultExt};
use rand::RngExt;
use rquickjs::{
    atom::PredefinedAtom, class::Trace, prelude::Opt, Array, Class, Ctx, Exception, Function,
    IntoJs, JsLifetime, Result, Value,
};

#[derive(Clone, Trace, JsLifetime)]
enum FormValue<'js> {
    Text(String),
    File(#[qjs(skip_trace)] File<'js>),
    Blob(#[qjs(skip_trace)] Blob<'js>),
}

impl<'js> IntoJs<'js> for FormValue<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self {
            FormValue::Text(s) => s.into_js(ctx),
            FormValue::File(f) => f.into_js(ctx),
            FormValue::Blob(b) => b.into_js(ctx),
        }
    }
}

#[derive(Clone, Default)]
struct FormEntries<'js>(Vec<(String, FormValue<'js>)>);

unsafe impl<'js> JsLifetime<'js> for FormEntries<'js> {
    type Changed<'to> = FormEntries<'to>;
}

impl<'js> std::ops::Deref for FormEntries<'js> {
    type Target = Vec<(String, FormValue<'js>)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js> std::ops::DerefMut for FormEntries<'js> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Trace, JsLifetime, Default)]
#[rquickjs::class]
pub struct FormData<'js> {
    #[qjs(skip_trace)]
    entries: Rc<RefCell<FormEntries<'js>>>,
}

impl<'js> IteratorDef<'js> for FormData<'js> {
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        let entries = self.entries.borrow();
        map_to_entries(&ctx, entries.0.clone())
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> FormData<'js> {
    #[qjs(constructor)]
    pub fn new(_form: Opt<Value<'js>>, _submitter: Opt<Value<'js>>) -> Self {
        Self::default()
    }

    pub fn append(&self, ctx: Ctx<'js>, name: String, value: Value<'js>) -> Result<()> {
        let mut entries = self.entries.borrow_mut();

        if let Some(obj) = value.clone().into_object() {
            if let Some(f) = Class::<File>::from_object(&obj) {
                let file = f.borrow().clone();
                entries.push((name, FormValue::File(file)));
                return Ok(());
            }
            if let Some(b) = Class::<Blob>::from_object(&obj) {
                let blob = b.borrow().clone();
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
        let entries = self.entries.borrow();
        for (k, v) in entries.iter().rev() {
            if *k == name {
                return Ok(v.clone().into_js(&ctx).ok());
            }
        }
        Ok(None)
    }

    pub fn get_all(&self, ctx: Ctx<'js>, name: String) -> Result<Vec<Value<'js>>> {
        let entries = self.entries.borrow();
        Ok(entries
            .iter()
            .filter(|(k, _)| *k == name)
            .filter_map(|(_, v)| v.clone().into_js(&ctx).ok())
            .collect())
    }

    pub fn has(&self, _ctx: Ctx<'js>, name: String) -> Result<bool> {
        let entries = self.entries.borrow();
        Ok(entries.iter().any(|(n, _)| n == &name))
    }

    pub fn set(&self, ctx: Ctx<'js>, name: String, value: Value<'js>) -> Result<()> {
        let mut entries = self.entries.borrow_mut();
        entries.retain(|(k, _)| *k != name);

        if let Some(obj) = value.clone().into_object() {
            if let Some(f) = Class::<File>::from_object(&obj) {
                let file = f.borrow().clone();
                entries.push((name, FormValue::File(file)));
                return Ok(());
            }
            if let Some(b) = Class::<Blob>::from_object(&obj) {
                let blob = b.borrow().clone();
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

    pub fn delete(&self, _ctx: Ctx<'js>, name: String) -> Result<()> {
        self.entries.borrow_mut().retain(|(k, _)| *k != name);
        Ok(())
    }

    pub fn keys(&self, _ctx: Ctx<'js>) -> Result<Vec<String>> {
        Ok(self
            .entries
            .borrow()
            .iter()
            .map(|(k, _)| k.clone())
            .collect())
    }

    pub fn values(&self, ctx: Ctx<'js>) -> Result<Vec<Value<'js>>> {
        let entries = self.entries.borrow();
        Ok(entries
            .iter()
            .filter_map(|(_, v)| v.clone().into_js(&ctx).ok())
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
        let entries = self.entries.borrow().0.clone();
        for (name, value) in entries.into_iter() {
            let val = value.into_js(&ctx)?;
            () = callback.call((val, name))?;
        }
        Ok(())
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(FormData)
    }
}

impl<'js> FormData<'js> {
    #[allow(private_interfaces)]
    pub fn iter(&self) -> Vec<(String, FormValue<'js>)> {
        self.entries.borrow().0.clone()
    }

    pub fn from_multipart_bytes(
        ctx: &Ctx<'js>,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<Self> {
        if bytes.is_empty() {
            return Ok(Self::default());
        }

        let Some(boundary) = extract_boundary(content_type) else {
            return Ok(Self::default());
        };
        let boundary_marker = ["--", &boundary].concat().into_bytes();

        let mut entries = Vec::new();
        let parts = bytes.split(|b| *b == b'\n').collect::<Vec<_>>();

        let mut current_headers = Vec::new();
        let mut current_data = Vec::new();
        let mut in_headers = false;

        let mut name: Option<String> = None;
        let mut filename: Option<String> = None;
        let mut mime_type: Option<String> = None;

        for line in parts {
            if line.starts_with(&boundary_marker) {
                if !current_data.is_empty() && name.is_some() {
                    let data = std::mem::take(&mut current_data);
                    if let Some(filename) = filename.take() {
                        let file = File::from_bytes(ctx, data, filename, mime_type)?;
                        entries.push((name.take().or_throw(ctx)?, FormValue::File(file)));
                    } else {
                        let text = String::from_utf8_lossy(&data).into_owned();
                        entries.push((name.take().or_throw(ctx)?, FormValue::Text(text)));
                    }
                }
                current_headers.clear();
                current_data.clear();
                name = None;
                filename = None;
                mime_type = None;
                in_headers = true;
                continue;
            }

            if in_headers {
                let line_str = String::from_utf8_lossy(line);
                if line_str.trim().is_empty() {
                    in_headers = false;
                } else {
                    current_headers.push(line_str.to_string());
                    if starts_with_ignore_case(&line_str, CONTENT_DISPOSITION.as_str()) {
                        for seg in line_str.split(';') {
                            let seg = seg.trim();
                            if let Some(n) = seg.strip_prefix("name=") {
                                name = Some(n.trim_matches('"').into());
                            } else if let Some(f) = seg.strip_prefix("filename=") {
                                filename = Some(f.trim_matches('"').into());
                            }
                        }
                    } else if starts_with_ignore_case(&line_str, CONTENT_TYPE.as_str()) {
                        if let Some(ct) = line_str.split(':').nth(1) {
                            mime_type = Some(ct.trim().into());
                        }
                    }
                }
            } else {
                current_data.extend_from_slice(line);
                current_data.push(b'\n');
            }
        }

        Ok(Self {
            entries: Rc::new(RefCell::new(FormEntries(entries))),
        })
    }

    pub fn to_multipart_bytes(&self, _ctx: &Ctx<'js>) -> Result<(Vec<u8>, String)> {
        let boundary = generate_boundary();
        let mut body = Vec::new();
        let entries = self.entries.borrow();
        if entries.is_empty() {
            // Empty FormData serialises to an empty body in browsers.
            return Ok((body, boundary));
        }

        for (name, value) in entries.iter() {
            match value {
                FormValue::Text(text) => {
                    write!(
                        body,
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{text}\r\n"
                    )?;
                },
                FormValue::File(file) => {
                    let filename = file.name().clone();
                    let content_type = file.mime_type().clone();
                    let blob = file.get_blob();
                    let bytes = blob.as_bytes();
                    write!(
                        body,
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
                    )?;
                    body.extend_from_slice(bytes);
                    body.extend_from_slice(b"\r\n");
                },
                FormValue::Blob(blob) => {
                    let bytes = blob.as_bytes();
                    let content_type = blob.mime_type();
                    write!(
                        body,
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; filename=\"blob\"\r\nContent-Type: {content_type}\r\n\r\n"
                    )?;
                    body.extend_from_slice(bytes);
                    body.extend_from_slice(b"\r\n");
                },
            }
        }

        write!(body, "--{boundary}--\r\n")?;

        Ok((body, boundary))
    }
}

fn extract_boundary(content_type: &str) -> Option<String> {
    content_type.split(';').find_map(|part| {
        let part = part.trim();
        part.find("boundary=").map(|idx| {
            part[(idx + "boundary=".len())..]
                .trim()
                .trim_matches('"')
                .into()
        })
    })
}

fn generate_boundary() -> String {
    let rand_string: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();

    ["----WebKitFormBoundary", &rand_string].concat()
}

fn starts_with_ignore_case(haystack: &str, needle: &str) -> bool {
    haystack.len() >= needle.len()
        && haystack.as_bytes()[..needle.len()].eq_ignore_ascii_case(needle.as_bytes())
}
