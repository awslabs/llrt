// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::bytes::ObjectBytes;
use rquickjs::{Array, Ctx, Exception, Result, Value};

use crate::subtle::CryptoKey;

pub async fn subtle_import_key<'js>(
    ctx: Ctx<'js>,
    format: String,
    key_data: ObjectBytes<'js>,
    algorithm: Value<'js>,
    extractable: bool,
    key_usages: Array<'js>,
) -> Result<CryptoKey<'js>> {
    let handle = if format == "raw" {
        key_data
    } else {
        return Err(Exception::throw_type(
            &ctx,
            &["Format '", &format, "' is not implemented"].concat(),
        ));
    };

    CryptoKey::new(
        ctx,
        "secret".to_string(),
        extractable,
        algorithm,
        key_usages,
        handle.as_bytes(),
    )
}
