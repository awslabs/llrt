// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::provider::{CryptoProvider, HmacProvider};
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{ArrayBuffer, Class, Ctx, Result};

use crate::{subtle::CryptoKey, CRYPTO_PROVIDER};

use super::{
    algorithm_mismatch_error, key_algorithm::KeyAlgorithm, rsa_hash_digest,
    sign_algorithm::SigningAlgorithm,
};

pub async fn subtle_sign<'js>(
    ctx: Ctx<'js>,
    algorithm: SigningAlgorithm,
    key: Class<'js, CryptoKey>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();
    key.check_validity("sign").or_throw(&ctx)?;

    let bytes = sign(&ctx, &algorithm, &key, data.as_bytes(&ctx)?)?;
    ArrayBuffer::new(ctx, bytes)
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
                _ => return algorithm_mismatch_error(ctx, "ECDSA"),
            };

            let digest = crate::subtle::digest::digest(hash, data);

            crate::CRYPTO_PROVIDER
                .ecdsa_sign(*curve, handle, &digest)
                .or_throw(ctx)?
        },
        SigningAlgorithm::Ed25519 => {
            if !matches!(&key.algorithm, KeyAlgorithm::Ed25519) {
                return algorithm_mismatch_error(ctx, "Ed25519");
            }
            crate::CRYPTO_PROVIDER
                .ed25519_sign(handle, data)
                .or_throw(ctx)?
        },
        SigningAlgorithm::Hmac => {
            let hash = if let KeyAlgorithm::Hmac { hash, .. } = &key.algorithm {
                hash
            } else {
                return algorithm_mismatch_error(ctx, "HMAC");
            };

            let mut hmac = CRYPTO_PROVIDER.hmac(*hash, handle);
            hmac.update(data);
            hmac.finalize()
        },
        SigningAlgorithm::RsaPss { salt_length } => {
            let (hash, digest) = rsa_hash_digest(ctx, key, data, "RSA-PSS")?;
            crate::CRYPTO_PROVIDER
                .rsa_pss_sign(&key.handle, digest.as_ref(), *salt_length as usize, *hash)
                .or_throw(ctx)?
        },
        SigningAlgorithm::RsassaPkcs1v15 => {
            let (hash, digest) = rsa_hash_digest(ctx, key, data, "RSASSA-PKCS1-v1_5")?;
            crate::CRYPTO_PROVIDER
                .rsa_pkcs1v15_sign(&key.handle, digest.as_ref(), *hash)
                .or_throw(ctx)?
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
//     F: FnOnce(&ShaAlgorithm, &[u8], &rsa::RsaPrivateKey) -> Result<Vec<u8>>,
// {
//     let (hash, digest) = rsa_hash_digest(ctx, key, data, algorithm_name)?;

//     sign_fn(hash, digest.as_ref())
// }
