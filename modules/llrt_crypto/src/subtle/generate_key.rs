// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::OnceLock;

use num_traits::FromPrimitive;
use ring::{
    rand::{SecureRandom, SystemRandom},
    signature::EcdsaKeyPair,
};
use rquickjs::{Ctx, Exception, Result};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::{rand_core::OsRng, BigUint, RsaPrivateKey};

use crate::subtle::{CryptoNamedCurve, KeyGenAlgorithm, Sha};

static PUB_EXPONENT_1: OnceLock<BigUint> = OnceLock::new();
static PUB_EXPONENT_2: OnceLock<BigUint> = OnceLock::new();

pub fn generate_key(ctx: &Ctx<'_>, algorithm: &KeyGenAlgorithm) -> Result<Vec<u8>> {
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
                    .map_err(|_| Exception::throw_message(ctx, "Failed to generate RSA key"))?;

            let private_key = private_key
                .to_pkcs1_der()
                .map_err(|_| Exception::throw_message(ctx, "Failed to serialize RSA key"))?;

            Ok(private_key.as_bytes().to_vec())
        },
        KeyGenAlgorithm::Ec { curve } => {
            let curve = match curve {
                CryptoNamedCurve::P256 => &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                CryptoNamedCurve::P384 => &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
            };
            let rng = SystemRandom::new();
            let pkcs8 = EcdsaKeyPair::generate_pkcs8(curve, &rng)
                .map_err(|_| Exception::throw_message(ctx, "Failed to generate EC key"))?;

            Ok(pkcs8.as_ref().to_vec())
        },
        KeyGenAlgorithm::Aes { length } => {
            let length = *length as usize;

            if length % 8 != 0 || length > 256 {
                return Err(Exception::throw_message(ctx, "Invalid AES key length"));
            }

            let mut key = vec![0u8; length / 8];
            let rng = SystemRandom::new();
            rng.fill(&mut key)
                .map_err(|_| Exception::throw_message(ctx, "Failed to generate key"))?;

            Ok(key)
        },
        KeyGenAlgorithm::Hmac { hash, length } => {
            let _hash = match hash {
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
                //hash.digest_algorithm().block_len
                ring::digest::MAX_BLOCK_LEN
            };

            let rng = ring::rand::SystemRandom::new();
            let mut key = vec![0u8; length];
            rng.fill(&mut key)
                .map_err(|_| Exception::throw_message(ctx, "Failed to generate key"))?;

            Ok(key)
        },
    }
}
