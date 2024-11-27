// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::OnceLock;

use llrt_utils::{object::ObjectExt, result::ResultExt};
use num_traits::FromPrimitive;
use ring::{rand::SecureRandom, signature::EcdsaKeyPair};
use rquickjs::{Array, Ctx, Exception, IntoJs, Object, Result, Value};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::{rand_core::OsRng, BigUint, RsaPrivateKey};

use crate::{
    subtle::{extract_sha_hash, CryptoNamedCurve, KeyGenAlgorithm, Sha},
    SYSTEM_RANDOM,
};

use super::crypto_key::CryptoKey;

static PUB_EXPONENT_1: OnceLock<BigUint> = OnceLock::new();
static PUB_EXPONENT_2: OnceLock<BigUint> = OnceLock::new();

pub async fn subtle_generate_key<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Value<'js>> {
    let (name, key_gen_algorithm) = extract_generate_key_algorithm(&ctx, &algorithm)?;

    let bytes = generate_key(&ctx, &key_gen_algorithm)?;

    if name.starts_with("AES") || name == "HMAC" {
        CryptoKey::new(
            ctx.clone(),
            "secret".to_string(),
            extractable,
            algorithm,
            key_usages, // for test
            bytes,      // for test
        )
        .into_js(&ctx)
    } else {
        let private = CryptoKey::new(
            ctx.clone(),
            "private".to_string(),
            extractable,
            algorithm.clone(),
            key_usages.clone(), // for test
            bytes.clone(),      // for test
        )?;
        let public = CryptoKey::new(
            ctx.clone(),
            "public".to_string(),
            true,
            algorithm,
            key_usages, // for test
            bytes,      // for test
        )?;
        let obj = Object::new(ctx)?;
        obj.set("privateKey", private)?;
        obj.set("publicKey", public)?;
        Ok(obj.into())
    }
}

fn extract_generate_key_algorithm(
    ctx: &Ctx<'_>,
    algorithm: &Value,
) -> Result<(String, KeyGenAlgorithm)> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_message(ctx, "Algorithm name not found"))?;

    match name.as_str() {
        "RSASSA-PKCS1-v1_5" | "RSA-PSS" | "RSA-OAEP" => {
            let modulus_length = algorithm.get_optional("modulusLength")?.ok_or_else(|| {
                Exception::throw_message(ctx, "Algorithm modulusLength not found")
            })?;

            let public_exponent = algorithm.get_optional("publicExponent")?.ok_or_else(|| {
                Exception::throw_message(ctx, "Algorithm publicExponent not found")
            })?;

            Ok((name, KeyGenAlgorithm::Rsa { modulus_length, public_exponent }))
        },
        "ECDSA" | "ECDH" => {
            let named_curve = algorithm
                .get_optional::<_, String>("namedCurve")?
                .ok_or_else(|| Exception::throw_message(ctx, "Algorithm namedCurve not found"))?;

            let curve = CryptoNamedCurve::try_from(named_curve.as_str()).or_throw(ctx)?;

            Ok((name, KeyGenAlgorithm::Ec { curve }))
        },
        "HMAC" => {
            let hash = extract_sha_hash(ctx, algorithm)?;

            let length = algorithm.get_optional::<_, u32>("length")?;

            Ok((name, KeyGenAlgorithm::Hmac { hash, length }))
        },
        "AES-CTR" | "AES-CBC" | "AES-GCM" | "AES-KW" => {
            let length = algorithm
                .get_optional("length")?
                .ok_or_else(|| Exception::throw_message(ctx, "Algorithm length not found"))?;

            if length != 128 && length != 192 && length != 256 {
                return Err(Exception::throw_message(
                    ctx,
                    "Algorithm length must be one of: 128, 192, or 256.",
                ));
            }

            Ok((name, KeyGenAlgorithm::Aes { length }))
        },
        _ => Err(Exception::throw_message(
            ctx,
            "Algorithm must be RsaHashedKeyGenParams | EcKeyGenParams | HmacKeyGenParams | AesKeyGenParams",
        )),
    }
}

fn generate_key(ctx: &Ctx<'_>, algorithm: &KeyGenAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        KeyGenAlgorithm::Rsa {
            modulus_length,
            ref public_exponent,
        } => {
            let exponent = BigUint::from_bytes_be(public_exponent);

            if exponent != *PUB_EXPONENT_1.get_or_init(|| BigUint::from_u64(3).unwrap())
                && exponent != *PUB_EXPONENT_2.get_or_init(|| BigUint::from_u64(65537).unwrap())
            {
                return Err(Exception::throw_message(ctx, "Bad public exponent"));
            }

            let mut rng = OsRng;

            let private_key =
                RsaPrivateKey::new_with_exp(&mut rng, *modulus_length as usize, &exponent)
                    .or_throw(ctx)?;

            let private_key = private_key.to_pkcs1_der().or_throw(ctx)?;

            Ok(private_key.as_bytes().to_vec())
        },
        KeyGenAlgorithm::Ec { curve } => {
            let curve = match curve {
                CryptoNamedCurve::P256 => &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                CryptoNamedCurve::P384 => &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
            };
            let pkcs8 =
                EcdsaKeyPair::generate_pkcs8(curve, &SYSTEM_RANDOM.to_owned()).or_throw(ctx)?;

            Ok(pkcs8.as_ref().to_vec())
        },
        KeyGenAlgorithm::Aes { length } => {
            let length = *length as usize;

            if length % 8 != 0 || length > 256 {
                return Err(Exception::throw_message(ctx, "Invalid AES key length"));
            }

            let mut key = vec![0u8; length / 8];
            SYSTEM_RANDOM.fill(&mut key).or_throw(ctx)?;

            Ok(key)
        },
        KeyGenAlgorithm::Hmac { hash, length } => {
            let hash = match hash {
                Sha::Sha1 => &ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
                Sha::Sha256 => &ring::hmac::HMAC_SHA256,
                Sha::Sha384 => &ring::hmac::HMAC_SHA384,
                Sha::Sha512 => &ring::hmac::HMAC_SHA512,
            };

            let length = if let Some(length) = length {
                if length % 8 != 0 {
                    return Err(Exception::throw_message(ctx, "Invalid HMAC key length"));
                }

                let length = length / 8;

                if length > ring::digest::MAX_BLOCK_LEN.try_into().unwrap() {
                    return Err(Exception::throw_message(ctx, "Invalid HMAC key length"));
                }

                length as usize
            } else {
                hash.digest_algorithm().block_len()
            };

            let mut key = vec![0u8; length];
            SYSTEM_RANDOM.fill(&mut key).or_throw(ctx)?;

            Ok(key)
        },
    }
}
