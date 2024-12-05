// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::HashSet;

use llrt_utils::bytes::ObjectBytes;
use llrt_utils::{object::ObjectExt, result::ResultExt};
use ring::{
    rand::SecureRandom,
    signature::{EcdsaKeyPair, Ed25519KeyPair},
};
use rquickjs::{object::Property, Array, Ctx, Exception, IntoJs, Object, Result, Value};
use rsa::{
    pkcs1::EncodeRsaPrivateKey,
    {rand_core::OsRng, BigUint, RsaPrivateKey},
};

use crate::{
    subtle::{extract_sha_hash, EllipticCurve, Hash, KeyGenAlgorithm},
    CryptoKey, SYSTEM_RANDOM,
};

static SYMMETRIC_USAGES: &[&str] = &["encrypt", "decrypt", "wrapKey", "unwrapKey"];
static SIGNATURE_USAGES: &[&str] = &["sign", "verify"];
static EMPTY_USAGES: &[&str] = &[];
static SIGN_USAGES: &[&str] = &["sign"];
static RSA_OAEP_USAGES: &[&str] = &["decrypt", "unwrapKey"];
static ECDH_USAGES: &[&str] = &["deriveKey", "deriveBits"];
static AES_KW_USAGES: &[&str] = &["wrapKey", "unwrapKey"];

static SUPPORTED_USAGES_ARRAY: &[(&str, &[&str])] = &[
    ("AES-CBC", SYMMETRIC_USAGES),
    ("AES-CTR", SYMMETRIC_USAGES),
    ("AES-GCM", SYMMETRIC_USAGES),
    ("AES-KW", AES_KW_USAGES),
    ("ECDH", ECDH_USAGES),
    ("ECDSA", SIGNATURE_USAGES),
    ("Ed25519", SIGNATURE_USAGES),
    ("HMAC", SIGNATURE_USAGES),
    ("RSA-OAEP", SYMMETRIC_USAGES),
    ("RSA-PSS", SIGNATURE_USAGES),
    ("RSASSA-PKCS1-v1_5", SIGNATURE_USAGES),
];

static MANDATORY_USAGES_ARRAY: &[(&str, &[&str])] = &[
    ("AES-CBC", EMPTY_USAGES),
    ("AES-CTR", EMPTY_USAGES),
    ("AES-GCM", EMPTY_USAGES),
    ("AES-KW", EMPTY_USAGES),
    ("ECDH", ECDH_USAGES),
    ("ECDSA", SIGN_USAGES),
    ("Ed25519", SIGN_USAGES),
    ("HMAC", EMPTY_USAGES),
    ("RSA-OAEP", RSA_OAEP_USAGES),
    ("RSA-PSS", SIGN_USAGES),
    ("RSASSA-PKCS1-v1_5", SIGN_USAGES),
];

pub async fn subtle_generate_key<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Value<'js>> {
    let (name, key_gen_algorithm) = extract_generate_key_algorithm(&ctx, &algorithm)?;

    let (private_usages, public_usages) = classify_and_check_usages(&ctx, &name, &key_usages)?;

    let bytes = generate_key(&ctx, &key_gen_algorithm)?;

    if name.starts_with("AES") || name == "HMAC" {
        CryptoKey::new(
            ctx.clone(),
            "secret".to_string(),
            extractable,
            algorithm,
            public_usages.into_js(&ctx)?.into_array().unwrap(),
            &bytes,
        )
        .into_js(&ctx)
    } else {
        let private_key = CryptoKey::new(
            ctx.clone(),
            "private".to_string(),
            extractable,
            algorithm.clone(),
            private_usages.into_js(&ctx)?.into_array().unwrap(),
            &bytes,
        )?;
        let public_key = CryptoKey::new(
            ctx.clone(),
            "public".to_string(),
            true,
            algorithm,
            public_usages.into_js(&ctx)?.into_array().unwrap(),
            &bytes,
        )?;

        let key_pair = Object::new(ctx.clone())?;
        key_pair.prop("privateKey", Property::from(private_key).enumerable())?;
        key_pair.prop("publicKey", Property::from(public_key).enumerable())?;
        key_pair.into_js(&ctx)
    }
}

fn extract_generate_key_algorithm(
    ctx: &Ctx<'_>,
    algorithm: &Value,
) -> Result<(String, KeyGenAlgorithm)> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'name' property required"))?;

    match name.as_str() {
        "AES-CBC" | "AES-CTR" | "AES-GCM" | "AES-KW" => {
            let length = algorithm
                .get_optional::<_,u32>("length")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'length' property required"))?;

            if ![128, 192 ,256].contains(&length) {
                return Err(Exception::throw_type(
                    ctx,
                    "Algorithm 'length' must be one of: 128, 192, or 256",
                ));
            }

            Ok((name, KeyGenAlgorithm::Aes { length }))
        },
        "ECDH" | "ECDSA" => {
            let named_curve = algorithm
                .get_optional::<_, String>("namedCurve")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'namedCurve' property required"))?;

            let curve = EllipticCurve::try_from(named_curve.as_str()).or_throw(ctx)?;

            Ok((name, KeyGenAlgorithm::Ec { curve }))
        },
        "Ed25519" => {
            Ok((name, KeyGenAlgorithm::Ed25519))
        },
        "HMAC" => {
            let hash = extract_sha_hash(ctx, algorithm)?;

            let length = algorithm.get_optional::<_, u32>("length")?;

            Ok((name, KeyGenAlgorithm::Hmac { hash, length }))
        },
        "RSA-OAEP" | "RSA-PSS" | "RSASSA-PKCS1-v1_5" => {
            let modulus_length = algorithm.get_optional("modulusLength")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'modulusLength' property required")
            })?;

            let public_exponent = algorithm.get_optional::<_, ObjectBytes>("publicExponent")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'publicExponent' property required")
            })?.into_bytes();

            Ok((name, KeyGenAlgorithm::Rsa { modulus_length, public_exponent}))
        },
        _ => Err(Exception::throw_message(
            ctx,
            "Algorithm 'name' must be RsaHashedKeyGenParams | EcKeyGenParams | HmacKeyGenParams | AesKeyGenParams",
        )),
    }
}

fn generate_key(ctx: &Ctx<'_>, algorithm: &KeyGenAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        KeyGenAlgorithm::Aes { length } => {
            let length = *length as usize;
            if length % 8 != 0 || length > 256 {
                return Err(Exception::throw_message(ctx, "Invalid AES key length"));
            }

            let mut key = vec![0u8; length / 8];
            SYSTEM_RANDOM.fill(&mut key).or_throw(ctx)?;

            Ok(key)
        },
        KeyGenAlgorithm::Ec { curve } => {
            let curve = match curve {
                EllipticCurve::P256 => &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                EllipticCurve::P384 => &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
            };
            let pkcs8 =
                EcdsaKeyPair::generate_pkcs8(curve, &SYSTEM_RANDOM.to_owned()).or_throw(ctx)?;

            Ok(pkcs8.as_ref().to_vec())
        },
        KeyGenAlgorithm::Ed25519 => {
            let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SYSTEM_RANDOM.to_owned()).or_throw(ctx)?;

            Ok(pkcs8.as_ref().to_vec())
        },
        KeyGenAlgorithm::Hmac { hash, length } => {
            let hash = match hash {
                Hash::Sha1 => &ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
                Hash::Sha256 => &ring::hmac::HMAC_SHA256,
                Hash::Sha384 => &ring::hmac::HMAC_SHA384,
                Hash::Sha512 => &ring::hmac::HMAC_SHA512,
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
        KeyGenAlgorithm::Rsa {
            modulus_length,
            ref public_exponent,
        } => {
            #[allow(clippy::if_same_then_else)]
            let exponent: u64 = if public_exponent == &[0x01, 0x00, 0x01] {
                65537 // fast pass
            } else if public_exponent == &[0x03] {
                3 // fast pass
            } else if public_exponent.ends_with(&[0x03])
                && public_exponent
                    .iter()
                    .take(public_exponent.len() - 1)
                    .all(|&byte| byte == 0)
            {
                3
            } else {
                return Err(Exception::throw_message(ctx, "Bad public exponent"));
            };

            let mut rng = OsRng;
            let private_key = RsaPrivateKey::new_with_exp(
                &mut rng,
                *modulus_length as usize,
                &BigUint::from(exponent),
            )
            .or_throw(ctx)?;
            let private_key = private_key.to_pkcs1_der().or_throw(ctx)?;

            Ok(private_key.as_bytes().to_vec())
        },
    }
}

fn classify_and_check_usages<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    key_usages: &Array<'js>,
) -> Result<(Vec<String>, Vec<String>)> {
    fn find_usages<'a>(table: &'a [(&str, &[&str])], algorithm: &str) -> Option<&'a [&'a str]> {
        table
            .iter()
            .find(|(name, _)| *name == algorithm)
            .map(|(_, usages)| *usages)
    }

    let mut key_usages_set = HashSet::with_capacity(8);
    for value in key_usages.clone().into_iter() {
        if let Some(string) = value?.as_string() {
            key_usages_set.insert(string.to_string()?);
        }
    }

    let supported_usages = find_usages(SUPPORTED_USAGES_ARRAY, name).unwrap();
    let mandatory_usages = find_usages(MANDATORY_USAGES_ARRAY, name).unwrap();
    let mut private_usages: Vec<String> = Vec::with_capacity(key_usages_set.len());
    let mut public_usages: Vec<String> = Vec::with_capacity(key_usages_set.len());

    for usage in key_usages_set {
        if !supported_usages.contains(&usage.as_str()) {
            return Err(Exception::throw_range(
                ctx,
                "A required parameter was missing or out-of-range",
            ));
        }
        if mandatory_usages.contains(&usage.as_str()) {
            private_usages.push(usage);
        } else {
            public_usages.push(usage);
        }
    }
    if !mandatory_usages.is_empty() && private_usages.is_empty() {
        return Err(Exception::throw_range(
            ctx,
            "A required parameter was missing or out-of-range",
        ));
    }

    Ok((private_usages, public_usages))
}
