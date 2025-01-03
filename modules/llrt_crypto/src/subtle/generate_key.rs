// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use llrt_utils::result::ResultExt;
use ring::{
    rand::SecureRandom,
    signature::{EcdsaKeyPair, Ed25519KeyPair},
};
use rquickjs::Class;
use rquickjs::{object::Property, Array, Ctx, Exception, Object, Result, Value};
use rsa::{pkcs1::EncodeRsaPrivateKey, rand_core::OsRng, BigUint, RsaPrivateKey};

use crate::{sha_hash::ShaAlgorithm, CryptoKey, SYSTEM_RANDOM};

use super::{
    algorithm_not_supported_error,
    crypto_key::KeyKind,
    key_algorithm::{KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages},
};

pub async fn subtle_generate_key<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Value<'js>> {
    let KeyAlgorithmWithUsages {
        name,
        algorithm: key_algorithm,
        private_usages,
        public_usages,
    } = KeyAlgorithm::from_js(&ctx, KeyAlgorithmMode::Generate, algorithm, key_usages)?;

    let bytes = generate_key(&ctx, &key_algorithm)?;

    if matches!(
        key_algorithm,
        KeyAlgorithm::Aes { .. } | KeyAlgorithm::Hmac { .. }
    ) {
        return Ok(Class::instance(
            ctx,
            CryptoKey::new(
                KeyKind::Secret,
                name,
                extractable,
                key_algorithm,
                public_usages,
                bytes,
            ),
        )?
        .into_value());
    }
    let bytes: Rc<[u8]> = bytes.into();

    let private_key = Class::instance(
        ctx.clone(),
        CryptoKey::new(
            KeyKind::Private,
            name.clone(),
            extractable,
            key_algorithm.clone(),
            private_usages,
            bytes.clone(),
        ),
    )?;

    let public_key = Class::instance(
        ctx.clone(),
        CryptoKey::new(
            KeyKind::Public,
            name,
            extractable,
            key_algorithm,
            public_usages,
            bytes.clone(),
        ),
    )?;

    let key_pair = Object::new(ctx.clone())?;
    key_pair.prop("privateKey", Property::from(private_key).enumerable())?;
    key_pair.prop("publicKey", Property::from(public_key).enumerable())?;
    Ok(key_pair.into_value())
}

fn generate_key(ctx: &Ctx<'_>, algorithm: &KeyAlgorithm) -> Result<Vec<u8>> {
    Ok(match algorithm {
        KeyAlgorithm::Aes { length } => {
            let length = *length as usize;
            if length % 8 != 0 || length > 256 {
                return Err(Exception::throw_message(ctx, "Invalid AES key length"));
            }

            let mut key = vec![0u8; length / 8];
            SYSTEM_RANDOM.fill(&mut key).or_throw(ctx)?;

            key
        },
        KeyAlgorithm::Ec { curve, .. } => {
            let rng = &(*SYSTEM_RANDOM);
            let curve = curve.as_signing_algorithm();
            let pkcs8 = EcdsaKeyPair::generate_pkcs8(curve, rng).or_throw(ctx)?;
            pkcs8.as_ref().to_vec()
        },
        KeyAlgorithm::Ed25519 => {
            let rng = &(*SYSTEM_RANDOM);
            let pkcs8 = Ed25519KeyPair::generate_pkcs8(rng).or_throw(ctx)?;
            pkcs8.as_ref().to_vec()
        },
        KeyAlgorithm::Hmac { hash, length } => {
            let length = get_hash_length(ctx, hash, *length)?;

            let mut key = vec![0u8; length];
            SYSTEM_RANDOM.fill(&mut key).or_throw(ctx)?;

            key
        },
        KeyAlgorithm::X25519 => {
            let secret_key = x25519_dalek::StaticSecret::random();
            let public_key = x25519_dalek::PublicKey::from(&secret_key);

            let secret_key_bytes = secret_key.as_bytes();
            let public_key_bytes = public_key.as_bytes();

            let mut merged = Vec::with_capacity(secret_key_bytes.len() + public_key_bytes.len());
            merged.extend_from_slice(secret_key_bytes);
            merged.extend_from_slice(public_key_bytes);
            merged
        },
        KeyAlgorithm::Rsa {
            modulus_length,
            public_exponent,
            ..
        } => {
            let public_exponent = public_exponent.as_ref().as_ref();
            let exponent: u64 = match public_exponent {
                [0x01, 0x00, 0x01] => 65537, // fast pass
                [0x03] => 3,                 // fast pass
                bytes
                    if bytes.ends_with(&[0x03])
                        && bytes[..bytes.len() - 1].iter().all(|&b| b == 0) =>
                {
                    3
                },
                _ => return Err(Exception::throw_message(ctx, "Bad public exponent")),
            };

            let mut rng = OsRng;
            let exp = BigUint::from(exponent);
            let private_key = RsaPrivateKey::new_with_exp(&mut rng, *modulus_length as usize, &exp)
                .or_throw(ctx)?;
            let pkcs = private_key.to_pkcs1_der().or_throw(ctx)?;
            pkcs.as_bytes().to_vec()
        },
        _ => return algorithm_not_supported_error(ctx),
    })
}

pub fn get_hash_length(ctx: &Ctx, hash: &ShaAlgorithm, length: u16) -> Result<usize> {
    if length == 0 {
        return Ok(hash.hmac_algorithm().digest_algorithm().block_len());
    }

    if length % 8 != 0 || (length / 8) > ring::digest::MAX_BLOCK_LEN.try_into().unwrap() {
        return Err(Exception::throw_message(ctx, "Invalid HMAC key length"));
    }

    Ok((length / 8) as usize)
}
