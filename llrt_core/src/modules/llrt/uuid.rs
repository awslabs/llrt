// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_encoding::bytes_to_hex;
use llrt_utils::bytes::ObjectBytes;
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
    modules::{crypto::SYSTEM_RANDOM, module::export_default},
    utils::result::ResultExt,
};

pub struct LlrtUuidModule;

const MAX_UUID: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";

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
        let bytes = ObjectBytes::from(ctx, &value)?;
        let bytes = bytes.as_bytes(ctx)?;
        Uuid::from_slice(bytes)
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

fn uuidv6() -> String {
    Uuid::now_v6(&NODE_ID).format_hyphenated().to_string()
}

fn uuidv7() -> String {
    Uuid::now_v7().format_hyphenated().to_string()
}

fn uuidv1_to_v6<'js>(ctx: Ctx<'js>, v1_value: Value<'js>) -> Result<String> {
    let v1_uuid = from_value(&ctx, v1_value)?;
    let v1_bytes = v1_uuid.as_bytes();
    let mut v6_bytes = [0u8; 16];

    // time_high
    v6_bytes[0] = ((v1_bytes[6] & 0x0f) << 4) | ((v1_bytes[7] & 0xf0) >> 4);
    v6_bytes[1] = ((v1_bytes[7] & 0x0f) << 4) | ((v1_bytes[4] & 0xf0) >> 4);
    v6_bytes[2] = ((v1_bytes[4] & 0x0f) << 4) | ((v1_bytes[5] & 0xf0) >> 4);
    v6_bytes[3] = ((v1_bytes[5] & 0x0f) << 4) | ((v1_bytes[0] & 0xf0) >> 4);

    // time_mid
    v6_bytes[4] = ((v1_bytes[0] & 0x0f) << 4) | ((v1_bytes[1] & 0xf0) >> 4);
    v6_bytes[5] = ((v1_bytes[1] & 0x0f) << 4) | ((v1_bytes[2] & 0xf0) >> 4);

    // version and time_low
    v6_bytes[6] = 0x60 | (v1_bytes[2] & 0x0f);
    v6_bytes[7] = v1_bytes[3];

    // clock_seq and node
    v6_bytes[8..16].copy_from_slice(&v1_bytes[8..16]);

    Ok(Uuid::from_bytes(v6_bytes).format_hyphenated().to_string())
}

fn uuidv6_to_v1<'js>(ctx: Ctx<'js>, v6_value: Value<'js>) -> Result<String> {
    let v6_uuid = from_value(&ctx, v6_value)?;
    let v6_bytes: &[u8; 16] = v6_uuid.as_bytes();
    let mut v1_bytes = [0u8; 16];

    // time_low
    v1_bytes[0] = ((v6_bytes[3] & 0x0f) << 4) | ((v6_bytes[4] & 0xf0) >> 4);
    v1_bytes[1] = ((v6_bytes[4] & 0x0f) << 4) | ((v6_bytes[5] & 0xf0) >> 4);
    v1_bytes[2] = ((v6_bytes[5] & 0x0f) << 4) | (v6_bytes[6] & 0x0f);
    v1_bytes[3] = v6_bytes[7];

    // time_mid
    v1_bytes[4] = ((v6_bytes[1] & 0x0f) << 4) | ((v6_bytes[2] & 0xf0) >> 4);
    v1_bytes[5] = ((v6_bytes[2] & 0x0f) << 4) | ((v6_bytes[3] & 0xf0) >> 4);

    // version and time_high
    v1_bytes[6] = 0x10 | ((v6_bytes[0] & 0xf0) >> 4);
    v1_bytes[7] = ((v6_bytes[0] & 0x0f) << 4) | ((v6_bytes[1] & 0xf0) >> 4);

    // clock_seq and node
    v1_bytes[8..16].copy_from_slice(&v6_bytes[8..16]);

    Ok(Uuid::from_bytes(v1_bytes).format_hyphenated().to_string())
}

fn parse(ctx: Ctx<'_>, value: String) -> Result<TypedArray<u8>> {
    let uuid = Uuid::try_parse(&value).or_throw_msg(&ctx, ERROR_MESSAGE)?;
    let bytes = uuid.as_bytes();
    TypedArray::<u8>::new(ctx, *bytes)
}

fn stringify<'js>(ctx: Ctx<'js>, value: Value<'js>, offset: Opt<u8>) -> Result<String> {
    let bytes = ObjectBytes::from_offset(
        &ctx,
        &value,
        offset.0.map(|o| o.into()).unwrap_or_default(),
        None,
    )?;
    let value = bytes_to_hex(bytes.as_bytes(&ctx)?);

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
    // the Node.js uuid package returns 15 for the version of MAX
    // https://github.com/uuidjs/uuid?tab=readme-ov-file#uuidversionstr
    if value == MAX_UUID {
        return Ok(15);
    }
    let uuid = Uuid::parse_str(&value).or_throw_msg(&ctx, ERROR_MESSAGE)?;
    Ok(uuid.get_version().map(|v| v as u8).unwrap_or(0))
}

impl ModuleDef for LlrtUuidModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("v1")?;
        declare.declare("v3")?;
        declare.declare("v4")?;
        declare.declare("v5")?;
        declare.declare("v6")?;
        declare.declare("v7")?;
        declare.declare("v1ToV6")?;
        declare.declare("v6ToV1")?;
        declare.declare("parse")?;
        declare.declare("validate")?;
        declare.declare("stringify")?;
        declare.declare("version")?;
        declare.declare("NIL")?;
        declare.declare("MAX")?;
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
            default.set("v6", Func::from(uuidv6))?;
            default.set("v7", Func::from(uuidv7))?;
            default.set("v1ToV6", Func::from(uuidv1_to_v6))?;
            default.set("v6ToV1", Func::from(uuidv6_to_v1))?;
            default.set("NIL", "00000000-0000-0000-0000-000000000000")?;
            default.set("MAX", MAX_UUID)?;
            default.set("parse", Func::from(parse))?;
            default.set("stringify", Func::from(stringify))?;
            default.set("validate", Func::from(validate))?;
            default.set("version", Func::from(version))?;
            Ok(())
        })
    }
}

impl From<LlrtUuidModule> for ModuleInfo<LlrtUuidModule> {
    fn from(val: LlrtUuidModule) -> Self {
        ModuleInfo {
            name: "llrt:uuid",
            module: val,
        }
    }
}
