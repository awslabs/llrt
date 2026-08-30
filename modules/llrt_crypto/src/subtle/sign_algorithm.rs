// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{Ctx, FromJs, Result, Value};

use crate::hash::HashAlgorithm;

use super::{
    algorithm_not_supported_error, enforce_range_u32, get_required_dictionary_value,
    key_algorithm::extract_sha_hash, normalize_algorithm_name, to_name_and_maybe_object,
};

#[derive(Debug)]
pub enum SigningAlgorithm {
    Ecdsa { hash: HashAlgorithm },
    Ed25519,
    RsaPss { salt_length: u32 },
    RsassaPkcs1v15,
    Hmac,
}

impl<'js> FromJs<'js> for SigningAlgorithm {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let (name, obj) = to_name_and_maybe_object(ctx, value)?;
        let name = normalize_algorithm_name(&name);

        let algorithm = match name.as_str() {
            "Ed25519" => SigningAlgorithm::Ed25519,
            "HMAC" => SigningAlgorithm::Hmac,
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
        }
    }
}
