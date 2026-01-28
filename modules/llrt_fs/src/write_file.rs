// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use either::Either;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{function::Opt, Ctx, Error, FromJs, Result, Value};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::security::ensure_access;

pub async fn write_file<'js>(
    ctx: Ctx<'js>,
    path: String,
    data: Value<'js>,
    options: Opt<Either<String, WriteFileOptions>>,
) -> Result<()> {
    ensure_access(&ctx, &path)?;

    let write_error_message = &["Can't write file \"", &path, "\""].concat();

    let mut file = fs::File::create(&path)
        .await
        .or_throw_msg(&ctx, write_error_message)?;

    #[cfg(unix)]
    if let Some(Either::Right(opts)) = options.0 {
        use std::os::unix::fs::PermissionsExt;

        let perm = PermissionsExt::from_mode(opts.mode.unwrap_or(0o666));
        file.set_permissions(perm)
            .await
            .or_throw_msg(&ctx, write_error_message)?;
    }
    #[cfg(not(unix))]
    {
        _ = options;
        if let Some(Either::Right(opts)) = options.0 {
            _ = opts.mode;
        }
    }

    let bytes = ObjectBytes::from(&ctx, &data)?;
    file.write_all(bytes.as_bytes(&ctx)?)
        .await
        .or_throw_msg(&ctx, write_error_message)?;
    file.flush().await.or_throw_msg(&ctx, write_error_message)?;

    Ok(())
}

pub fn write_file_sync<'js>(
    ctx: Ctx<'js>,
    path: String,
    bytes: ObjectBytes<'js>,
    options: Opt<Either<String, WriteFileOptions>>,
) -> Result<()> {
    ensure_access(&ctx, &path)?;

    let write_error_message = &["Can't write file \"", &path, "\""].concat();
    std::fs::write(&path, bytes.as_bytes(&ctx)?).or_throw_msg(&ctx, write_error_message)?;

    #[cfg(unix)]
    {
        if let Some(Either::Right(opts)) = options.0 {
            use std::os::unix::fs::PermissionsExt;

            std::fs::set_permissions(path, PermissionsExt::from_mode(opts.mode.unwrap_or(0o666)))
                .or_throw_msg(&ctx, write_error_message)?;
        }
    }
    #[cfg(not(unix))]
    {
        _ = options;
        if let Some(Either::Right(opts)) = options.0 {
            _ = opts.mode;
        }
    }

    Ok(())
}

pub(crate) struct WriteFileOptions {
    pub mode: Option<u32>,
}

impl<'js> FromJs<'js> for WriteFileOptions {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let mode = obj.get_optional::<_, u32>("mode")?;

        Ok(Self { mode })
    }
}
