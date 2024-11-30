// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use ring::{
    hmac::{Context as HmacContext, Key as HmacKey},
    signature::{EcdsaKeyPair, KeyPair},
};
use rquickjs::{Ctx, Exception, Result, Value};
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    pkcs1v15::Pkcs1v15Sign,
    pss::Pss,
    sha2::{Digest, Sha256},
    RsaPrivateKey,
};

use crate::{
    subtle::{check_supported_usage, extract_sign_verify_algorithm, Algorithm, CryptoKey, Sha},
    SYSTEM_RANDOM,
};

pub async fn subtle_verify<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: CryptoKey<'js>,
    signature: ObjectBytes<'js>,
    data: ObjectBytes<'js>,
) -> Result<bool> {
    check_supported_usage(&ctx, &key.usages(), "verify")?;

    let algorithm = extract_sign_verify_algorithm(&ctx, &algorithm)?;

    verify(
        &ctx,
        &algorithm,
        key.get_handle(),
        signature.as_bytes(),
        data.as_bytes(),
    )
}

fn verify(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key: &[u8],
    signature: &[u8],
    data: &[u8],
) -> Result<bool> {
    match algorithm {
        Algorithm::Ecdsa(sha) => {
            let (fixed_string, fixed) = match sha {
                Sha::Sha256 => (
                    &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                    &ring::signature::ECDSA_P256_SHA256_FIXED,
                ),
                Sha::Sha384 => (
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
            let private_key =
                EcdsaKeyPair::from_pkcs8(fixed_string, key, &SYSTEM_RANDOM.to_owned())
                    .or_throw(ctx)?;
            let public_key_bytes = private_key.public_key().as_ref();
            let public_key = ring::signature::UnparsedPublicKey::new(fixed, &public_key_bytes);

            Ok(public_key.verify(data, signature).is_ok())
        },
        Algorithm::Hmac => {
            let key = HmacKey::new(ring::hmac::HMAC_SHA256, key);
            let mut hmac = HmacContext::with_key(&key);
            hmac.update(data);

            Ok(hmac.sign().as_ref() == signature)
        },
        Algorithm::RsaPss(salt_length) => {
            let public_key = RsaPrivateKey::from_pkcs1_der(key)
                .or_throw(ctx)?
                .to_public_key();
            let mut hasher = Sha256::new();
            hasher.update(data);
            let hashed = hasher.finalize();

            Ok(public_key
                .verify(
                    Pss::new_with_salt::<Sha256>(*salt_length as usize),
                    &hashed,
                    signature,
                )
                .is_ok())
        },
        Algorithm::RsassaPkcs1v15 => {
            let public_key = RsaPrivateKey::from_pkcs1_der(key)
                .or_throw(ctx)?
                .to_public_key();
            let mut hasher = Sha256::new();
            hasher.update(data);

            let hashed = hasher.finalize();

            Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha256>(), &hashed, signature)
                .is_ok())
        },
        _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
    }
}
