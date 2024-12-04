// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::num::NonZeroU32;

use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use p256::pkcs8::DecodePrivateKey;
use ring::{hkdf, pbkdf2};
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};

use crate::subtle::{
    check_supported_usage, extract_sha_hash, CryptoKey, DeriveAlgorithm, EllipticCurve, Hash,
};

struct HkdfOutput(usize);

impl hkdf::KeyType for HkdfOutput {
    fn len(&self) -> usize {
        self.0
    }
}

pub async fn subtle_derive_bits<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    base_key: CryptoKey<'js>,
    length: u32,
) -> Result<ArrayBuffer<'js>> {
    check_supported_usage(&ctx, &base_key.usages(), "deriveBits")?;

    let derive_algorithm = extract_derive_algorithm(&ctx, &algorithm)?;

    let bytes = derive_bits(&ctx, &derive_algorithm, base_key.get_handle(), length)?;
    ArrayBuffer::new(ctx, bytes)
}

fn extract_derive_algorithm(ctx: &Ctx<'_>, algorithm: &Value) -> Result<DeriveAlgorithm> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'name' property required"))?;

    match name.as_str() {
        "ECDH" => {
            let public = algorithm
                .get_optional::<_, CryptoKey>("public")?
                .ok_or_else(|| {
                    Exception::throw_type(ctx, "algorithm 'public' property required")
                })?;

            let namedcurve = public
                .algorithm()
                .get_optional::<_, String>("namedCurve")?
                .ok_or_else(|| {
                    Exception::throw_type(ctx, "algorithm 'namedCurve' property required")
                })?;
            let curve = EllipticCurve::try_from(namedcurve.as_str()).or_throw(ctx)?;
            let public = public.get_handle().to_vec();

            Ok(DeriveAlgorithm::Edch { curve, public })
        },
        "HKDF" => {
            let hash = extract_sha_hash(ctx, algorithm)?;

            let salt = algorithm
                .get_optional::<_, ObjectBytes>("salt")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'salt' property required"))?
                .into_bytes();

            let info = algorithm
                .get_optional::<_, ObjectBytes>("info")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'info' property required"))?
                .into_bytes();

            Ok(DeriveAlgorithm::Hkdf { hash, salt, info })
        },
        "PBKDF2" => {
            let hash = extract_sha_hash(ctx, algorithm)?;

            let salt = algorithm
                .get_optional::<_, ObjectBytes>("salt")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'salt' property required"))?
                .into_bytes();

            let iterations = algorithm.get_optional("iterations")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'iterations' property required")
            })?;

            Ok(DeriveAlgorithm::Pbkdf2 {
                hash,
                salt,
                iterations,
            })
        },
        _ => Err(Exception::throw_message(
            ctx,
            "Algorithm 'name' must be ECDH | HKDF | PBKDF2",
        )),
    }
}

fn derive_bits(
    ctx: &Ctx<'_>,
    algorithm: &DeriveAlgorithm,
    base_key: &[u8],
    length: u32,
) -> Result<Vec<u8>> {
    match algorithm {
        DeriveAlgorithm::Edch { curve, public } => match curve {
            EllipticCurve::P256 => {
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
            EllipticCurve::P384 => {
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
        DeriveAlgorithm::Hkdf {
            hash,
            ref salt,
            info,
        } => {
            let hash_algorithm = match hash {
                Hash::Sha1 => hkdf::HKDF_SHA1_FOR_LEGACY_USE_ONLY,
                Hash::Sha256 => hkdf::HKDF_SHA256,
                Hash::Sha384 => hkdf::HKDF_SHA384,
                Hash::Sha512 => hkdf::HKDF_SHA512,
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
        DeriveAlgorithm::Pbkdf2 {
            hash,
            ref salt,
            iterations,
        } => {
            let hash_algorithm = match hash {
                Hash::Sha1 => pbkdf2::PBKDF2_HMAC_SHA1,
                Hash::Sha256 => pbkdf2::PBKDF2_HMAC_SHA256,
                Hash::Sha384 => pbkdf2::PBKDF2_HMAC_SHA384,
                Hash::Sha512 => pbkdf2::PBKDF2_HMAC_SHA512,
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
    }
}
