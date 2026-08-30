// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_exceptions::DOMException;
use rquickjs::{object::Property, Array, Class, Ctx, Object, Result, Value};

use crate::{provider::CryptoProvider, CRYPTO_PROVIDER};

use super::{
    algorithm_not_supported_error,
    crypto_key::{CryptoKey, KeyKind},
    key_algorithm::{KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages},
    util::ResultDomExt,
};

pub async fn subtle_generate_key<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    extractable: Value<'js>,
    key_usages: Array<'js>,
) -> Result<Value<'js>> {
    let KeyAlgorithmWithUsages {
        name,
        algorithm: key_algorithm,
        private_usages,
        public_usages,
    } = KeyAlgorithm::from_js(&ctx, KeyAlgorithmMode::Generate, algorithm, key_usages)?;

    let (private_key, public_or_secret_key) = generate_key(&ctx, &key_algorithm)?;

    let Some(extractable) = extractable.as_bool() else {
        return Err(DOMException::not_supported_error(&ctx, "Invalid parameter"));
    };

    if matches!(
        key_algorithm,
        KeyAlgorithm::Aes { .. } | KeyAlgorithm::Hmac { .. }
    ) {
        return Ok(Class::instance(
            ctx,
            CryptoKey::new(
                KeyKind::Secret,
                name,
                extractable,
                key_algorithm,
                public_usages,
                public_or_secret_key,
            ),
        )?
        .into_value());
    }

    let private_key = Class::instance(
        ctx.clone(),
        CryptoKey::new(
            KeyKind::Private,
            name.clone(),
            extractable,
            key_algorithm.clone(),
            private_usages,
            private_key,
        ),
    )?;

    let public_key = Class::instance(
        ctx.clone(),
        CryptoKey::new(
            KeyKind::Public,
            name,
            true,
            key_algorithm,
            public_usages,
            public_or_secret_key,
        ),
    )?;

    let key_pair = Object::new(ctx.clone())?;
    key_pair.prop("privateKey", Property::from(private_key).enumerable())?;
    key_pair.prop("publicKey", Property::from(public_key).enumerable())?;
    Ok(key_pair.into_value())
}

fn generate_key(ctx: &Ctx<'_>, algorithm: &KeyAlgorithm) -> Result<(Vec<u8>, Vec<u8>)> {
    match algorithm {
        KeyAlgorithm::Aes { length, .. } => {
            // Default to AES-256
            let key = CRYPTO_PROVIDER
                .generate_aes_key(*length)
                .or_throw_dom_with_msg(ctx, "AES key generation failed")?;
            Ok((vec![], key))
        },
        KeyAlgorithm::Hmac { hash, length } => {
            let key = CRYPTO_PROVIDER
                .generate_hmac_key(*hash, *length as u16)
                .or_throw_dom_with_msg(ctx, "HMAC key generation failed")?;
            Ok((vec![], key))
        },
        KeyAlgorithm::Ec { curve, .. } => CRYPTO_PROVIDER
            .generate_ec_key(*curve)
            .or_throw_dom_with_msg(ctx, "EC key generation failed"),
        KeyAlgorithm::Ed25519 => CRYPTO_PROVIDER
            .generate_ed25519_key()
            .or_throw_dom_with_msg(ctx, "Ed25519 key generation failed"),
        KeyAlgorithm::X25519 => CRYPTO_PROVIDER
            .generate_x25519_key()
            .or_throw_dom_with_msg(ctx, "X25519 key generation failed"),
        KeyAlgorithm::Rsa {
            modulus_length,
            public_exponent,
            ..
        } => CRYPTO_PROVIDER
            .generate_rsa_key(*modulus_length, public_exponent.as_ref())
            .or_throw_dom_with_msg(ctx, "RSA key generation failed"),
        _ => algorithm_not_supported_error(ctx),
    }
}

#[allow(dead_code)]
fn generate_symmetric_key(_ctx: &Ctx<'_>, length: usize) -> Result<Vec<u8>> {
    Ok(crate::random_byte_array(length))
}
