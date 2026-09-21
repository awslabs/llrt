// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc,
};

use llrt_utils::{
    bytes::{encode_wtf8, flush_wtf8, get_wtf8_string_bytes},
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    atom::PredefinedAtom, prelude::This, Ctx, Function, Object, Result, TypedArray, Value,
};

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct TextEncoderStream<'js> {
    readable: Value<'js>,
    writable: Value<'js>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> TextEncoderStream<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>) -> Result<Self> {
        let pending = Arc::new(AtomicU16::new(0));
        let transform_pending = pending.clone();
        let transform = Function::new(ctx.clone(), move |ctx, chunk, controller| {
            transform_chunk(ctx, &transform_pending, chunk, controller)
        })?;
        let flush_pending = pending;
        let flush = Function::new(ctx.clone(), move |ctx, controller| {
            flush_stream(ctx, &flush_pending, controller)
        })?;
        let transformer = Object::new(ctx.clone())?;
        transformer.set("transform", transform)?;
        transformer.set("flush", flush)?;

        let stream = llrt_stream_web::create_transform_stream(&ctx, transformer)?;

        Ok(Self {
            readable: stream.get("readable")?,
            writable: stream.get("writable")?,
        })
    }

    #[qjs(get)]
    fn encoding(&self) -> &str {
        "utf-8"
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
        stringify!(TextEncoderStream)
    }
}

fn transform_chunk<'js>(
    ctx: Ctx<'js>,
    pending: &AtomicU16,
    chunk: Value<'js>,
    controller: Object<'js>,
) -> Result<()> {
    let string = if chunk.is_string() {
        chunk
    } else {
        let string = BasePrimordials::get(&ctx)?.constructor_string.clone();
        string.call((chunk,))?
    };
    let bytes = get_wtf8_string_bytes(string)?;
    let mut output = Vec::new();
    let mut pending_value = match pending.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };
    encode_wtf8(&bytes, &mut pending_value, &mut output);
    pending.store(pending_value.unwrap_or(0), Ordering::Relaxed);
    if !output.is_empty() {
        let encoded = TypedArray::new(ctx.clone(), output)?;
        let enqueue: Function = controller.get("enqueue")?;
        enqueue.call::<_, ()>((This(controller), encoded))?;
    }
    Ok(())
}

fn flush_stream<'js>(ctx: Ctx<'js>, pending: &AtomicU16, controller: Object<'js>) -> Result<()> {
    let mut pending_value = match pending.load(Ordering::Relaxed) {
        0 => None,
        value => Some(value),
    };
    let mut output = Vec::new();
    flush_wtf8(&mut pending_value, &mut output);
    pending.store(pending_value.unwrap_or(0), Ordering::Relaxed);
    if !output.is_empty() {
        let encoded = TypedArray::new(ctx.clone(), output)?;
        let enqueue: Function = controller.get("enqueue")?;
        enqueue.call::<_, ()>((This(controller), encoded))?;
    }
    Ok(())
}
