// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use ring::{
    hmac::{Context as HmacContext, Key as HmacKey},
    signature::{EcdsaKeyPair, Ed25519KeyPair, KeyPair, UnparsedPublicKey},
};
use rquickjs::{Class, Ctx, Exception, Result};
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    pkcs1v15::Pkcs1v15Sign,
    pss::Pss,
    sha2::{Digest, Sha256},
    RsaPrivateKey,
};

use crate::{sha_hash::ShaAlgorithm, subtle::CryptoKey, SYSTEM_RANDOM};

use super::sign_algorithm::SigningAlgorithm;

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
        &key.handle,
        signature.as_bytes(),
        data.as_bytes(),
    )
}

fn verify(
    ctx: &Ctx<'_>,
    algorithm: &SigningAlgorithm,
    key: &[u8],
    signature: &[u8],
    data: &[u8],
) -> Result<bool> {
    Ok(match algorithm {
        SigningAlgorithm::Ecdsa { hash } => {
            let (fixed_signing, fixed) = match hash {
                ShaAlgorithm::SHA256 => (
                    &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                    &ring::signature::ECDSA_P256_SHA256_FIXED,
                ),
                ShaAlgorithm::SHA384 => (
                    &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
                    &ring::signature::ECDSA_P384_SHA384_FIXED,
                ),
                _ => {
                    return Err(Exception::throw_message(
                        ctx,
                        "Ecdsa.hash only support Sha256 or Sha384",
                    ))
                },
            };

            let rng = &(*SYSTEM_RANDOM);

            let private_key = EcdsaKeyPair::from_pkcs8(fixed_signing, key, rng).or_throw(ctx)?;
            let public_key_bytes = private_key.public_key().as_ref();
            let public_key = UnparsedPublicKey::new(fixed, &public_key_bytes);

            public_key.verify(data, signature).is_ok()
        },
        SigningAlgorithm::Ed25519 => {
            let private_key = Ed25519KeyPair::from_pkcs8(key).or_throw(ctx)?;
            let public_key_bytes = private_key.public_key().as_ref();
            let public_key = UnparsedPublicKey::new(&ring::signature::ED25519, public_key_bytes);

            public_key.verify(data, signature).is_ok()
        },
        SigningAlgorithm::Hmac => {
            let key = HmacKey::new(ring::hmac::HMAC_SHA256, key);
            let mut hmac = HmacContext::with_key(&key);
            hmac.update(data);

            hmac.sign().as_ref() == signature
        },
        SigningAlgorithm::RsaPss { salt_length } => {
            let public_key = RsaPrivateKey::from_pkcs1_der(key)
                .or_throw(ctx)?
                .to_public_key();
            let mut hasher = Sha256::new();
            hasher.update(data);
            let hashed = hasher.finalize();

            public_key
                .verify(
                    Pss::new_with_salt::<Sha256>(*salt_length as usize),
                    &hashed,
                    signature,
                )
                .is_ok()
        },
        SigningAlgorithm::RsassaPkcs1v15 => {
            let public_key = RsaPrivateKey::from_pkcs1_der(key)
                .or_throw(ctx)?
                .to_public_key();
            let mut hasher = Sha256::new();
            hasher.update(data);

            let hashed = hasher.finalize();

            public_key
                .verify(Pkcs1v15Sign::new::<Sha256>(), &hashed, signature)
                .is_ok()
        },
    })
}
