// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Stub implementations for SubtleCrypto operations when `_rustcrypto` feature is disabled.
//! These return errors indicating the operation is not supported.

use rquickjs::{Ctx, Exception, Object, Result, Value};

use super::crypto_key::CryptoKey;
use super::encryption_algorithm;
use super::key_algorithm;

pub async fn subtle_export_key<'js>(
    ctx: Ctx<'js>,
    _format: key_algorithm::KeyFormat,
    _key: rquickjs::Class<'js, CryptoKey>,
) -> Result<Object<'js>> {
    Err(Exception::throw_message(
        &ctx,
        "exportKey is not supported with this crypto provider",
    ))
}

pub async fn subtle_import_key<'js>(
    ctx: Ctx<'js>,
    _format: key_algorithm::KeyFormat,
    _key_data: Value<'js>,
    _algorithm: Value<'js>,
    _extractable: bool,
    _key_usages: rquickjs::Array<'js>,
) -> Result<rquickjs::Class<'js, CryptoKey>> {
    Err(Exception::throw_message(
        &ctx,
        "importKey is not supported with this crypto provider",
    ))
}

pub async fn subtle_wrap_key<'js>(
    ctx: Ctx<'js>,
    _format: key_algorithm::KeyFormat,
    _key: rquickjs::Class<'js, CryptoKey>,
    _wrapping_key: rquickjs::Class<'js, CryptoKey>,
    _wrap_algo: encryption_algorithm::EncryptionAlgorithm,
) -> Result<rquickjs::ArrayBuffer<'js>> {
    Err(Exception::throw_message(
        &ctx,
        "wrapKey is not supported with this crypto provider",
    ))
}

pub async fn subtle_unwrap_key<'js>(
    _format: key_algorithm::KeyFormat,
    wrapped_key: rquickjs::ArrayBuffer<'js>,
    _unwrapping_key: rquickjs::Class<'js, CryptoKey>,
    _unwrap_algo: encryption_algorithm::EncryptionAlgorithm,
    _unwrapped_key_algo: Value<'js>,
    _extractable: bool,
    _key_usages: rquickjs::Array<'js>,
) -> Result<rquickjs::Class<'js, CryptoKey>> {
    let ctx = wrapped_key.ctx().clone();
    Err(Exception::throw_message(
        &ctx,
        "unwrapKey is not supported with this crypto provider",
    ))
}
