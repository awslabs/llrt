// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, Mutex};

use llrt_utils::bytes::ObjectBytes;
use rquickjs::{
    atom::PredefinedAtom, function::Opt, prelude::This, Ctx, Function, Object, Result, Value,
};

use crate::text_decoder::TextDecoder;

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct TextDecoderStream<'js> {
    #[qjs(skip_trace)]
    encoding: String,
    #[qjs(skip_trace)]
    fatal: bool,
    #[qjs(skip_trace)]
    ignore_bom: bool,
    readable: Value<'js>,
    writable: Value<'js>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> TextDecoderStream<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, label: Opt<String>, options: Opt<Object<'js>>) -> Result<Self> {
        let decoder = TextDecoder::new(ctx.clone(), label, options)?;
        let encoding = decoder.encoding().to_owned();
        let fatal = decoder.fatal();
        let ignore_bom = decoder.ignore_bom();
        let decoder = Arc::new(Mutex::new(decoder));

        let transform_decoder = decoder.clone();
        let transform = Function::new(ctx.clone(), move |ctx, chunk, controller| {
            transform(&transform_decoder, ctx, chunk, controller)
        })?;

        let flush_decoder = decoder.clone();
        let flush = Function::new(ctx.clone(), move |ctx, controller| {
            flush(&flush_decoder, ctx, controller)
        })?;

        let transformer = Object::new(ctx.clone())?;
        transformer.set("transform", transform)?;
        transformer.set("flush", flush)?;

        let stream = llrt_stream_web::create_transform_stream(&ctx, transformer)?;

        Ok(Self {
            encoding,
            fatal,
            ignore_bom,
            readable: stream.get("readable")?,
            writable: stream.get("writable")?,
        })
    }

    #[qjs(get)]
    fn encoding(&self) -> &str {
        &self.encoding
    }

    #[qjs(get, rename = "fatal")]
    fn fatal(&self) -> bool {
        self.fatal
    }

    #[qjs(get, rename = "ignoreBOM")]
    fn ignore_bom(&self) -> bool {
        self.ignore_bom
    }

    #[qjs(get)]
    fn readable(&self) -> Value<'js> {
        self.readable.clone()
    }

    #[qjs(get)]
    fn writable(&self) -> Value<'js> {
        self.writable.clone()
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(TextDecoderStream)
    }
}

fn transform<'js>(
    decoder: &Mutex<TextDecoder>,
    ctx: Ctx<'js>,
    chunk: Value<'js>,
    controller: Object<'js>,
) -> Result<()> {
    let bytes = ObjectBytes::from(&ctx, &chunk)?;
    let opts = Object::new(ctx.clone())?;
    opts.set("stream", true)?;
    let text =
        decoder
            .lock()
            .unwrap()
            .decode(ctx, Opt(Some(bytes)), Opt(Some(opts.into_value())))?;
    if !text.is_empty() {
        let enqueue: Function = controller.get("enqueue")?;
        enqueue.call::<_, ()>((This(controller), text))?;
    }
    Ok(())
}

fn flush<'js>(decoder: &Mutex<TextDecoder>, ctx: Ctx<'js>, controller: Object<'js>) -> Result<()> {
    let text = decoder.lock().unwrap().decode(ctx, Opt(None), Opt(None))?;
    if !text.is_empty() {
        let enqueue: Function = controller.get("enqueue")?;
        enqueue.call::<_, ()>((This(controller), text))?;
    }
    Ok(())
}
