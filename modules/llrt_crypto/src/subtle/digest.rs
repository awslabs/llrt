use std::future::Future;

// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Result, Value};
use sha3::{Digest, Sha3_256, Sha3_384, Sha3_512};

use crate::{
    hash::HashAlgorithm,
    provider::{CryptoProvider, SimpleDigest},
    CRYPTO_PROVIDER,
};

use super::algorithm_not_supported_error;

enum DigestAlgorithm {
    Fixed(HashAlgorithm),
    Sha3_256,
    Sha3_384,
    Sha3_512,
}

pub fn subtle_digest<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    data: ObjectBytes<'js>,
) -> impl Future<Output = Result<ArrayBuffer<'js>>> + 'js {
    // Snapshot inputs synchronously so mutating/detaching the buffer after the call can't affect the result (WPT digest.https.any.js).
    let prepared = prepare_digest(&ctx, algorithm, data);

    async move {
        let (algorithm, input) = prepared?;
        let bytes = match algorithm {
            DigestAlgorithm::Fixed(hash) => digest(&hash, &input),
            DigestAlgorithm::Sha3_256 => Sha3_256::digest(&input).to_vec(),
            DigestAlgorithm::Sha3_384 => Sha3_384::digest(&input).to_vec(),
            DigestAlgorithm::Sha3_512 => Sha3_512::digest(&input).to_vec(),
        };
        ArrayBuffer::new(ctx, bytes)
    }
}

fn prepare_digest<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    data: ObjectBytes<'js>,
) -> Result<(DigestAlgorithm, Vec<u8>)> {
    let algorithm = if let Some(s) = algorithm.as_string() {
        s.to_string().or_throw(ctx)?
    } else if let Some(name) = algorithm.get_optional::<_, String>("name")? {
        name
    } else {
        return Err(rquickjs::Exception::throw_type(
            ctx,
            "Algorithm 'name' property required",
        ));
    };
    let algorithm = algorithm.to_ascii_uppercase();
    let algorithm = match algorithm.as_str() {
        "SHA3-256" => DigestAlgorithm::Sha3_256,
        "SHA3-384" => DigestAlgorithm::Sha3_384,
        "SHA3-512" => DigestAlgorithm::Sha3_512,
        name => match HashAlgorithm::from_strict_str(name) {
            Ok(hash) => DigestAlgorithm::Fixed(hash),
            Err(_) => return algorithm_not_supported_error(ctx),
        },
    };
    let input = data.as_bytes_opt().map(<[u8]>::to_vec).unwrap_or_default();
    Ok((algorithm, input))
}

pub fn digest(hash_algorithm: &HashAlgorithm, data: &[u8]) -> Vec<u8> {
    let mut hasher = CRYPTO_PROVIDER.digest(*hash_algorithm);
    hasher.update(data);
    hasher.finalize()
}
