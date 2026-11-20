// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::chmod::{set_mode, set_mode_sync};

use llrt_path::resolve_path;
use llrt_utils::result::ResultExt;
use rquickjs::{function::Opt, Ctx, Object, Result};
use tokio::fs;

pub async fn mkdir<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<String> {
    let (recursive, mode, path) = get_params(&path, options)?;

    if recursive {
        fs::create_dir_all(&path).await
    } else {
        fs::create_dir(&path).await
    }
    .or_throw_msg(&ctx, &["Can't create dir \"", &path, "\""].concat())?;

    set_mode(ctx, &path, mode).await?;

    Ok(path)
}

pub fn mkdir_sync<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<String> {
    let (recursive, mode, path) = get_params(&path, options)?;

    if recursive {
        std::fs::create_dir_all(&path)
    } else {
        std::fs::create_dir(&path)
    }
    .or_throw_msg(&ctx, &["Can't create dir \"", &path, "\""].concat())?;

    set_mode_sync(ctx, &path, mode)?;

    Ok(path)
}

fn get_params(path: &str, options: Opt<Object>) -> Result<(bool, u32, String)> {
    let mut recursive = false;
    let mut mode = 0o777;

    if let Some(options) = options.0 {
        recursive = options.get("recursive").unwrap_or_default();
        mode = options.get("mode").unwrap_or(0o777);
    }
    let path = resolve_path([path])?;
    Ok((recursive, mode, path))
}

const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn random_chars(ctx: &Ctx, len: usize) -> Result<String> {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).or_throw(ctx)?;
    Ok(bytes
        .iter()
        .map(|&byte| {
            let idx = (byte as usize) % CHARS.len();
            CHARS[idx] as char
        })
        .collect::<String>())
}

pub async fn mkdtemp(ctx: Ctx<'_>, prefix: String) -> Result<String> {
    let path = [prefix.as_str(), random_chars(&ctx, 6)?.as_str()].join(",");
    fs::create_dir_all(&path)
        .await
        .or_throw_msg(&ctx, &["Can't create dir \"", &path, "\""].concat())?;
    Ok(path)
}

pub fn mkdtemp_sync(ctx: Ctx<'_>, prefix: String) -> Result<String> {
    let path = [prefix.as_str(), random_chars(&ctx, 6)?.as_str()].join(",");
    std::fs::create_dir_all(&path)
        .or_throw_msg(&ctx, &["Can't create dir \"", &path, "\""].concat())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use llrt_test::test_async_with;

    use super::*;

    #[tokio::test]
    async fn test_random_chars_shape() {
        test_async_with(|ctx| {
            Box::pin(async move {
                let chars = random_chars(&ctx, 6).unwrap();
                assert_eq!(chars.len(), 6);
                assert!(chars.bytes().all(|b| b.is_ascii_alphanumeric()));
            })
        })
        .await;
    }
}
