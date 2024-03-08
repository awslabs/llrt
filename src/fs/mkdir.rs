// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::os::unix::prelude::PermissionsExt;

use ring::rand::{SecureRandom, SystemRandom};
use rquickjs::{function::Opt, Ctx, Object, Result};
use tokio::fs;

use crate::utils::result::ResultExt;

pub async fn mkdir<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<String> {
    let (recursive, mode) = get_mkdir_params(options);

    if recursive {
        fs::create_dir_all(&path).await
    } else {
        fs::create_dir(&path).await
    }
    .or_throw_msg(&ctx, &format!("Can't create dir \"{}\"", &path))?;

    fs::set_permissions(&path, PermissionsExt::from_mode(mode))
        .await
        .or_throw_msg(&ctx, &format!("Can't set permissions of \"{}\"", &path))?;

    Ok(path)
}

pub fn mkdir_sync<'js>(ctx: Ctx<'js>, path: String, options: Opt<Object<'js>>) -> Result<String> {
    let (recursive, mode) = get_mkdir_params(options);

    if recursive {
        std::fs::create_dir_all(&path)
    } else {
        std::fs::create_dir(&path)
    }
    .or_throw_msg(&ctx, &format!("Can't create dir \"{}\"", &path))?;

    std::fs::set_permissions(&path, PermissionsExt::from_mode(mode))
        .or_throw_msg(&ctx, &format!("Can't set permissions of \"{}\"", &path))?;

    Ok(path)
}

fn get_mkdir_params(options: Opt<Object>) -> (bool, u32) {
    let mut recursive = false;
    let mut mode = 0o777;

    if let Some(options) = options.0 {
        recursive = options.get("recursive").unwrap_or_default();
        mode = options.get("mode").unwrap_or(0o777);
    }
    (recursive, mode)
}

const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn random_chars(len: usize) -> String {
    let random = SystemRandom::new();

    let mut bytes = vec![0u8; len];
    random.fill(&mut bytes).unwrap();
    bytes
        .iter()
        .map(|&byte| {
            let idx = (byte as usize) % CHARS.len();
            CHARS[idx] as char
        })
        .collect::<String>()
}

pub async fn mkdtemp(ctx: Ctx<'_>, prefix: String) -> Result<String> {
    let path = format!("{},{}", &prefix, &random_chars(6));
    fs::create_dir_all(&path)
        .await
        .or_throw_msg(&ctx, &format!("Can't create dir \"{}\"", &path))?;
    Ok(path)
}

pub fn mkdtemp_sync(ctx: Ctx<'_>, prefix: String) -> Result<String> {
    let path = format!("{},{}", &prefix, &random_chars(6));
    std::fs::create_dir_all(&path)
        .or_throw_msg(&ctx, &format!("Can't create dir \"{}\"", &path))?;
    Ok(path)
}
