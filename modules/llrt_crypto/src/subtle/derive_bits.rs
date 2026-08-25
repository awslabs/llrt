// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::future::Future;

use llrt_exceptions::DOMException;
use llrt_utils::result::ResultExt;
use rquickjs::{prelude::Opt, ArrayBuffer, Class, Ctx, FromJs, Result, Value};

use crate::{provider::CryptoProvider, CRYPTO_PROVIDER};

use super::{
    algorithm_invalid_access_error, algorithm_mismatch_error,
    crypto_key::{CryptoKey, KeyKind},
    derive_algorithm::DeriveAlgorithm,
    key_algorithm::{EcAlgorithm, KeyAlgorithm, KeyDerivation},
    util::ResultDomExt,
    EllipticCurve,
};

pub fn subtle_derive_bits<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    base_key: Class<'js, CryptoKey<'js>>,
    length: Opt<Value<'js>>,
) -> impl Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    let prepared = DeriveAlgorithm::from_js(&ctx, algorithm);

    async move {
        let algorithm = prepared?;

        let base_key = base_key.borrow();
        base_key.check_validity("deriveBits").or_throw_dom(&ctx)?;

        let length = parse_derive_bits_length(&ctx, length)?;
        let bytes = derive_bits(&ctx, &algorithm, &base_key, length)?;

        ArrayBuffer::new(ctx, bytes)
    }
}

pub(super) fn derive_bits(
    ctx: &Ctx<'_>,
    algorithm: &DeriveAlgorithm,
    base_key: &CryptoKey,
    length: DeriveBitsLength,
) -> Result<Vec<u8>> {
    match algorithm {
        DeriveAlgorithm::Ecdh {
            curve,
            ec_algorithm,
            public_key,
        } => {
            if !matches!(ec_algorithm, EcAlgorithm::Ecdh) {
                return algorithm_invalid_access_error(ctx, "ECDH");
            }
            let length = validate_ecdh_length(ctx, *curve, length)?;
            if let KeyAlgorithm::Ec {
                curve: base_key_curve,
                algorithm,
            } = &base_key.algorithm
            {
                if curve == base_key_curve
                    && base_key.kind == KeyKind::Private
                    && matches!(algorithm, EcAlgorithm::Ecdh)
                {
                    let bytes = CRYPTO_PROVIDER
                        .ecdh_derive_bits(*curve, &base_key.handle, public_key)
                        .or_throw_dom(ctx)?;
                    return truncate_derived_bits(ctx, bytes, length);
                }

                return Err(DOMException::invalid_access_error(
                    ctx,
                    "ECDH curve must be same as baseKey",
                ));
            }
            algorithm_mismatch_error(ctx, "ECDH")
        },
        DeriveAlgorithm::X25519 { public_key } => {
            if !matches!(base_key.algorithm, KeyAlgorithm::X25519) {
                return algorithm_mismatch_error(ctx, "X25519");
            }
            let length = match length {
                DeriveBitsLength::Default => 256,
                DeriveBitsLength::Specified(length) if length <= 256 => length,
                DeriveBitsLength::Specified(_) => {
                    return Err(DOMException::operation_error(ctx, "Invalid length"));
                },
            };
            let bytes = CRYPTO_PROVIDER
                .x25519_derive_bits(&base_key.handle, public_key)
                .or_throw_dom(ctx)?;

            truncate_derived_bits(ctx, bytes, length)
        },
        DeriveAlgorithm::Derive(KeyDerivation::Hkdf { hash, salt, info }) => {
            if !matches!(base_key.algorithm, KeyAlgorithm::HkdfImport) {
                return algorithm_invalid_access_error(ctx, "HKDF");
            }
            let length = match length {
                DeriveBitsLength::Specified(length) if length % 8 == 0 => length,
                _ => {
                    return Err(DOMException::operation_error(ctx, "Invalid length"));
                },
            };
            let out_length = (length / 8).try_into().or_throw(ctx)?;
            CRYPTO_PROVIDER
                .hkdf_derive_key(&base_key.handle, salt, info, out_length, *hash)
                .or_throw(ctx)
        },
        DeriveAlgorithm::Derive(KeyDerivation::Pbkdf2 {
            hash,
            salt,
            iterations,
        }) => {
            if !matches!(base_key.algorithm, KeyAlgorithm::Pbkdf2Import) {
                return algorithm_invalid_access_error(ctx, "PBKDF2");
            }
            let length = match length {
                DeriveBitsLength::Specified(length) if length % 8 == 0 => length,
                _ => {
                    return Err(DOMException::operation_error(ctx, "Invalid length"));
                },
            };
            let out_length = (length / 8).try_into().or_throw(ctx)?;
            CRYPTO_PROVIDER
                .pbkdf2_derive_key(&base_key.handle, salt, *iterations, out_length, *hash)
                .or_throw(ctx)
        },
    }
}

fn truncate_derived_bits(ctx: &Ctx<'_>, mut bytes: Vec<u8>, length: u32) -> Result<Vec<u8>> {
    let max_bits = (bytes.len() * 8) as u32;

    if length > max_bits {
        return Err(DOMException::operation_error(
            ctx,
            "Requested length exceeds derived secret size",
        ));
    }

    let byte_length = length.div_ceil(8) as usize;
    bytes.truncate(byte_length);

    let remainder = (length % 8) as u8;
    if remainder != 0 {
        let mask = 0xff << (8 - remainder);
        if let Some(last) = bytes.last_mut() {
            *last &= mask;
        }
    }

    Ok(bytes)
}

pub(super) enum DeriveBitsLength {
    Default,
    Specified(u32),
}

pub(super) fn validate_ecdh_length(
    ctx: &Ctx<'_>,
    curve: EllipticCurve,
    length: DeriveBitsLength,
) -> Result<u32> {
    let maximum_length = match curve {
        EllipticCurve::P256 => 256,
        EllipticCurve::P384 => 384,
        EllipticCurve::P521 => 528,
    };
    match length {
        DeriveBitsLength::Default => Ok(maximum_length),
        DeriveBitsLength::Specified(length) if length <= maximum_length => Ok(length),
        DeriveBitsLength::Specified(_) => Err(DOMException::operation_error(ctx, "Invalid length")),
    }
}

fn parse_derive_bits_length<'js>(
    ctx: &Ctx<'js>,
    length: Opt<Value<'js>>,
) -> Result<DeriveBitsLength> {
    match length.0 {
        None => Ok(DeriveBitsLength::Default),
        Some(value) if value.is_null() || value.is_undefined() => Ok(DeriveBitsLength::Default),
        Some(value) => {
            let length = u32::from_js(ctx, value)
                .map_err(|_| DOMException::operation_error(ctx, "Invalid length"))?;

            Ok(DeriveBitsLength::Specified(length))
        },
    }
}
