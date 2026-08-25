// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_exceptions::DOMException;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt};
use rquickjs::{Ctx, FromJs, Result, Value};

use crate::{hash::HashAlgorithm, provider::MlDsaVariant};

use super::{
    algorithm_not_supported_error, enforce_range_u32, get_required_dictionary_value,
    key_algorithm::extract_sha_hash, normalize_algorithm_name, to_name_and_maybe_object,
};

#[derive(Debug)]
pub enum SigningAlgorithm {
    Ecdsa {
        hash: HashAlgorithm,
    },
    Ed25519,
    RsaPss {
        salt_length: u32,
    },
    RsassaPkcs1v15,
    Hmac,
    MlDsa {
        variant: MlDsaVariant,
        context: Box<[u8]>,
    },
}

impl<'js> FromJs<'js> for SigningAlgorithm {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let (name, obj) = to_name_and_maybe_object(ctx, value)?;
        let name = normalize_algorithm_name(&name);

        let algorithm = match name.as_str() {
            "Ed25519" => SigningAlgorithm::Ed25519,
            "HMAC" => SigningAlgorithm::Hmac,
            "ML-DSA-44" | "ML-DSA-65" | "ML-DSA-87" => {
                let variant = match name.as_str() {
                    "ML-DSA-44" => MlDsaVariant::MlDsa44,
                    "ML-DSA-65" => MlDsaVariant::MlDsa65,
                    "ML-DSA-87" => MlDsaVariant::MlDsa87,
                    _ => unreachable!(),
                };
                let context = if let Ok(obj) = obj {
                    obj.get_optional::<_, ObjectBytes>("context")?
                        .map(|value| value.into_bytes(ctx))
                        .transpose()?
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                if context.len() > 255 {
                    return Err(DOMException::operation_error(
                        ctx,
                        "ML-DSA context must not exceed 255 bytes",
                    ));
                }
                SigningAlgorithm::MlDsa {
                    variant,
                    context: context.into_boxed_slice(),
                }
            },
            "RSASSA-PKCS1-v1_5" => SigningAlgorithm::RsassaPkcs1v15,
            "ECDSA" => {
                let obj = obj?;
                let hash = extract_sha_hash(ctx, &obj)?;
                SigningAlgorithm::Ecdsa { hash }
            },
            "RSA-PSS" => {
                let value = get_required_dictionary_value(&obj?, "saltLength", "algorithm")?;
                let salt_length = enforce_range_u32(ctx, value, "saltLength")?;

                SigningAlgorithm::RsaPss { salt_length }
            },
            _ => return algorithm_not_supported_error(ctx),
        };
        Ok(algorithm)
    }
}

impl SigningAlgorithm {
    pub fn name(&self) -> &'static str {
        match self {
            SigningAlgorithm::Ecdsa { .. } => "ECDSA",
            SigningAlgorithm::Ed25519 => "Ed25519",
            SigningAlgorithm::RsaPss { .. } => "RSA-PSS",
            SigningAlgorithm::RsassaPkcs1v15 => "RSASSA-PKCS1-v1_5",
            SigningAlgorithm::Hmac => "HMAC",
            SigningAlgorithm::MlDsa { variant, .. } => variant.name(),
        }
    }
}
