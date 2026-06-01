// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result, String, TypedArray, Value,
};
use std::string::String as StdString;

use crate::libs::{
    encoding::{bytes_from_b64_strict, bytes_from_hex, bytes_to_b64_string, bytes_to_hex_string},
    utils::{
        bytes::ObjectBytes,
        module::{export_default, ModuleInfo},
        result::ResultExt,
    },
};

pub struct LlrtCodecModule;

impl LlrtCodecModule {
    pub fn decode_from_base64<'js>(ctx: Ctx<'js>, encoded: String<'js>) -> Result<Value<'js>> {
        let encoded = encoded.to_cstring()?;
        let bytes = bytes_from_b64_strict(encoded.as_str().as_bytes())
            .or_throw_msg(&ctx, "Cannot decode unrecognized sequence")?;

        Ok(TypedArray::new(ctx, bytes)?.into_value())
    }

    pub fn encode_to_base64<'js>(ctx: Ctx<'js>, bytes: ObjectBytes<'js>) -> Result<StdString> {
        Ok(bytes_to_b64_string(bytes.as_bytes(&ctx)?))
    }

    pub fn decode_from_hex<'js>(ctx: Ctx<'js>, encoded: String<'js>) -> Result<Value<'js>> {
        let encoded = encoded.to_cstring()?;
        let bytes = bytes_from_hex(encoded.as_str().as_bytes())
            .or_throw_msg(&ctx, "Cannot decode unrecognized sequence")?;

        Ok(TypedArray::new(ctx, bytes)?.into_value())
    }

    pub fn encode_to_hex<'js>(ctx: Ctx<'js>, bytes: ObjectBytes<'js>) -> Result<StdString> {
        Ok(bytes_to_hex_string(bytes.as_bytes(&ctx)?))
    }
}

impl ModuleDef for LlrtCodecModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("decodeFromBase64")?;
        declare.declare("encodeToBase64")?;
        declare.declare("decodeFromHex")?;
        declare.declare("encodeToHex")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("decodeFromBase64", Func::from(Self::decode_from_base64))?;
            default.set("encodeToBase64", Func::from(Self::encode_to_base64))?;
            default.set("decodeFromHex", Func::from(Self::decode_from_hex))?;
            default.set("encodeToHex", Func::from(Self::encode_to_hex))?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<LlrtCodecModule> for ModuleInfo<LlrtCodecModule> {
    fn from(val: LlrtCodecModule) -> Self {
        ModuleInfo {
            name: "llrt:codec",
            module: val,
        }
    }
}
