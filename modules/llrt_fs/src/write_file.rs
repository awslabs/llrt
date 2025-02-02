// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{Ctx, Result, Value};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub async fn write_file<'js>(ctx: Ctx<'js>, path: String, data: Value<'js>) -> Result<()> {
    let write_error_message = &["Can't write file \"", &path, "\""].concat();

    let mut file = fs::File::create(&path)
        .await
        .or_throw_msg(&ctx, write_error_message)?;

    let bytes = ObjectBytes::from(&ctx, &data)?;
    file.write_all(bytes.as_bytes(&ctx)?)
        .await
        .or_throw_msg(&ctx, write_error_message)?;
    file.flush().await.or_throw_msg(&ctx, write_error_message)?;

    Ok(())
}

pub fn write_file_sync<'js>(ctx: Ctx<'js>, path: String, bytes: ObjectBytes<'js>) -> Result<()> {
    std::fs::write(&path, bytes.as_bytes(&ctx)?)
        .or_throw_msg(&ctx, &["Can't write \"{}\"", &path].concat())?;

    Ok(())
}
