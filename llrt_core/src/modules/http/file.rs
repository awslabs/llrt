// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::time::UNIX_EPOCH;

use rquickjs::{
    class::Trace, function::Opt, ArrayBuffer, Coerced, Ctx, Object,
    Result, Value,
};

use super::blob::Blob;

#[rquickjs::class]
#[derive(Trace, Clone)]
pub struct File {
    #[qjs(skip_trace)]
    blob: Blob,
    filename: String,
    last_modified: i64,
}

#[rquickjs::methods]
impl File {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, filebits: Value<'js>, filename: String, options: Opt<Object<'js>>) -> Result<Self> {
        let mut last_modified = UNIX_EPOCH.elapsed().unwrap().as_millis() as i64;

        if let Some(ref opts) = options.0 {
            if let Some(x) = opts.get::<_, Option<Coerced<i64>>>("lastModified")? {
                println!("lastModified: {:?}", x.0);
                last_modified = x.0;
            }
        }

        let blob = Blob::new(ctx, Opt::from(Some(filebits)), options)?;

        Ok(Self { blob, filename, last_modified})
    }

    #[qjs(get)]
    pub fn size(&self) -> usize {
        self.blob.size()
    }

    #[qjs(get)]
    pub fn name(&self) -> String {
        self.filename.clone()
    }

    #[qjs(get, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.blob.mime_type()
    }

    #[qjs(get, rename =  "lastModified")]
    pub fn last_modified(&self) -> i64 {
        self.last_modified
    }

    pub fn slice(&self, start: Opt<isize>, end: Opt<isize>, content_type: Opt<String>) -> Blob {
        self.blob.slice(start, end, content_type)
    }

    pub async fn text(&mut self) -> String {
        self.blob.text().await
    }

    #[qjs(rename = "arrayBuffer")]
    pub async fn array_buffer<'js>(&self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        self.blob.array_buffer(ctx).await
    }
}
