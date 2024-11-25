// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use hmac::Mac;
use llrt_utils::result::ResultExt;
use rand::rngs::OsRng;
use ring::{rand::SystemRandom, signature::EcdsaKeyPair};
use rquickjs::{Ctx, Exception, Result};
use rsa::{
    pkcs1::DecodeRsaPrivateKey,
    pss::Pss,
    sha2::{Digest, Sha256},
};
use rsa::{Pkcs1v15Sign, RsaPrivateKey};

use crate::subtle::{Algorithm, HmacSha256, Sha};

pub fn sign(ctx: &Ctx<'_>, algorithm: &Algorithm, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    match algorithm {
        Algorithm::Hmac => {
            let mut mac = HmacSha256::new_from_slice(key).or_throw(ctx)?;
            mac.update(data);

            Ok(mac.finalize().into_bytes().to_vec())
        },
        Algorithm::RsassaPkcs1v15 => {
            let private_key = RsaPrivateKey::from_pkcs1_der(key).or_throw(ctx)?;
            let mut hasher = Sha256::new();
            hasher.update(data);

            let hashed = hasher.finalize()[..].to_vec();

            Ok(private_key
                .sign(Pkcs1v15Sign::new::<Sha256>(), &hashed)
                .or_throw(ctx)?)
        },
        Algorithm::RsaPss(salt_length) => {
            let private_key = RsaPrivateKey::from_pkcs1_der(key).or_throw(ctx)?;
            let mut rng = OsRng;
            let mut hasher = Sha256::new();
            hasher.update(data);

            let hashed = hasher.finalize()[..].to_vec();

            Ok(private_key
                .sign_with_rng(
                    &mut rng,
                    Pss::new_with_salt::<Sha256>(*salt_length as usize),
                    &hashed,
                )
                .or_throw(ctx)?)
        },
        Algorithm::Ecdsa(sha) => match sha {
            Sha::Sha256 => {
                let curve = &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING;
                let rng = SystemRandom::new();
                let key_pair = EcdsaKeyPair::from_pkcs8(curve, key, &rng).or_throw(ctx)?;

                let signature = key_pair.sign(&rng, data).or_throw(ctx)?;

                Ok(signature.as_ref().to_vec())
            },
            Sha::Sha384 => {
                let curve = &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING;
                let rng = SystemRandom::new();
                let key_pair = EcdsaKeyPair::from_pkcs8(curve, key, &rng).or_throw(ctx)?;

                let signature = key_pair.sign(&rng, data).or_throw(ctx)?;

                Ok(signature.as_ref().to_vec())
            },
            _ => Err(Exception::throw_message(
                ctx,
                "Ecdsa.hash only support Sha256 or Sha384",
            )),
        },
        _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
    }
}
