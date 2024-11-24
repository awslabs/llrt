// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{Ctx, Result};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::subtle::{get_sha, Sha};

pub fn digest(ctx: &Ctx<'_>, name: &str, data: Vec<u8>) -> Result<Vec<u8>> {
    let sha = get_sha(ctx, name)?;

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
