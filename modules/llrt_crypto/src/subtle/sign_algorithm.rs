// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{object::ObjectExt, result::ResultExt};
use rquickjs::{Ctx, FromJs, Result, Value};

use crate::sha_hash::ShaAlgorithm;

use super::{
    algorithm_not_supported_error, key_algorithm::extract_sha_hash, to_name_and_maybe_object,
};

#[derive(Debug)]
pub enum SigningAlgorithm {
    Ecdsa { hash: ShaAlgorithm },
    Ed25519,
    RsaPss { salt_length: u32 },
    RsassaPkcs1v15,
    Hmac,
}

impl<'js> FromJs<'js> for SigningAlgorithm {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let (name, obj) = to_name_and_maybe_object(ctx, value)?;

        let algorithm = match name.as_str() {
            "Ed25519" => SigningAlgorithm::Ed25519,
            "HMAC" => SigningAlgorithm::Hmac,
            "RSASSA-PKCS1-v1_5" => SigningAlgorithm::RsassaPkcs1v15,
            "ECDSA" => {
                let obj = obj.or_throw(ctx)?;
                let hash = extract_sha_hash(ctx, &obj)?;
                SigningAlgorithm::Ecdsa { hash }
            },
            "RSA-PSS" => {
                let salt_length = obj.or_throw(ctx)?.get_required("saltLength", "algorithm")?;

                SigningAlgorithm::RsaPss { salt_length }
            },
            _ => return algorithm_not_supported_error(ctx),
        };
        Ok(algorithm)
    }
}
