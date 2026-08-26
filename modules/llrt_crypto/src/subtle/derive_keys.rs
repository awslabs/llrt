// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::future::Future;

use llrt_utils::bytes::ObjectBytes;
use rquickjs::{Array, Class, Ctx, FromJs, Result, Value};

use super::import_key::import_key;
use super::{
    algorithm_not_supported_error,
    crypto_key::CryptoKey,
    derive_algorithm::DeriveAlgorithm,
    derive_bits::{derive_bits, DeriveBitsLength},
    key_algorithm::{KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages, KeyFormatData},
    util::ResultDomExt,
};

pub fn subtle_derive_key<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    base_key: Class<'js, CryptoKey<'js>>,
    derived_key_algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> impl Future<Output = Result<Class<'js, CryptoKey<'js>>>> + 'js {
    let prepared = prepare_derive_key(&ctx, algorithm, derived_key_algorithm, key_usages);

    async move {
        let (algorithm, key_algorithm, derived_key_algorithm, key_usages) = prepared?;

        derive_key(
            &ctx,
            &algorithm,
            &base_key,
            derived_key_algorithm,
            extractable,
            key_usages,
            key_algorithm,
        )
    }
}

fn prepare_derive_key<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    derived_key_algorithm: Value<'js>,
    key_usages: Array<'js>,
) -> Result<(
    DeriveAlgorithm,
    KeyAlgorithmWithUsages,
    Value<'js>,
    Array<'js>,
)> {
    let algorithm = DeriveAlgorithm::from_js(ctx, algorithm)?;
    let key_algorithm = KeyAlgorithm::from_js(
        ctx,
        KeyAlgorithmMode::Derive,
        derived_key_algorithm.clone(),
        key_usages.clone(),
    )?;

    Ok((algorithm, key_algorithm, derived_key_algorithm, key_usages))
}

fn derive_key<'js>(
    ctx: &Ctx<'js>,
    algorithm: &DeriveAlgorithm,
    base_key: &Class<'js, CryptoKey<'js>>,
    derived_key_algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
    key_algorithm: KeyAlgorithmWithUsages,
) -> Result<Class<'js, CryptoKey<'js>>> {
    let length = match &key_algorithm.algorithm {
        KeyAlgorithm::Aes { length, .. } => DeriveBitsLength::Specified(u32::from(*length)),
        KeyAlgorithm::Hmac { length, .. } => DeriveBitsLength::Specified(*length),
        KeyAlgorithm::HkdfImport | KeyAlgorithm::Pbkdf2Import => DeriveBitsLength::Default,
        _ => {
            return algorithm_not_supported_error(ctx);
        },
    };

    let base_key = base_key.borrow();

    base_key.check_validity("deriveKey").or_throw_dom(ctx)?;

    let bytes = derive_bits(ctx, algorithm, &base_key, length)?;

    import_key(
        ctx.clone(),
        KeyFormatData::Raw(ObjectBytes::Vec(bytes)),
        derived_key_algorithm,
        extractable,
        key_usages,
    )
}
