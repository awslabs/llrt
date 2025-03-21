// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod text_decoder;
pub mod text_encoder;

use llrt_logging::format_plain;
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    function::Func,
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Function, Result,
};
use text_decoder::TextDecoder;
use text_encoder::TextEncoder;

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare(stringify!(format))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;
            default.set(
                "format",
                Func::from(|ctx, args| format_plain(ctx, true, args)),
            )?;

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

    Ok(())
}
