// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{Array, Class, Ctx, Exception, FromJs, Result, Value};

use crate::subtle::CryptoKey;

use super::{
    crypto_key::KeyKind,
    key_algorithm::{KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages, KeyFormat},
};

#[allow(dead_code)]
pub async fn subtle_import_key<'js>(
    ctx: Ctx<'js>,
    format: String,
    key_data: Value<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey>> {
    let format: KeyFormat = match format.as_str() {
        "raw" => KeyFormat::Raw(ObjectBytes::from_js(&ctx, key_data)?),
        "pkcs8" => KeyFormat::Pkcs8(ObjectBytes::from_js(&ctx, key_data)?),
        "spki" => KeyFormat::Spki(ObjectBytes::from_js(&ctx, key_data)?),
        "jwk" => KeyFormat::Jwk(key_data.into_object_or_throw(&ctx, "keyData")?),
        _ => {
            return Err(Exception::throw_type(
                &ctx,
                &["Invalid format: ", &format].concat(),
            ))
        },
    };

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
