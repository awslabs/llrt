// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Result, Value};

use crate::{sha_hash::ShaAlgorithm, provider::{CryptoProvider, SimpleDigest}, CRYPTO_PROVIDER};

pub async fn subtle_digest<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let algorithm = if let Some(algorithm) = algorithm.as_string() {
        algorithm.to_string().or_throw(&ctx)?
    } else {
        algorithm.get_required::<_, String>("name", "algorithm")?
    };

    let sha_algorithm = ShaAlgorithm::try_from(algorithm.as_str()).or_throw(&ctx)?;
    let bytes = digest(&sha_algorithm, data.as_bytes(&ctx)?);
    ArrayBuffer::new(ctx, bytes)
}

pub fn digest(sha_algorithm: &ShaAlgorithm, data: &[u8]) -> Vec<u8> {
    let mut hasher = CRYPTO_PROVIDER.digest(*sha_algorithm);
    hasher.update(data);
    hasher.finalize()
}
