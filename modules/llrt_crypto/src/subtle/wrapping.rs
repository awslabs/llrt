// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_json::{parse::json_parse, stringify::json_stringify};
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{Array, ArrayBuffer, Class, Ctx, Exception, Result, Value};

use crate::subtle::CryptoKey;

use super::{
    encryption::{self, encrypt_decrypt},
    encryption_algorithm::EncryptionAlgorithm,
    export_key::{export_key, ExportOutput},
    import_key::import_key,
    key_algorithm::{KeyFormat, KeyFormatData},
    EncryptionMode,
};

pub async fn subtle_wrap_key<'js>(
    ctx: Ctx<'js>,
    format: KeyFormat,
    key: Class<'js, CryptoKey>,
    wrapping_key: Class<'js, CryptoKey>,
    wrap_algo: EncryptionAlgorithm,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();

    let export = export_key(&ctx, format, &key)?;

    let (bytes, padding) = match export {
        ExportOutput::Bytes(bytes) => (bytes, 0),
        ExportOutput::Object(value) => {
            let json = json_stringify(&ctx, value.into_value())?.unwrap();
            (json.into_bytes(), b' ')
        },
    };

    let wrapping_key = wrapping_key.borrow();
    wrapping_key.check_validity("wrapKey").or_throw(&ctx)?;

    let bytes = encrypt_decrypt(
        &ctx,
        &wrap_algo,
        &wrapping_key,
        &bytes,
        EncryptionMode::Wrapping(padding),
        encryption::EncryptionOperation::Encrypt,
    )?;

    ArrayBuffer::new(ctx, bytes)
}

//cant take more than 7 args
pub async fn subtle_unwrap_key<'js>(
    format: KeyFormat,
    wrapped_key: ArrayBuffer<'js>,
    unwrapping_key: Class<'js, CryptoKey>,
    unwrap_algo: EncryptionAlgorithm,
    unwrapped_key_algo: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey>> {
    let unwrapping_key = unwrapping_key.borrow();
    let ctx = wrapped_key.ctx().clone();
    unwrapping_key.check_validity("unwrapKey").or_throw(&ctx)?;

    let bytes = wrapped_key
        .as_bytes()
        .ok_or_else(|| Exception::throw_message(&ctx, "ArrayBuffer is detached"))?;

    let padding = match format {
        KeyFormat::Jwk => b' ',
        _ => 0,
    };

    let bytes = encrypt_decrypt(
        &ctx,
        &unwrap_algo,
        &unwrapping_key,
        bytes,
        EncryptionMode::Wrapping(padding),
        encryption::EncryptionOperation::Decrypt,
    )?;

    let key_format = match format {
        KeyFormat::Jwk => {
            KeyFormatData::Jwk(json_parse(&ctx, bytes)?.into_object_or_throw(&ctx, "wrappedKey")?)
        },
        KeyFormat::Raw => KeyFormatData::Raw(ObjectBytes::Vec(bytes)),
        KeyFormat::Spki => KeyFormatData::Spki(ObjectBytes::Vec(bytes)),
        KeyFormat::Pkcs8 => KeyFormatData::Pkcs8(ObjectBytes::Vec(bytes)),
    };

    import_key(ctx, key_format, unwrapped_key_algo, extractable, key_usages)
}
