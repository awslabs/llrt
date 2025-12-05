// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::io;

use llrt_utils::result::ResultExt;
use rquickjs::{function::Opt, Ctx, Result};

fn symlink_blocking(target: &str, path: &str, type_value: Option<String>) -> io::Result<()> {
    #[cfg(unix)]
    {
        _ = type_value;
        std::os::unix::fs::symlink(target, path)
    }
    #[cfg(windows)]
    {
        let type_str = match type_value.as_deref() {
            Some(t @ ("file" | "dir" | "junction")) => t,
            _ => {
                if std::fs::metadata(target)
                    .map(|m| m.is_dir())
                    .unwrap_or(false)
                {
                    "dir"
                } else {
                    "file"
                }
            },
        };
        match type_str {
            "junction" => junction::create(target, path),
            "dir" => std::os::windows::fs::symlink_dir(target, path),
            _ => std::os::windows::fs::symlink_file(target, path),
        }
    }
}

pub async fn symlink<'js>(
    ctx: Ctx<'js>,
    target: String,
    path: String,
    type_value: Opt<String>,
) -> Result<()> {
    let path_clone = path.clone();

    tokio::task::spawn_blocking(move || symlink_blocking(&target, &path_clone, type_value.0))
        .await
        .map_err(io::Error::other)?
        .or_throw_msg(&ctx, &["Can't create symlink \"", &path, "\""].concat())
}

pub fn symlink_sync<'js>(
    ctx: Ctx<'js>,
    target: String,
    path: String,
    type_value: Opt<String>,
) -> Result<()> {
    symlink_blocking(&target, &path, type_value.0)
        .or_throw_msg(&ctx, &["Can't create symlink \"", &path, "\""].concat())
}
