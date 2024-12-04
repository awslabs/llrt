// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use ring::digest::Context;
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};

use crate::subtle::Hash;

pub async fn subtle_digest<'js>(
    ctx: Ctx<'js>,
    algorithm: Value<'js>,
    data: ObjectBytes<'js>,
) -> Result<ArrayBuffer<'js>> {
    let algorithm = if let Some(algorithm) = algorithm.as_string() {
        algorithm.to_string().or_throw(&ctx)?
    } else {
        algorithm
            .get_optional::<_, String>("name")?
            .ok_or_else(|| Exception::throw_type(&ctx, "algorithm 'name' property required"))?
    };

    let bytes = digest(&ctx, &algorithm, data.as_bytes())?;
    ArrayBuffer::new(ctx, bytes)
}

fn digest(ctx: &Ctx<'_>, algorithm: &str, data: &[u8]) -> Result<Vec<u8>> {
    let hash = Hash::try_from(algorithm).or_throw(ctx)?;
    let hash = match hash {
        Hash::Sha1 => &ring::digest::SHA1_FOR_LEGACY_USE_ONLY,
        Hash::Sha256 => &ring::digest::SHA256,
        Hash::Sha384 => &ring::digest::SHA384,
        Hash::Sha512 => &ring::digest::SHA512,
    };

    let mut context = Context::new(hash);
    context.update(data);
    let digest = context.finish();

    Ok(digest.as_ref().to_vec())
}
