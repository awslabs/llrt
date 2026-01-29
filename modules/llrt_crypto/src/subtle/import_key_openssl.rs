// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! OpenSSL-based key import implementation.

use llrt_utils::{bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{Array, Class, Ctx, FromJs, Result, Value};

use super::{
    crypto_key::KeyKind,
    key_algorithm::{
        KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages, KeyFormat, KeyFormatData,
    },
    CryptoKey,
};

pub async fn subtle_import_key<'js>(
    ctx: Ctx<'js>,
    format: KeyFormat,
    key_data: Value<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey>> {
    let format = match format {
        KeyFormat::Raw => KeyFormatData::Raw(ObjectBytes::from_js(&ctx, key_data)?),
        KeyFormat::Pkcs8 => KeyFormatData::Pkcs8(ObjectBytes::from_js(&ctx, key_data)?),
        KeyFormat::Spki => KeyFormatData::Spki(ObjectBytes::from_js(&ctx, key_data)?),
        KeyFormat::Jwk => KeyFormatData::Jwk(key_data.into_object_or_throw(&ctx, "keyData")?),
    };

    import_key(ctx, format, algorithm, extractable, key_usages)
}

pub fn import_key<'js>(
    ctx: Ctx<'js>,
    format: KeyFormatData<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey>> {
    let mut kind = KeyKind::Public;
    let mut data = Vec::new();

    let KeyAlgorithmWithUsages {
        name,
        algorithm: key_algorithm,
        public_usages,
        private_usages,
    } = KeyAlgorithm::from_js(
        &ctx,
        KeyAlgorithmMode::Import {
            kind: &mut kind,
            data: &mut data,
            format,
        },
        algorithm,
        key_usages,
    )?;

    let usages = match kind {
        KeyKind::Public | KeyKind::Secret => public_usages,
        KeyKind::Private => private_usages,
    };

    Class::instance(
        ctx,
        CryptoKey::new(kind, name, extractable, key_algorithm, usages, data),
    )
}
