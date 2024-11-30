// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{ArrayBuffer, Ctx, Exception, IntoJs, Result, Value};

use super::crypto_key::CryptoKey;

pub async fn subtle_export_key<'js>(
    ctx: Ctx<'js>,
    format: String,
    key: CryptoKey<'js>,
) -> Result<Value<'js>> {
    if format == "raw" {
        export_raw(&ctx, &key).into_js(&ctx)
    } else {
        Err(Exception::throw_type(
            &ctx,
            &["Format '", &format, "' is not implemented"].concat(),
        ))
    }
}

fn export_raw<'js>(ctx: &Ctx<'js>, key: &CryptoKey<'js>) -> Result<ArrayBuffer<'js>> {
    ArrayBuffer::new(ctx.clone(), key.get_handle())
}
