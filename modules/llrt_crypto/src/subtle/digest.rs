// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use ring::digest::Context;
use rquickjs::{ArrayBuffer, Ctx, Result, Value};

use crate::sha_hash::ShaAlgorithm;

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

    let bytes = digest(&ctx, &algorithm, data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn digest(ctx: &Ctx<'_>, algorithm: &str, data: &[u8]) -> Result<Vec<u8>> {
    let hash = ShaAlgorithm::try_from(algorithm).or_throw(ctx)?;

    let hash = hash.digest_algorithm();
    let mut context = Context::new(hash);
    context.update(data);
    let digest = context.finish();

    Ok(digest.as_ref().to_vec())
}
