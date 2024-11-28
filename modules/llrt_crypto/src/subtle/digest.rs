// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{ArrayBuffer, Ctx, Exception, Result, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::subtle::Sha;

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
    let sha = Sha::try_from(algorithm).or_throw(ctx)?;

    match sha {
        Sha::Sha1 => {
            let mut hasher = Sha1::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        },
        Sha::Sha256 => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        },
        Sha::Sha384 => {
            let mut hasher = Sha384::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        },
        Sha::Sha512 => {
            let mut hasher = Sha512::new();
            hasher.update(data);
            Ok(hasher.finalize().to_vec())
        },
    }
}
