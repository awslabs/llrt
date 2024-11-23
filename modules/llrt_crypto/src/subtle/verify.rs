// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use hmac::Mac;
use llrt_utils::result::ResultExt;
use ring::{
    rand::SystemRandom,
    signature::{EcdsaKeyPair, KeyPair},
};
use rquickjs::{Ctx, Exception, Result};
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    pkcs1v15::Pkcs1v15Sign,
    pss::Pss,
    sha2::{Digest, Sha256},
    RsaPrivateKey,
};

use crate::subtle::{Algorithm, HmacSha256, Sha};

pub fn verify(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key_value: Vec<u8>,
    signature: Vec<u8>,
    data: Vec<u8>,
) -> Result<bool> {
    match algorithm {
        Algorithm::Hmac => {
            let mut mac = HmacSha256::new_from_slice(&key_value).or_throw(ctx)?;
            mac.update(&data);

            Ok(mac.verify_slice(&signature).is_ok())
        },
        Algorithm::RsassaPkcs1v15 => {
            let public_key = RsaPrivateKey::from_pkcs1_der(&key_value)
                .or_throw(ctx)?
                .to_public_key();
            let mut hasher = Sha256::new();
            hasher.update(&data);

            let hashed = hasher.finalize()[..].to_vec();

            Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha256>(), &hashed, &signature)
                .is_ok())
        },
        Algorithm::RsaPss(salt_length) => {
            let public_key = RsaPrivateKey::from_pkcs1_der(&key_value)
                .or_throw(ctx)?
                .to_public_key();
            let mut hasher = Sha256::new();

            hasher.update(&data);
            let hashed = hasher.finalize()[..].to_vec();

            Ok(public_key
                .verify(
                    Pss::new_with_salt::<Sha256>(*salt_length as usize),
                    &hashed,
                    &signature,
                )
                .is_ok())
        },
        Algorithm::Ecdsa(sha) => match sha {
            Sha::Sha256 => {
                let rng = SystemRandom::new();
                let private_key = EcdsaKeyPair::from_pkcs8(
                    &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                    &key_value,
                    &rng,
                )
                .or_throw(ctx)?;

                let public_key_bytes = private_key.public_key().as_ref();
                let public_key = ring::signature::UnparsedPublicKey::new(
                    &ring::signature::ECDSA_P256_SHA256_FIXED,
                    &public_key_bytes,
                );

                Ok(public_key.verify(&data, &signature).is_ok())
            },
            Sha::Sha384 => {
                let rng = SystemRandom::new();
                let private_key = EcdsaKeyPair::from_pkcs8(
                    &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
                    &key_value,
                    &rng,
                )
                .or_throw(ctx)?;

                let public_key_bytes = private_key.public_key().as_ref();
                let public_key = ring::signature::UnparsedPublicKey::new(
                    &ring::signature::ECDSA_P384_SHA384_FIXED,
                    &public_key_bytes,
                );

                Ok(public_key.verify(&data, &signature).is_ok())
            },
            _ => Err(Exception::throw_message(
                ctx,
                "Ecdsa.hash only support Sha256 or Sha384",
            )),
        },
        _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
    }
}
