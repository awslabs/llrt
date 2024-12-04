// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{ArrayBuffer, Ctx, Exception, IntoJs, Result, Value};

use crate::subtle::CryptoKey;

pub async fn subtle_export_key<'js>(
    ctx: Ctx<'js>,
    format: String,
    key: CryptoKey<'js>,
) -> Result<Value<'js>> {
    if !key.extractable() {
        return Err(Exception::throw_type(
            &ctx,
            "The CryptoKey is nonextractable",
        ));
    };

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
