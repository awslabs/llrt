// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    path::{Path, PathBuf},
    result::Result as StdResult,
};

use tokio::fs::{self, DirEntry};

pub fn get_basename_ext_name(path: &str) -> (String, String) {
    let path = path.strip_prefix("./").unwrap_or(path);
    let (basename, ext) = path.split_at(path.rfind('.').unwrap_or(path.len()));
    (basename.to_string(), ext.to_string())
}

pub static JS_EXTENSIONS: &[&str] = &[".js", ".mjs", ".cjs"];

pub fn get_js_path(path: &str) -> Option<PathBuf> {
    let (mut basename, ext) = get_basename_ext_name(path);

    let filepath = Path::new(path);

    let exists = filepath.exists();

    if !ext.is_empty() && exists {
        return Some(filepath.to_owned());
    }

    if filepath.is_dir() && exists {
        basename = format!("{}/index", &basename);
    }

    for ext in JS_EXTENSIONS {
        let path = &format!("{}{}", &basename, ext);

        let path = Path::new(path);
        if path.exists() {
            return Some(path.to_owned());
        }
    }

    None
}

pub async fn walk_directory<F>(path: PathBuf, mut f: F) -> StdResult<(), std::io::Error>
where
    F: FnMut(&DirEntry) -> bool,
{
    let mut stack = vec![path];
    while let Some(dir) = stack.pop() {
        let mut stream = fs::read_dir(dir).await?;
        while let Some(entry) = stream.next_entry().await? {
            let entry_path = entry.path();

            if f(&entry) && entry_path.is_dir() {
                stack.push(entry_path);
            }
        }
    }
    Ok(())
}
