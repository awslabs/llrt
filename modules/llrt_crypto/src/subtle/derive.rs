// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::result::ResultExt;
use rquickjs::{Array, ArrayBuffer, Class, Ctx, Exception, Result, Value};

use crate::{provider::CryptoProvider, subtle::CryptoKey, CRYPTO_PROVIDER};

use super::{
    algorithm_mismatch_error, algorithm_not_supported_error,
    crypto_key::KeyKind,
    derive_algorithm::DeriveAlgorithm,
    key_algorithm::{
        EcAlgorithm, KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages, KeyDerivation,
    },
};

pub async fn subtle_derive_bits<'js>(
    ctx: Ctx<'js>,
    algorithm: DeriveAlgorithm,
    base_key: Class<'js, CryptoKey>,
    length: u32,
) -> Result<ArrayBuffer<'js>> {
    let base_key = base_key.borrow();
    base_key.check_validity("deriveBits").or_throw(&ctx)?;

    let bytes = derive_bits(&ctx, &algorithm, &base_key, length)?;
    ArrayBuffer::new(ctx, bytes)
}

fn derive_bits(
    ctx: &Ctx<'_>,
    algorithm: &DeriveAlgorithm,
    base_key: &CryptoKey,
    length: u32,
) -> Result<Vec<u8>> {
    match algorithm {
        DeriveAlgorithm::Ecdh { curve, public_key } => {
            if let KeyAlgorithm::Ec {
                curve: base_key_curve,
                algorithm,
            } = &base_key.algorithm
            {
                if curve == base_key_curve
                    && base_key.kind == KeyKind::Private
                    && matches!(algorithm, EcAlgorithm::Ecdh)
                {
                    let handle = &base_key.handle;
                    return CRYPTO_PROVIDER
                        .ecdh_derive_bits(*curve, handle, public_key)
                        .or_throw(ctx);
                }
                return Err(Exception::throw_message(
                    ctx,
                    "ECDH curve must be same as baseKey",
                ));
            }
            algorithm_mismatch_error(ctx, "ECDH")
        },
        DeriveAlgorithm::X25519 { public_key } => {
            if !matches!(base_key.algorithm, KeyAlgorithm::X25519) {
                return algorithm_mismatch_error(ctx, "X25519");
            }

            CRYPTO_PROVIDER
                .x25519_derive_bits(&base_key.handle, public_key)
                .or_throw(ctx)
        },
        DeriveAlgorithm::Derive(KeyDerivation::Hkdf { hash, salt, info }) => {
            if !matches!(base_key.algorithm, KeyAlgorithm::HkdfImport) {
                return algorithm_mismatch_error(ctx, "HKDF");
            }
            let out_length = (length / 8).try_into().or_throw(ctx)?;
            CRYPTO_PROVIDER
                .hkdf_derive_key(&base_key.handle, salt, info, out_length, *hash)
                .or_throw(ctx)
        },
        DeriveAlgorithm::Derive(KeyDerivation::Pbkdf2 {
            hash,
            salt,
            iterations,
        }) => {
            if !matches!(base_key.algorithm, KeyAlgorithm::Pbkdf2Import) {
                return algorithm_mismatch_error(ctx, "PBKDF2");
            }
            let out_length = (length / 8).try_into().or_throw(ctx)?;
            CRYPTO_PROVIDER
                .pbkdf2_derive_key(&base_key.handle, salt, *iterations, out_length, *hash)
                .or_throw(ctx)
        },
    }
}

pub async fn subtle_derive_key<'js>(
    ctx: Ctx<'js>,
    algorithm: DeriveAlgorithm,
    base_key: Class<'js, CryptoKey>,
    derived_key_algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey>> {
    let KeyAlgorithmWithUsages {
        algorithm: derived_key_algorithm,
        name,
        public_usages,
        ..
    } = KeyAlgorithm::from_js(
        &ctx,
        KeyAlgorithmMode::Derive,
        derived_key_algorithm,
        key_usages,
    )?;

    let length = match &derived_key_algorithm {
        KeyAlgorithm::Aes { length } => *length,
        KeyAlgorithm::Hmac { length, .. } => *length,
        KeyAlgorithm::Derive { .. } => 0,
        _ => {
            return algorithm_not_supported_error(&ctx);
        },
    };

    let base_key = &base_key.borrow();

    let bytes = derive_bits(&ctx, &algorithm, base_key, length as u32)?;

    let key = CryptoKey::new(
        KeyKind::Secret,
        name,
        extractable,
        derived_key_algorithm,
        public_usages,
        bytes,
    );

    Class::instance(ctx, key)
}
