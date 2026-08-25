// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::future::Future;

use crate::provider::{modern, CryptoProvider, HmacProvider};
use llrt_utils::bytes::ObjectBytes;
use rquickjs::{ArrayBuffer, Class, Ctx, FromJs, Result, Value};

use crate::CRYPTO_PROVIDER;

use super::{
    algorithm_invalid_access_error,
    crypto_key::{CryptoKey, KeyKind},
    key_algorithm::KeyAlgorithm,
    rsa_hash_digest,
    sign_algorithm::SigningAlgorithm,
    util::ResultDomExt,
    validate_rsa_pss_salt_length,
};

pub fn subtle_sign<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    data: ObjectBytes<'js>,
) -> impl Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    // Keep preparation outside the async block: Rust async function bodies are deferred until
    // polled, while WebCrypto requires call-time algorithm normalization and input snapshotting.
    // Retaining the Result lets preparation failures reject the rquickjs-created Promise.
    let prepared = prepare_sign(&ctx, algorithm, key, data);

    async move {
        let (algorithm, key, data) = prepared?;
        let key = key.borrow();
        if key.name.as_ref() != algorithm.name() {
            return algorithm_invalid_access_error(&ctx, algorithm.name());
        }
        key.check_validity("sign").or_throw_dom(&ctx)?;
        let expected_kind = match &algorithm {
            SigningAlgorithm::Hmac => KeyKind::Secret,
            _ => KeyKind::Private,
        };
        key.check_kind(expected_kind).or_throw_dom(&ctx)?;

        let bytes = sign(&ctx, &algorithm, &key, &data)?;
        ArrayBuffer::new(ctx, bytes)
    }
}

fn prepare_sign<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    data: ObjectBytes<'js>,
) -> Result<(SigningAlgorithm, Class<'js, CryptoKey<'js>>, Vec<u8>)> {
    let algorithm = SigningAlgorithm::from_js(ctx, algorithm)?;
    let data = data.as_bytes_opt().unwrap_or_default().to_vec();
    Ok((algorithm, key, data))
}

fn sign(
    ctx: &Ctx<'_>,
    algorithm: &SigningAlgorithm,
    key: &CryptoKey,
    data: &[u8],
) -> Result<Vec<u8>> {
    let handle = key.handle.as_ref();
    Ok(match algorithm {
        SigningAlgorithm::Ecdsa { hash } => {
            let curve = match &key.algorithm {
                KeyAlgorithm::Ec { curve, .. } => curve,
                _ => return algorithm_invalid_access_error(ctx, "ECDSA"),
            };

            let digest = crate::subtle::digest::digest(hash, data);

            crate::CRYPTO_PROVIDER
                .ecdsa_sign(*curve, handle, &digest)
                .or_throw_dom(ctx)?
        },
        SigningAlgorithm::Ed25519 => {
            if !matches!(&key.algorithm, KeyAlgorithm::Ed25519) {
                return algorithm_invalid_access_error(ctx, "Ed25519");
            }
            crate::CRYPTO_PROVIDER
                .ed25519_sign(handle, data)
                .or_throw_dom(ctx)?
        },
        SigningAlgorithm::Hmac => {
            let hash = if let KeyAlgorithm::Hmac { hash, .. } = &key.algorithm {
                hash
            } else {
                return algorithm_invalid_access_error(ctx, "HMAC");
            };

            let mut hmac = CRYPTO_PROVIDER.hmac(*hash, handle);
            hmac.update(data);
            hmac.finalize()
        },
        SigningAlgorithm::MlDsa { variant, context } => {
            if !matches!(&key.algorithm, KeyAlgorithm::MlDsa(key_variant) if key_variant == variant)
            {
                return algorithm_invalid_access_error(ctx, variant.name());
            }
            modern::ml_dsa_sign(*variant, handle, data, context).or_throw_dom(ctx)?
        },
        SigningAlgorithm::RsaPss { salt_length } => {
            let (hash, digest) = rsa_hash_digest(ctx, key, data, "RSA-PSS")?;
            validate_rsa_pss_salt_length(ctx, key, hash, *salt_length)?;
            crate::CRYPTO_PROVIDER
                .rsa_pss_sign(&key.handle, digest.as_ref(), *salt_length as usize, *hash)
                .or_throw_dom(ctx)?
        },
        SigningAlgorithm::RsassaPkcs1v15 => {
            let (hash, digest) = rsa_hash_digest(ctx, key, data, "RSASSA-PKCS1-v1_5")?;
            crate::CRYPTO_PROVIDER
                .rsa_pkcs1v15_sign(&key.handle, digest.as_ref(), *hash)
                .or_throw_dom(ctx)?
        },
    })
}

// // Helper function for RSA signing
// fn rsa_sign<F>(
//     ctx: &Ctx<'_>,
//     key: &CryptoKey,
//     algorithm_name: &str,
//     data: &[u8],
//     sign_fn: F,
// ) -> Result<Vec<u8>>
// where
//     F: FnOnce(&HashAlgorithm, &[u8], &rsa::RsaPrivateKey) -> Result<Vec<u8>>,
// {
//     let (hash, digest) = rsa_hash_digest(ctx, key, data, algorithm_name)?;

//     sign_fn(hash, digest.as_ref())
// }
