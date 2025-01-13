// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use ecdsa::signature::hazmat::PrehashVerifier;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use ring::{
    hmac::{Context as HmacContext, Key as HmacKey},
    signature::UnparsedPublicKey,
};
use rquickjs::{Class, Ctx, Result};
use rsa::{
    pkcs1v15::Pkcs1v15Sign,
    pss::Pss,
    sha2::{Sha256, Sha384, Sha512},
};

use crate::{
    sha_hash::ShaAlgorithm,
    subtle::{digest, CryptoKey},
};

use super::{
    algorithm_mismatch_error, key_algorithm::KeyAlgorithm, rsa_private_key,
    sign_algorithm::SigningAlgorithm, EllipticCurve,
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
        signature.as_bytes(),
        data.as_bytes(),
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

            let hash = digest::digest(hash, data);

            match curve {
                EllipticCurve::P256 => {
                    let verifying_key =
                        p256::ecdsa::VerifyingKey::from_sec1_bytes(handle).or_throw(ctx)?;
                    let signature = p256::ecdsa::Signature::from_slice(signature).or_throw(ctx)?;
                    verifying_key.verify_prehash(&hash, &signature).is_ok()
                },
                EllipticCurve::P384 => {
                    let verifying_key =
                        p384::ecdsa::VerifyingKey::from_sec1_bytes(handle).or_throw(ctx)?;
                    let signature = p384::ecdsa::Signature::from_slice(signature).or_throw(ctx)?;
                    verifying_key.verify_prehash(&hash, &signature).is_ok()
                },
                EllipticCurve::P521 => {
                    let verifying_key =
                        p521::ecdsa::VerifyingKey::from_sec1_bytes(handle).or_throw(ctx)?;
                    let signature = p521::ecdsa::Signature::from_slice(signature).or_throw(ctx)?;
                    verifying_key.verify_prehash(&hash, &signature).is_ok()
                },
            }
        },
        SigningAlgorithm::Ed25519 => {
            if !matches!(&key.algorithm, KeyAlgorithm::Ed25519) {
                return algorithm_mismatch_error(ctx, "Ed25519");
            }

            let public_key = UnparsedPublicKey::new(&ring::signature::ED25519, handle);
            public_key.verify(data, signature).is_ok()
        },
        SigningAlgorithm::Hmac => {
            let hash = match &key.algorithm {
                KeyAlgorithm::Hmac { hash, .. } => hash,
                _ => return algorithm_mismatch_error(ctx, "HMAC"),
            };

            let key = HmacKey::new(*hash.hmac_algorithm(), handle);
            let mut hmac = HmacContext::with_key(&key);
            hmac.update(data);
            hmac.sign().as_ref() == signature
        },
        SigningAlgorithm::RsaPss { salt_length } => rsa_verify(
            ctx,
            key,
            "RSA-PSS",
            data,
            |hash, digest, public_key| match hash {
                ShaAlgorithm::SHA256 => public_key
                    .verify(
                        Pss::new_with_salt::<Sha256>(*salt_length as usize),
                        digest,
                        signature,
                    )
                    .is_ok(),
                ShaAlgorithm::SHA384 => public_key
                    .verify(
                        Pss::new_with_salt::<Sha384>(*salt_length as usize),
                        digest,
                        signature,
                    )
                    .is_ok(),
                ShaAlgorithm::SHA512 => public_key
                    .verify(
                        Pss::new_with_salt::<Sha512>(*salt_length as usize),
                        digest,
                        signature,
                    )
                    .is_ok(),
                _ => unreachable!(),
            },
        )?,
        SigningAlgorithm::RsassaPkcs1v15 => rsa_verify(
            ctx,
            key,
            "RSASSA-PKCS1-v1_5",
            data,
            |hash, digest, public_key| match hash {
                ShaAlgorithm::SHA256 => public_key
                    .verify(Pkcs1v15Sign::new::<Sha256>(), digest, signature)
                    .is_ok(),
                ShaAlgorithm::SHA384 => public_key
                    .verify(Pkcs1v15Sign::new::<Sha384>(), digest, signature)
                    .is_ok(),
                ShaAlgorithm::SHA512 => public_key
                    .verify(Pkcs1v15Sign::new::<Sha512>(), digest, signature)
                    .is_ok(),
                _ => unreachable!(),
            },
        )?,
    })
}

// Helper function for RSA verification
fn rsa_verify<F>(
    ctx: &Ctx<'_>,
    key: &CryptoKey,
    algorithm_name: &str,
    data: &[u8],
    verify_fn: F,
) -> Result<bool>
where
    F: FnOnce(&ShaAlgorithm, &[u8], &rsa::RsaPublicKey) -> bool,
{
    let (private_key, hash, digest) = rsa_private_key(ctx, key, data, algorithm_name)?;

    let public_key = private_key.to_public_key();

    let result = verify_fn(hash, digest.as_ref(), &public_key);

    Ok(result)
}
