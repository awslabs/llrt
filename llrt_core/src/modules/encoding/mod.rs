// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod encoder;
pub mod text_decoder;
pub mod text_encoder;

use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Result, Value,
};

use crate::{
    module_builder::ModuleInfo,
    modules::module::export_default,
    utils::{
        object::{bytes_to_typed_array, get_bytes},
        result::ResultExt,
    },
};

use self::encoder::{bytes_from_b64, bytes_from_hex, bytes_to_b64_string, bytes_to_hex_string};
use self::text_decoder::TextDecoder;
use self::text_encoder::TextEncoder;

pub struct HexModule;

impl HexModule {
    pub fn encode<'js>(ctx: Ctx<'js>, buffer: Value<'js>) -> Result<String> {
        let bytes = get_bytes(&ctx, buffer)?;
        Ok(bytes_to_hex_string(&bytes))
    }

    pub fn decode(ctx: Ctx, encoded: String) -> Result<Value> {
        let bytes = bytes_from_hex(encoded.as_bytes())
            .or_throw_msg(&ctx, "Cannot decode unrecognized sequence")?;

        bytes_to_typed_array(ctx, &bytes)
    }
}

impl ModuleDef for HexModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(encode))?;
        declare.declare(stringify!(decode))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set(stringify!(encode), Func::from(Self::encode))?;
            default.set(stringify!(decode), Func::from(Self::decode))?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<HexModule> for ModuleInfo<HexModule> {
    fn from(val: HexModule) -> Self {
        ModuleInfo {
            name: "hex",
            module: val,
        }
    }
}

pub fn atob(ctx: Ctx<'_>, encoded_value: String) -> Result<String> {
    let vec = bytes_from_b64(encoded_value.as_bytes()).or_throw(&ctx)?;
    Ok(unsafe { String::from_utf8_unchecked(vec) })
}

pub fn btoa(value: String) -> String {
    bytes_to_b64_string(value.as_bytes())
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    globals.set("atob", Func::from(atob))?;
    globals.set("btoa", Func::from(btoa))?;

    Class::<TextEncoder>::define(&globals)?;
    Class::<TextDecoder>::define(&globals)?;

    Ok(())
}
