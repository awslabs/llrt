// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use llrt_utils::result::ResultExt;
use rquickjs::{function::Opt, Ctx, Result};

pub async fn symlink<'js>(
    ctx: Ctx<'js>,
    target: String,
    path: String,
    type_value: Opt<String>,
) -> Result<()> {
    let error_message = &["Can't create symlink \"", &path, "\""].concat();

    #[cfg(unix)]
    {
        _ = type_value;
        tokio::fs::symlink(&target, &path)
            .await
            .or_throw_msg(&ctx, error_message)?;
    }
    #[cfg(not(unix))]
    {
        let mut type_str = type_value.0.unwrap_or("".to_string());
        if type_str != "file" && type_str != "dir" && type_str != "junction" && type_str != "" {
            type_str = "".to_string();
        }
        if type_str == "" {
            let metadata = tokio::fs::metadata(&target)
                .await
                .or_throw_msg(&ctx, &["Can't stat \"", &path, "\""].concat());
            match metadata {
                Ok(meta) => {
                    if meta.is_dir() {
                        type_str = "dir".to_string();
                    } else {
                        type_str = "file".to_string();
                    }
                },
                Err(_) => {
                    type_str = "file".to_string(); // Default to file if we can't determine
                },
            }
        }
        if type_str == "junction" {
            std::os::windows::fs::junction_point(&target, &path)
                .or_throw_msg(&ctx, error_message)?;
        } else if type_str == "dir" {
            tokio::fs::symlink_dir(&target, &path)
                .await
                .or_throw_msg(&ctx, error_message)?;
        } else {
            tokio::fs::symlink_file(&target, &path)
                .await
                .or_throw_msg(&ctx, error_message)?;
        }
    }

    Ok(())
}

pub fn symlink_sync<'js>(
    ctx: Ctx<'js>,
    target: String,
    path: String,
    type_value: Opt<String>,
) -> Result<()> {
    let error_message = &["Can't create symlink \"", &path, "\""].concat();

    #[cfg(unix)]
    {
        _ = type_value;
        std::os::unix::fs::symlink(&target, &path).or_throw_msg(&ctx, error_message)?;
    }
    #[cfg(not(unix))]
    {
        use std::os::windows::fs;

        let mut type_str = type_value.0.unwrap_or("".to_string());
        if type_str != "file" && type_str != "dir" && type_str != "junction" && type_str != "" {
            type_str = "".to_string();
        }
        if type_str == "" {
            let metadata =
                fs::metadata(&target).or_throw_msg(&ctx, &["Can't stat \"", &path, "\""].concat());
            match metadata {
                Ok(meta) => {
                    if meta.is_dir() {
                        type_str = "dir".to_string();
                    } else {
                        type_str = "file".to_string();
                    }
                },
                Err(_) => {
                    type_str = "file".to_string(); // Default to file if we can't determine
                },
            }
        }
        if type_str == "junction" {
            fs::junction_point(&target, &path).or_throw_msg(&ctx, error_message)?;
        } else if type_str == "dir" {
            fs::symlink_dir(&target, &path).or_throw_msg(&ctx, error_message)?;
        } else {
            fs::symlink_file(&target, &path).or_throw_msg(&ctx, error_message)?;
        }
    }

    Ok(())
}
