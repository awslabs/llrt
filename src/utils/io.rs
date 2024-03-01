// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    io,
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

pub struct DirectoryWalker {
    stack: Vec<(PathBuf, bool)>,
    filters: Vec<Box<dyn Fn(&DirEntry) -> bool>>,
}

impl DirectoryWalker {
    pub fn new(path: PathBuf) -> Self {
        Self {
            stack: vec![(path, true)],
            filters: vec![],
        }
    }

    pub fn with_filter(&mut self, handler: impl Fn(&DirEntry) -> bool + 'static) {
        self.filters.push(Box::new(handler));
    }

    async fn with_walk(
        &mut self,
        mut handler: impl FnMut(&mut Self, PathBuf, bool),
    ) -> io::Result<Option<PathBuf>> {
        let is_filter = self.filters.is_empty();

        if let Some((dir, is_recursive)) = self.stack.pop() {
            if is_recursive {
                let mut stream = fs::read_dir(dir.clone()).await?;

                while let Some(entry) = stream.next_entry().await? {
                    let entry_path = entry.path();

                    if is_filter || (self.filters.iter().all(|filter| filter(&entry))) {
                        let is_dir = entry_path.is_dir();
                        handler(self, entry_path, is_dir);
                    }
                }
            }

            Ok(Some(dir))
        } else {
            Ok(None)
        }
    }

    pub async fn walk_recursive(&mut self) -> io::Result<Option<PathBuf>> {
        self.with_walk(|this, pathbuf, is_recursive| this.stack.push((pathbuf, is_recursive)))
            .await
    }

    pub async fn walk(&mut self) -> io::Result<Option<PathBuf>> {
        self.with_walk(|this, pathbuf, _| this.stack.push((pathbuf, false)))
            .await
    }
}
