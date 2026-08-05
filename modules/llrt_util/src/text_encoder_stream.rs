// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::primordials::Primordial;
use rquickjs::{
    atom::PredefinedAtom, convert::Coerced, prelude::This, Ctx, Function, Object, Result,
    TypedArray, Value,
};

use crate::UtilPrimordials;

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

        let transform_stream_ctor = &UtilPrimordials::get(&ctx)?.constructor_transform_stream;
        let stream: Object = transform_stream_ctor.construct((transformer,))?;

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
