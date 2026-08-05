// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(test)]
mod tests;
pub mod text_decoder;
pub mod text_decoder_stream;
pub mod text_encoder;
pub mod text_encoder_stream;

use llrt_logging::format_plain;
use llrt_utils::{
    module::{export_default, ModuleInfo},
    primordials::Primordial,
};
use rquickjs::{
    function::{Constructor, Func},
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Function, JsLifetime, Object, Result,
};
use text_decoder::TextDecoder;
use text_decoder_stream::TextDecoderStream;
use text_encoder::TextEncoder;
use text_encoder_stream::TextEncoderStream;

#[derive(JsLifetime)]
pub(crate) struct UtilPrimordials<'js> {
    pub(crate) constructor_transform_stream: Constructor<'js>,
}

impl<'js> Primordial<'js> for UtilPrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self> {
        Ok(Self {
            constructor_transform_stream: ctx.globals().get("TransformStream")?,
        })
    }
}

fn inherits<'js>(ctor: Function<'js>, super_ctor: Function<'js>) -> Result<()> {
    let super_proto: Object<'js> = super_ctor.get("prototype")?;
    let proto: Object<'js> = ctor.get("prototype")?;
    proto.set_prototype(Some(&super_proto))?;
    ctor.set("super_", super_ctor)?;
    Ok(())
}

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextDecoderStream))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare(stringify!(TextEncoderStream))?;
        declare.declare(stringify!(format))?;
        declare.declare(stringify!(inherits))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;
            let encoder_stream: Function = globals.get(stringify!(TextEncoderStream))?;
            let decoder_stream: Function = globals.get(stringify!(TextDecoderStream))?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;
            default.set(stringify!(TextEncoderStream), encoder_stream)?;
            default.set(stringify!(TextDecoderStream), decoder_stream)?;
            default.set(
                "format",
                Func::from(|ctx, args| format_plain(ctx, true, args)),
            )?;
            default.set("inherits", Func::from(inherits))?;

            Ok(())
        })
    }
}

impl From<UtilModule> for ModuleInfo<UtilModule> {
    fn from(val: UtilModule) -> Self {
        ModuleInfo {
            name: "util",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<TextEncoder>::define(&globals)?;
    Class::<TextDecoder>::define(&globals)?;

    UtilPrimordials::init(ctx)?;

    Class::<TextEncoderStream>::define(&globals)?;
    Class::<TextDecoderStream>::define(&globals)?;

    Ok(())
}
