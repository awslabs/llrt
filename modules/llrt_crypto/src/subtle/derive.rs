// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::num::NonZeroU32;

use llrt_utils::result::ResultExt;
use p256::pkcs8::DecodePrivateKey;
use ring::{hkdf, pbkdf2};
use rquickjs::{Array, ArrayBuffer, Class, Ctx, Exception, Result, Value};

use super::{
    algorithm_not_supported_error,
    derive_algorithm::DeriveAlgorithm,
    key_algorithm::{KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages, KeyDerivation},
};

use crate::{
    sha_hash::ShaAlgorithm,
    subtle::{CryptoKey, EllipticCurve},
};

struct HkdfOutput(usize);

impl hkdf::KeyType for HkdfOutput {
    fn len(&self) -> usize {
        self.0
    }
}

pub async fn subtle_derive_bits<'js>(
    ctx: Ctx<'js>,
    algorithm: DeriveAlgorithm,
    base_key: Class<'js, CryptoKey>,
    length: u32,
) -> Result<ArrayBuffer<'js>> {
    let base_key = base_key.borrow();
    base_key.check_validity("deriveBits").or_throw(&ctx)?;

    let bytes = derive_bits(&ctx, &algorithm, &base_key.handle, length)?;
    ArrayBuffer::new(ctx, bytes)
}

fn derive_bits(
    ctx: &Ctx<'_>,
    algorithm: &DeriveAlgorithm,
    base_key: &[u8],
    length: u32,
) -> Result<Vec<u8>> {
    Ok(match algorithm {
        DeriveAlgorithm::Ecdh { curve, public } => match curve {
            EllipticCurve::P256 => {
                let secret_key = p256::SecretKey::from_pkcs8_der(base_key).or_throw(ctx)?;
                let public_key = p256::SecretKey::from_pkcs8_der(public)
                    .or_throw(ctx)?
                    .public_key();
                let shared_secret = p256::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );

                shared_secret.raw_secret_bytes().to_vec()
            },
            EllipticCurve::P384 => {
                let secret_key = p384::SecretKey::from_pkcs8_der(base_key).or_throw(ctx)?;
                let public_key = p384::SecretKey::from_pkcs8_der(public)
                    .or_throw(ctx)?
                    .public_key();
                let shared_secret = p384::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );

                shared_secret.raw_secret_bytes().to_vec()
            },
        },
        DeriveAlgorithm::Derive(KeyDerivation::Hkdf { hash, salt, info }) => {
            let hash_algorithm = match hash {
                ShaAlgorithm::SHA1 => hkdf::HKDF_SHA1_FOR_LEGACY_USE_ONLY,
                ShaAlgorithm::SHA256 => hkdf::HKDF_SHA256,
                ShaAlgorithm::SHA384 => hkdf::HKDF_SHA384,
                ShaAlgorithm::SHA512 => hkdf::HKDF_SHA512,
            };
            let salt = hkdf::Salt::new(hash_algorithm, salt);
            let info: &[&[u8]] = &[&info[..]];
            let prk = salt.extract(base_key);
            let out_length = (length / 8).try_into().or_throw(ctx)?;
            let okm = prk
                .expand(info, HkdfOutput((length / 8).try_into().or_throw(ctx)?))
                .or_throw(ctx)?;
            let mut out = vec![0u8; out_length];
            okm.fill(&mut out).or_throw(ctx)?;

            out
        },
        DeriveAlgorithm::Derive(KeyDerivation::Pbkdf2 {
            hash,
            salt,
            iterations,
        }) => {
            let hash_algorithm = match hash {
                ShaAlgorithm::SHA1 => pbkdf2::PBKDF2_HMAC_SHA1,
                ShaAlgorithm::SHA256 => pbkdf2::PBKDF2_HMAC_SHA256,
                ShaAlgorithm::SHA384 => pbkdf2::PBKDF2_HMAC_SHA384,
                ShaAlgorithm::SHA512 => pbkdf2::PBKDF2_HMAC_SHA512,
            };

            let mut out = vec![0; (length / 8).try_into().or_throw(ctx)?];
            let not_zero_iterations = NonZeroU32::new(*iterations)
                .ok_or_else(|| Exception::throw_message(ctx, "Iterations are zero"))?;
            pbkdf2::derive(
                hash_algorithm,
                not_zero_iterations,
                salt,
                base_key,
                &mut out,
            );

            out
        },
    })
}

pub async fn subtle_derive_key<'js>(
    ctx: Ctx<'js>,
    algorithm: DeriveAlgorithm,
    base_key: Class<'js, CryptoKey>,
    derived_key_algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<Class<'js, CryptoKey>> {
    let KeyAlgorithmWithUsages {
        algorithm: derived_key_algorithm,
        name,
        public_usages,
        ..
    } = KeyAlgorithm::from_js(
        &ctx,
        KeyAlgorithmMode::Derive,
        derived_key_algorithm,
        key_usages,
    )?;

    let length: u16 = match &derived_key_algorithm {
        KeyAlgorithm::Aes { length } => *length,
        KeyAlgorithm::Hmac { length, .. } => *length,
        KeyAlgorithm::Derive { .. } => 0,
        _ => {
            return algorithm_not_supported_error(&ctx);
        },
    };

    let handle = &base_key.borrow().handle;

    let bytes = derive_bits(&ctx, &algorithm, handle, length as u32)?;

    let key = CryptoKey::new(
        "secret",
        name,
        extractable,
        derived_key_algorithm,
        public_usages,
        bytes,
    );

    Class::instance(ctx, key)
}
