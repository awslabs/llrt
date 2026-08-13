// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom, convert::Coerced, prelude::This, Ctx, Function, Object, Result,
    TypedArray, Value,
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
        let transform = Function::new(ctx.clone(), transform)?;
        let transformer = Object::new(ctx.clone())?;
        transformer.set("transform", transform)?;

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

fn transform<'js>(ctx: Ctx<'js>, chunk: Coerced<String>, controller: Object<'js>) -> Result<()> {
    let bytes = TypedArray::new(ctx.clone(), chunk.0.as_bytes())?;
    let enqueue: Function = controller.get("enqueue")?;
    enqueue.call::<_, ()>((This(controller), bytes))
}
