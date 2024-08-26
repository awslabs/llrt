// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::result::ResultExt;
use rquickjs::{function::Opt, Ctx, Object, Result};
use tokio::fs;

#[allow(clippy::manual_async_fn)]
pub async fn rmdir<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<()> {
    let recursive = get_params_rm_dir(options);

    if recursive {
        fs::remove_dir_all(&path).await
    } else {
        fs::remove_dir(&path).await
    }
    .or_throw_msg(&ctx, &["Can't remove dir \"", &path, "\""].concat())?;

    Ok(())
}

#[allow(clippy::manual_async_fn)]
pub fn rmdir_sync<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<()> {
    let recursive = get_params_rm_dir(options);

    if recursive {
        std::fs::remove_dir_all(&path)
    } else {
        std::fs::remove_dir(&path)
    }
    .or_throw_msg(&ctx, &["Can't remove dir \"", &path, "\""].concat())?;

    Ok(())
}

pub async fn rmfile<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<()> {
    let (recursive, force) = get_params_rm(options);

    let res = async move {
        let is_dir = fs::metadata(&path)
            .await
            .map(|metadata| metadata.is_dir())
            .or_throw(&ctx)?;

        (if is_dir && recursive {
            fs::remove_dir_all(&path).await
        } else if is_dir && !recursive {
            fs::remove_dir(&path).await
        } else {
            fs::remove_file(&path).await
        })
        .or_throw_msg(&ctx, &["Can't remove file \"", &path, "\""].concat())?;

        Ok(())
    }
    .await;

    if !force {
        return res;
    }

    Ok(())
}

pub fn rmfile_sync(path: String, options: Opt<Object<'_>>) -> Result<()> {
    let (recursive, force) = get_params_rm(options);

    let res = (|| -> Result<()> {
        let is_dir = std::fs::metadata(&path).map(|metadata| metadata.is_dir())?;

        (if is_dir && recursive {
            std::fs::remove_dir_all(&path)
        } else if is_dir && !recursive {
            std::fs::remove_dir(&path)
        } else {
            std::fs::remove_file(&path)
        })?;

        Ok(())
    })();

    if !force {
        return res;
    }

    Ok(())
}

fn get_params_rm_dir(options: Opt<Object>) -> bool {
    let mut recursive = false;

    if let Some(options) = options.0 {
        recursive = options.get("recursive").unwrap_or_default();
    }
    recursive
}

fn get_params_rm(options: Opt<Object>) -> (bool, bool) {
    let mut recursive = false;
    let mut force = false;

    if let Some(options) = options.0 {
        recursive = options.get("recursive").unwrap_or_default();
        force = options.get("force").unwrap_or_default();
    }
    (recursive, force)
}
