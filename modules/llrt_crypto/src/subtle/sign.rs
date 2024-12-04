// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rand::rngs::OsRng;
use ring::{
    hmac::{Context as HmacContext, Key as HmacKey},
    signature::{EcdsaKeyPair, Ed25519KeyPair},
};
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    pss::Pss,
    sha2::{Digest, Sha256},
    Pkcs1v15Sign, RsaPrivateKey,
};

use crate::{
    subtle::{check_supported_usage, extract_sign_verify_algorithm, Algorithm, CryptoKey, Hash},
    SYSTEM_RANDOM,
};

pub async fn subtle_sign<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    key: CryptoKey<'js>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    check_supported_usage(&ctx, &key.usages(), "sign")?;

    let algorithm = extract_sign_verify_algorithm(&ctx, &algorithm)?;

    let bytes = sign(&ctx, &algorithm, key.get_handle(), data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn sign(ctx: &Ctx<'_>, algorithm: &Algorithm, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::Ecdsa { hash } => {
            let hash = match hash {
                Hash::Sha256 => &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                Hash::Sha384 => &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
                _ => {
                    return Err(Exception::throw_message(
                        ctx,
                        "Ecdsa.hash only support Sha256 or Sha384",
                    ))
                },
            };
            let key_pair =
                EcdsaKeyPair::from_pkcs8(hash, key, &SYSTEM_RANDOM.to_owned()).or_throw(ctx)?;
            let signature = key_pair
                .sign(&SYSTEM_RANDOM.to_owned(), data)
                .or_throw(ctx)?;

            Ok(signature.as_ref().to_vec())
        },
        Algorithm::Ed25519 => {
            let key_pair = Ed25519KeyPair::from_pkcs8(key).or_throw(ctx)?;
            let signature = key_pair.sign(data);

            Ok(signature.as_ref().to_vec())
        },
        Algorithm::Hmac => {
            let key = HmacKey::new(ring::hmac::HMAC_SHA256, key);
            let mut hmac = HmacContext::with_key(&key);
            hmac.update(data);

            Ok(hmac.sign().as_ref().to_vec())
        },
        Algorithm::RsaPss { salt_length } => {
            let private_key = RsaPrivateKey::from_pkcs1_der(key).or_throw(ctx)?;
            let mut rng = OsRng;
            let mut hasher = Sha256::new();
            hasher.update(data);
            let hashed = hasher.finalize();

            Ok(private_key
                .sign_with_rng(
                    &mut rng,
                    Pss::new_with_salt::<Sha256>(*salt_length as usize),
                    &hashed,
                )
                .or_throw(ctx)?)
        },
        Algorithm::RsassaPkcs1v15 => {
            let private_key = RsaPrivateKey::from_pkcs1_der(key).or_throw(ctx)?;
            let mut hasher = Sha256::new();
            hasher.update(data);
            let hashed = hasher.finalize();

            Ok(private_key
                .sign(Pkcs1v15Sign::new::<Sha256>(), &hashed)
                .or_throw(ctx)?)
        },
        _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
    }
}
