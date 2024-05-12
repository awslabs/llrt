// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use once_cell::sync::Lazy;
use ring::rand::SecureRandom;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    Ctx, Function, Result, TypedArray, Value,
};
use uuid::Uuid;
use uuid_simd::UuidExt;

use crate::{
    module_builder::ModuleInfo,
    modules::{crypto::SYSTEM_RANDOM, encoding::encoder::bytes_to_hex, module::export_default},
    utils::{
        object::{get_bytes, get_bytes_offset_length},
        result::ResultExt,
    },
};

pub struct UuidModule;

static ERROR_MESSAGE: &str = "Not a valid UUID";

static NODE_ID: Lazy<[u8; 6]> = Lazy::new(|| {
    let mut bytes = [0; 6];
    SYSTEM_RANDOM.fill(&mut bytes).unwrap();
    bytes
});

fn from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Uuid> {
    if value.is_string() {
        Uuid::try_parse(&value.as_string().unwrap().to_string()?)
    } else {
        Uuid::from_slice(&get_bytes(ctx, value)?)
    }
    .or_throw_msg(ctx, ERROR_MESSAGE)
}

fn uuidv1() -> String {
    Uuid::now_v1(&NODE_ID).format_hyphenated().to_string()
}

fn uuidv3<'js>(ctx: Ctx<'js>, name: String, namespace: Value<'js>) -> Result<String> {
    let uuid = Uuid::new_v3(&from_value(&ctx, namespace)?, name.as_bytes())
        .format_hyphenated()
        .to_string();
    Ok(uuid)
}

fn uuidv5<'js>(ctx: Ctx<'js>, name: String, namespace: Value<'js>) -> Result<String> {
    let uuid = Uuid::new_v5(&from_value(&ctx, namespace)?, name.as_bytes())
        .format_hyphenated()
        .to_string();
    Ok(uuid)
}

pub fn uuidv4() -> String {
    Uuid::new_v4().format_hyphenated().to_string()
}

fn parse(ctx: Ctx<'_>, value: String) -> Result<TypedArray<u8>> {
    let uuid = Uuid::try_parse(&value).or_throw_msg(&ctx, ERROR_MESSAGE)?;
    let bytes = uuid.as_bytes();
    TypedArray::<u8>::new(ctx, *bytes)
}

fn stringify<'js>(ctx: Ctx<'js>, value: Value<'js>, offset: Opt<u8>) -> Result<String> {
    let value = get_bytes_offset_length(&ctx, value, offset.0.map(|o| o.into()), None)?;
    let value = bytes_to_hex(&value);

    let uuid = Uuid::try_parse_ascii(&value)
        .or_throw_msg(&ctx, ERROR_MESSAGE)?
        .as_hyphenated()
        .to_string();

    Ok(uuid)
}

fn validate(value: String) -> bool {
    Uuid::parse_str(&value).is_ok()
}

fn version(ctx: Ctx<'_>, value: String) -> Result<u8> {
    let uuid = Uuid::parse_str(&value).or_throw_msg(&ctx, ERROR_MESSAGE)?;
    Ok(uuid.get_version().map(|v| v as u8).unwrap_or(0))
}

impl ModuleDef for UuidModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("v1")?;
        declare.declare("v3")?;
        declare.declare("v4")?;
        declare.declare("v5")?;
        declare.declare("parse")?;
        declare.declare("validate")?;
        declare.declare("stringify")?;
        declare.declare("version")?;
        declare.declare("NIL")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let dns_namespace = Uuid::NAMESPACE_DNS.format_hyphenated().to_string();
            let url_namespace = Uuid::NAMESPACE_URL.format_hyphenated().to_string();

            let v3_func = Function::new(ctx.clone(), uuidv3)?;
            let v3_object = v3_func.as_object().unwrap();

            let v5_func = Function::new(ctx.clone(), uuidv5)?;
            let v5_object = v5_func.as_object().unwrap();

            v3_object.set("DNS", dns_namespace.clone())?;
            v3_object.set("URL", url_namespace.clone())?;
            v5_object.set("DNS", dns_namespace)?;
            v5_object.set("URL", url_namespace)?;

            default.set("v1", Func::from(uuidv1))?;
            default.set("v3", v3_func)?;
            default.set("v4", Func::from(uuidv4))?;
            default.set("v5", v5_func)?;
            default.set("NIL", "00000000-0000-0000-0000-000000000000")?;
            default.set("parse", Func::from(parse))?;
            default.set("stringify", Func::from(stringify))?;
            default.set("validate", Func::from(validate))?;
            default.set("version", Func::from(version))?;
            Ok(())
        })
    }
}

impl From<UuidModule> for ModuleInfo<UuidModule> {
    fn from(val: UuidModule) -> Self {
        ModuleInfo {
            name: "uuid",
            module: val,
        }
    }
}
