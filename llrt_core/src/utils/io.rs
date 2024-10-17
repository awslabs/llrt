// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::path::{Path, PathBuf};

pub use llrt_utils::fs::DirectoryWalker;

pub fn get_basename_ext_name(path: &str) -> (&str, &str) {
    let path = path.strip_prefix("./").unwrap_or(path);
    let (basename, ext) = path.rsplit_once('.').unwrap_or((path, ""));
    (basename, ext)
}

pub static JS_EXTENSIONS: &[&str] = &[".js", ".mjs", ".cjs"];

pub fn get_js_path(path: &str) -> Option<PathBuf> {
    let (basename, ext) = get_basename_ext_name(path);

    let filepath = Path::new(path);

    let exists = filepath.exists();

    if !ext.is_empty() && exists {
        return Some(filepath.to_owned());
    }

    fn check_extensions(basename: &str) -> Option<PathBuf> {
        for ext in JS_EXTENSIONS {
            let path: &str = &[basename, ext].concat();
            let path = Path::new(path);
            if path.exists() {
                return Some(path.to_owned());
            }
        }
        None
    }

    if filepath.is_dir() && exists {
        let basename: &str = &([basename, "/index"].concat());
        return check_extensions(basename);
    }
    check_extensions(basename)
}
