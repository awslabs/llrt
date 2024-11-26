// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::num::NonZeroU32;

use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use p256::pkcs8::DecodePrivateKey;
use ring::{hkdf, pbkdf2};
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};

use crate::subtle::{extract_derive_algorithm, CryptoNamedCurve, DeriveAlgorithm, Sha};

struct HkdfOutput(usize);

impl hkdf::KeyType for HkdfOutput {
    fn len(&self) -> usize {
        self.0
    }
}

pub async fn subtle_derive_bits<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    base_key: ObjectBytes<'js>,
    length: u32,
) -> Result<ArrayBuffer<'js>> {
    let derive_algorithm = extract_derive_algorithm(&ctx, &algorithm)?;

    let bytes = derive_bits(&ctx, &derive_algorithm, base_key.as_bytes(), length)?;
    ArrayBuffer::new(ctx, bytes)
}

fn derive_bits(
    ctx: &Ctx<'_>,
    algorithm: &DeriveAlgorithm,
    base_key: &[u8],
    length: u32,
) -> Result<Vec<u8>> {
    match algorithm {
        DeriveAlgorithm::Edch { curve, public } => match curve {
            CryptoNamedCurve::P256 => {
                let secret_key = p256::SecretKey::from_pkcs8_der(base_key).or_throw(ctx)?;

                let public_key = p256::SecretKey::from_pkcs8_der(public)
                    .or_throw(ctx)?
                    .public_key();

                let shared_secret = p256::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );

                Ok(shared_secret.raw_secret_bytes().to_vec())
            },
            CryptoNamedCurve::P384 => {
                let secret_key = p384::SecretKey::from_pkcs8_der(base_key).or_throw(ctx)?;

                let public_key = p384::SecretKey::from_pkcs8_der(public)
                    .or_throw(ctx)?
                    .public_key();

                let shared_secret = p384::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );

                Ok(shared_secret.raw_secret_bytes().to_vec())
            },
        },
        DeriveAlgorithm::Pbkdf2 {
            hash,
            ref salt,
            iterations,
        } => {
            let hash_algorithm = match hash {
                Sha::Sha1 => pbkdf2::PBKDF2_HMAC_SHA1,
                Sha::Sha256 => pbkdf2::PBKDF2_HMAC_SHA256,
                Sha::Sha384 => pbkdf2::PBKDF2_HMAC_SHA384,
                Sha::Sha512 => pbkdf2::PBKDF2_HMAC_SHA512,
            };

            let mut out = vec![0; (length / 8).try_into().or_throw(ctx)?];
            let not_zero_iterations = NonZeroU32::new(*iterations)
                .ok_or_else(|| Exception::throw_message(ctx, "Iterations not zero"))?;

            pbkdf2::derive(
                hash_algorithm,
                not_zero_iterations,
                salt,
                base_key,
                &mut out,
            );

            Ok(out)
        },
        DeriveAlgorithm::Hkdf {
            hash,
            ref salt,
            info,
        } => {
            let hash_algorithm = match hash {
                Sha::Sha1 => hkdf::HKDF_SHA1_FOR_LEGACY_USE_ONLY,
                Sha::Sha256 => hkdf::HKDF_SHA256,
                Sha::Sha384 => hkdf::HKDF_SHA384,
                Sha::Sha512 => hkdf::HKDF_SHA512,
            };

            let salt = hkdf::Salt::new(hash_algorithm, salt);
            let info: &[&[u8]] = &[&info[..]];

            let prk = salt.extract(base_key);
            let out_length = (length / 8).try_into().or_throw(ctx)?;

            let okm = prk
                .expand(info, HkdfOutput((length / 8).try_into().or_throw(ctx)?))
                .or_throw(ctx)?;

            let mut out = vec![0u8; out_length];
            let _ = okm.fill(&mut out).or_throw(ctx);

            Ok(out)
        },
    }
}
