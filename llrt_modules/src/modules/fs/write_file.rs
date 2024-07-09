// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::bytes::get_bytes;
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result, Value};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub async fn write_file<'js>(ctx: Ctx<'js>, path: String, data: Value<'js>) -> Result<()> {
    let mut file = fs::File::create(&path)
        .await
        .or_throw_msg(&ctx, &["Can't create file \"", &path, "\""].concat())?;

    let write_error_message = &["Can't write file \"", &path, "\""].concat();

    let bytes = get_bytes(&ctx, data)?;
    file.write_all(&bytes)
        .await
        .or_throw_msg(&ctx, write_error_message)?;
    file.flush().await.or_throw_msg(&ctx, write_error_message)?;

    Ok(())
}

pub fn write_file_sync<'js>(ctx: Ctx<'js>, path: String, data: Value<'js>) -> Result<()> {
    let bytes = get_bytes(&ctx, data)?;

    std::fs::write(&path, bytes).or_throw_msg(&ctx, &["Can't write \"{}\"", &path].concat())?;

    Ok(())
}
