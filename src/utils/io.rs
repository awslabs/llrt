// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    fs::Metadata,
    io,
    path::{Path, PathBuf},
};

use tokio::fs::{self};

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

pub struct DirectoryWalker<T>
where
    T: Fn(&str) -> bool,
{
    stack: Vec<(PathBuf, Option<Metadata>)>,
    filter: T,
    recursive: bool,
    eat_root: bool,
}

impl<T> DirectoryWalker<T>
where
    T: Fn(&str) -> bool,
{
    pub fn new(root: PathBuf, filter: T) -> Self {
        Self {
            stack: vec![(root, None)],
            filter,
            recursive: false,
            eat_root: true,
        }
    }

    pub fn set_recursive(&mut self, recursive: bool) {
        self.recursive = recursive;
    }

    pub async fn walk(&mut self) -> io::Result<Option<(PathBuf, Metadata)>> {
        if self.eat_root {
            self.eat_root = false;
            let (dir, _) = self.stack.pop().unwrap();
            self.append_stack(&dir).await?;
        }
        if let Some((dir, metadata)) = self.stack.pop() {
            let metadata = metadata.unwrap();
            if self.recursive && metadata.is_dir() {
                self.append_stack(&dir).await?;
            }

            Ok(Some((dir, metadata)))
        } else {
            Ok(None)
        }
    }

    pub fn walk_sync(&mut self) -> io::Result<Option<(PathBuf, Metadata)>> {
        if self.eat_root {
            self.eat_root = false;
            let (dir, _) = self.stack.pop().unwrap();
            self.append_stack_sync(&dir)?;
        }
        if let Some((dir, metadata)) = self.stack.pop() {
            let metadata = metadata.unwrap();
            if self.recursive && metadata.is_dir() {
                self.append_stack_sync(&dir)?;
            }

            Ok(Some((dir, metadata)))
        } else {
            Ok(None)
        }
    }

    async fn append_stack(&mut self, dir: &PathBuf) -> io::Result<()> {
        let mut stream = fs::read_dir(dir).await?;

        while let Some(entry) = stream.next_entry().await? {
            let entry_path = entry.path();

            let name = entry.file_name().to_string_lossy().to_string();
            if !(self.filter)(&name) {
                continue;
            }

            let metadata = fs::symlink_metadata(&entry_path).await?;

            self.stack.push((entry_path, Some(metadata)));
        }
        Ok(())
    }

    fn append_stack_sync(&mut self, dir: &PathBuf) -> io::Result<()> {
        let dir = std::fs::read_dir(dir)?;

        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !(self.filter)(&name) {
                continue;
            }
            let entry_path = entry.path();
            let metadata = entry_path.symlink_metadata()?;
            self.stack.push((entry_path, Some(metadata)))
        }

        Ok(())
    }
}
