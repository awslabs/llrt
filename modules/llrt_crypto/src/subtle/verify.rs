// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::provider::{CryptoProvider, HmacProvider};
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{Class, Ctx, Result};

use crate::{
    subtle::{digest, CryptoKey},
    CRYPTO_PROVIDER,
};

use super::{
    algorithm_mismatch_error, key_algorithm::KeyAlgorithm, rsa_hash_digest,
    sign_algorithm::SigningAlgorithm,
};

pub async fn subtle_verify<'js>(
    ctx: Ctx<'js>,
    algorithm: SigningAlgorithm,
    key: Class<'js, CryptoKey>,
    signature: ObjectBytes<'js>,
    data: ObjectBytes<'js>,
) -> Result<bool> {
    let key = key.borrow();
    key.check_validity("verify").or_throw(&ctx)?;

    verify(
        &ctx,
        &algorithm,
        &key,
        signature.as_bytes(&ctx)?,
        data.as_bytes(&ctx)?,
    )
}

fn verify(
    ctx: &Ctx<'_>,
    algorithm: &SigningAlgorithm,
    key: &CryptoKey,
    signature: &[u8],
    data: &[u8],
) -> Result<bool> {
    let handle = key.handle.as_ref();
    Ok(match algorithm {
        SigningAlgorithm::Ecdsa { hash } => {
            let curve = match &key.algorithm {
                KeyAlgorithm::Ec { curve, .. } => curve,
                _ => return algorithm_mismatch_error(ctx, "ECDSA"),
            };

            let digest = digest::digest(hash, data);

            crate::CRYPTO_PROVIDER
                .ecdsa_verify(*curve, handle, signature, &digest)
                .or_throw(ctx)?
        },
        SigningAlgorithm::Ed25519 => {
            if !matches!(&key.algorithm, KeyAlgorithm::Ed25519) {
                return algorithm_mismatch_error(ctx, "Ed25519");
            }

            crate::CRYPTO_PROVIDER
                .ed25519_verify(handle, signature, data)
                .or_throw(ctx)?
        },
        SigningAlgorithm::Hmac => {
            let hash = match &key.algorithm {
                KeyAlgorithm::Hmac { hash, .. } => hash,
                _ => return algorithm_mismatch_error(ctx, "HMAC"),
            };

            let mut hmac = CRYPTO_PROVIDER.hmac(*hash, handle);
            hmac.update(data);
            let computed_signature = hmac.finalize();

            computed_signature == signature
        },
        SigningAlgorithm::RsaPss { salt_length } => {
            let (hash, digest) = rsa_hash_digest(ctx, key, data, "RSA-PSS")?;
            crate::CRYPTO_PROVIDER
                .rsa_pss_verify(
                    &key.handle,
                    signature,
                    digest.as_ref(),
                    *salt_length as usize,
                    *hash,
                )
                .or_throw(ctx)?
        },
        SigningAlgorithm::RsassaPkcs1v15 => {
            let (hash, digest) = rsa_hash_digest(ctx, key, data, "RSASSA-PKCS1-v1_5")?;
            crate::CRYPTO_PROVIDER
                .rsa_pkcs1v15_verify(&key.handle, signature, digest.as_ref(), *hash)
                .or_throw(ctx)?
        },
    })
}
