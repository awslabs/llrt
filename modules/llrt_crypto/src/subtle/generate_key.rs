// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::result::ResultExt;
use ring::{
    rand::SecureRandom,
    signature::{EcdsaKeyPair, Ed25519KeyPair, KeyPair},
};
use rquickjs::{object::Property, Array, Class, Ctx, Exception, Object, Result, Value};
use rsa::{
    pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey},
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
    BoxedUint, RsaPrivateKey,
};

use crate::{sha_hash::ShaAlgorithm, CryptoKey, SYSTEM_RANDOM};

use super::{
    algorithm_not_supported_error,
    crypto_key::KeyKind,
    key_algorithm::{KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages},
    EllipticCurve,
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

    let (private_key, public_or_secret_key) = generate_key(&ctx, &key_algorithm)?;

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
                public_or_secret_key,
            ),
        )?
        .into_value());
    }

    let private_key = Class::instance(
        ctx.clone(),
        CryptoKey::new(
            KeyKind::Private,
            name.clone(),
            extractable,
            key_algorithm.clone(),
            private_usages,
            private_key,
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
            public_or_secret_key,
        ),
    )?;

    let key_pair = Object::new(ctx.clone())?;
    key_pair.prop("privateKey", Property::from(private_key).enumerable())?;
    key_pair.prop("publicKey", Property::from(public_key).enumerable())?;
    Ok(key_pair.into_value())
}

fn generate_key(ctx: &Ctx<'_>, algorithm: &KeyAlgorithm) -> Result<(Vec<u8>, Vec<u8>)> {
    let private_key;
    let public_or_secret_key;
    match algorithm {
        KeyAlgorithm::Aes { length } => {
            let length = *length as usize;

            match length {
                128 | 192 | 256 => (),
                _ => {
                    return Err(Exception::throw_message(
                        ctx,
                        "AES key length must be 128, 192, or 256 bits",
                    ))
                },
            }

            public_or_secret_key = generate_symmetric_key(ctx, length / 8)?;
            private_key = vec![];
        },
        KeyAlgorithm::Hmac { hash, length } => {
            let length = get_hash_length(ctx, hash, *length)?;
            public_or_secret_key = generate_symmetric_key(ctx, length)?;
            private_key = vec![];
        },
        KeyAlgorithm::Ec { curve, .. } => {
            let rng = &(*SYSTEM_RANDOM);

            match curve {
                EllipticCurve::P256 => {
                    let pkcs8 = EcdsaKeyPair::generate_pkcs8(
                        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                        rng,
                    )
                    .or_throw(ctx)?;
                    private_key = pkcs8.as_ref().into();
                    let signing_key = p256::SecretKey::from_pkcs8_der(&private_key).unwrap();
                    public_or_secret_key = signing_key.public_key().to_sec1_bytes().into();
                },
                EllipticCurve::P384 => {
                    let pkcs8 = EcdsaKeyPair::generate_pkcs8(
                        &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
                        rng,
                    )
                    .or_throw(ctx)?;
                    private_key = pkcs8.as_ref().into();
                    let signing_key = p384::SecretKey::from_pkcs8_der(&private_key).unwrap();
                    public_or_secret_key = signing_key.public_key().to_sec1_bytes().into();
                },
                EllipticCurve::P521 => {
                    let mut rng = rand::rng();
                    let key = p521::SecretKey::random(&mut rng);
                    let pkcs8 = key.to_pkcs8_der().or_throw(ctx)?;
                    private_key = pkcs8.as_bytes().into();
                    public_or_secret_key = key.public_key().to_sec1_bytes().into();
                },
            }
        },
        KeyAlgorithm::Ed25519 => {
            let rng = &(*SYSTEM_RANDOM);
            let pkcs8 = Ed25519KeyPair::generate_pkcs8(rng).or_throw(ctx)?;
            private_key = pkcs8.as_ref().into();

            let key_pair = Ed25519KeyPair::from_pkcs8(&private_key).unwrap();
            public_or_secret_key = key_pair.public_key().as_ref().into();
        },

        KeyAlgorithm::X25519 => {
            let secret_key = x25519_dalek::StaticSecret::random();
            private_key = secret_key.as_bytes().into();
            public_or_secret_key = x25519_dalek::PublicKey::from(&secret_key).as_bytes().into();
        },
        KeyAlgorithm::Rsa {
            modulus_length,
            public_exponent,
            ..
        } => {
            let public_exponent = public_exponent.as_ref().as_ref();
            // Convert public exponent bytes to u64 value
            let exponent: u64 = match public_exponent {
                [0x01, 0x00, 0x01] => 65537, // Standard RSA exponent F4 (0x10001)
                [0x03] => 3,                 // Alternative RSA exponent 3
                bytes
                    if bytes.ends_with(&[0x03])
                        && bytes[..bytes.len() - 1].iter().all(|&b| b == 0) =>
                {
                    3
                },
                _ => return Err(Exception::throw_message(ctx, "Invalid RSA public exponent")),
            };
            let exp = BoxedUint::from(exponent);
            let mut rng = rand::rng();
            let rsa_private_key =
                RsaPrivateKey::new_with_exp(&mut rng, *modulus_length as usize, exp)
                    .or_throw(ctx)?;

            let public_key = rsa_private_key
                .to_public_key()
                .to_pkcs1_der()
                .or_throw(ctx)?;

            let pkcs1 = rsa_private_key.to_pkcs1_der().or_throw(ctx)?;

            private_key = pkcs1.as_bytes().into();

            public_or_secret_key = public_key.as_bytes().into();
        },
        _ => return algorithm_not_supported_error(ctx),
    };
    Ok((private_key, public_or_secret_key))
}

fn generate_symmetric_key(ctx: &Ctx<'_>, length: usize) -> Result<Vec<u8>> {
    let mut key = vec![0u8; length];
    SYSTEM_RANDOM.fill(&mut key).or_throw(ctx)?;
    Ok(key)
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
