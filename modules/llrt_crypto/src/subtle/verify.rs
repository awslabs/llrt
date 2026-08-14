// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::future::Future;

use crate::provider::{CryptoError, CryptoProvider, HmacProvider};
use llrt_utils::bytes::ObjectBytes;
use rquickjs::{Class, Ctx, FromJs, Result, Value};

use crate::CRYPTO_PROVIDER;

use super::{
    algorithm_invalid_access_error,
    crypto_key::{CryptoKey, KeyKind},
    digest,
    key_algorithm::KeyAlgorithm,
    rsa_hash_digest,
    sign_algorithm::SigningAlgorithm,
    util::ResultDomExt,
};

pub fn subtle_verify<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    signature: ObjectBytes<'js>,
    data: ObjectBytes<'js>,
) -> impl Future<Output = Result<bool>> + 'js {
    let prepared = prepare_verify(&ctx, algorithm, key, signature, data);

    async move {
        let (algorithm, key, signature, data) = prepared?;
        let key = key.borrow();
        if key.name.as_ref() != algorithm.name() {
            return algorithm_invalid_access_error(&ctx, algorithm.name());
        }
        key.check_validity("verify").or_throw_dom(&ctx)?;
        let expected_kind = match &algorithm {
            SigningAlgorithm::Hmac => KeyKind::Secret,
            _ => KeyKind::Public,
        };
        key.check_kind(expected_kind).or_throw_dom(&ctx)?;

        verify(&ctx, &algorithm, &key, &signature, &data)
    }
}

type PreparedVerify<'js> = (
    SigningAlgorithm,
    Class<'js, CryptoKey<'js>>,
    Vec<u8>,
    Vec<u8>,
);

fn prepare_verify<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    key: Class<'js, CryptoKey<'js>>,
    signature: ObjectBytes<'js>,
    data: ObjectBytes<'js>,
) -> Result<PreparedVerify<'js>> {
    let algorithm = SigningAlgorithm::from_js(ctx, algorithm)?;
    let signature = signature.as_bytes_opt().unwrap_or_default().to_vec();
    let data = data.as_bytes_opt().unwrap_or_default().to_vec();
    Ok((algorithm, key, signature, data))
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
                _ => return algorithm_invalid_access_error(ctx, "ECDSA"),
            };

            let digest = digest::digest(hash, data);

            crate::CRYPTO_PROVIDER
                .ecdsa_verify(*curve, handle, signature, &digest)
                .into_verification(ctx)?
        },
        SigningAlgorithm::Ed25519 => {
            if !matches!(&key.algorithm, KeyAlgorithm::Ed25519) {
                return algorithm_invalid_access_error(ctx, "Ed25519");
            }

            crate::CRYPTO_PROVIDER
                .ed25519_verify(handle, signature, data)
                .into_verification(ctx)?
        },
        SigningAlgorithm::Hmac => {
            let hash = match &key.algorithm {
                KeyAlgorithm::Hmac { hash, .. } => hash,
                _ => return algorithm_invalid_access_error(ctx, "HMAC"),
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
                .into_verification(ctx)?
        },
        SigningAlgorithm::RsassaPkcs1v15 => {
            let (hash, digest) = rsa_hash_digest(ctx, key, data, "RSASSA-PKCS1-v1_5")?;
            crate::CRYPTO_PROVIDER
                .rsa_pkcs1v15_verify(&key.handle, signature, digest.as_ref(), *hash)
                .into_verification(ctx)?
        },
    })
}

trait VerificationResultExt {
    fn into_verification(self, ctx: &Ctx<'_>) -> Result<bool>;
}

impl VerificationResultExt for std::result::Result<bool, CryptoError> {
    fn into_verification(self, ctx: &Ctx<'_>) -> Result<bool> {
        match self {
            Err(CryptoError::InvalidSignature(_)) => Ok(false),
            result => result.or_throw_dom(ctx),
        }
    }
}
