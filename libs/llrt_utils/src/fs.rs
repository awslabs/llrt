// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{fs::Metadata, io, path::PathBuf};

use tokio::fs::{self};

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
        if let Some((entry, metadata)) = self.stack.pop() {
            let metadata = metadata.unwrap();
            if self.recursive && metadata.is_dir() {
                self.append_stack(&entry).await?;
            }

            Ok(Some((entry, metadata)))
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
        if let Some((entry, metadata)) = self.stack.pop() {
            let metadata = metadata.unwrap();
            if self.recursive && metadata.is_dir() {
                self.append_stack_sync(&entry)?;
            }

            Ok(Some((entry, metadata)))
        } else {
            Ok(None)
        }
    }

    async fn append_stack(&mut self, dir: &PathBuf) -> io::Result<()> {
        let mut stream = fs::read_dir(dir).await?;

        while let Some(entry) = stream.next_entry().await? {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !(self.filter)(name.as_ref()) {
                continue;
            }
            let entry_path = entry.path();
            let metadata = fs::symlink_metadata(&entry_path).await?;

            self.stack.push((entry_path, Some(metadata)));
        }
        Ok(())
    }

    fn append_stack_sync(&mut self, dir: &PathBuf) -> io::Result<()> {
        let dir = std::fs::read_dir(dir)?;

        for entry in dir.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !(self.filter)(name.as_ref()) {
                continue;
            }
            let entry_path = entry.path();
            let metadata = entry_path.symlink_metadata()?;
            self.stack.push((entry_path, Some(metadata)))
        }

        Ok(())
    }
}
