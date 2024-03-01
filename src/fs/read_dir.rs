// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::VecDeque,
    fs::Metadata,
    io,
    path::{Path, PathBuf},
};

use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Class, Ctx, IntoJs, Object, Result, Value,
};
use tokio::fs;

use crate::{
    path::{self, is_absolute, CURRENT_DIR_STR},
    utils::{io::DirectoryWalker, result::ResultExt},
};

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Dirent {
    #[qjs(skip_trace)]
    metadata: Metadata,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Dirent {
    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }
    pub fn is_directory(&self) -> bool {
        self.metadata.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink()
    }
}

pub struct ReadDir {
    items: Vec<(String, Option<Metadata>)>,
}

impl<'js> IntoJs<'js> for ReadDir {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let arr = Array::new(ctx.clone())?;
        for (index, (name, metadata)) in self.items.into_iter().enumerate() {
            if let Some(metadata) = metadata {
                let dirent = Dirent { metadata };

                let dirent = Class::instance(ctx.clone(), dirent)?;
                dirent.set(PredefinedAtom::Name, name)?;
                arr.set(index, dirent)?;
            } else {
                arr.set(index, name)?;
            }
        }
        arr.into_js(ctx)
    }
}

pub async fn read_dir<'js>(
    ctx: Ctx<'js>,
    path: String,
    options: Opt<Object<'js>>,
) -> Result<ReadDir> {
    let mut path = path;

    let mut with_file_types = false;
    let mut with_recursive = false;

    if let Some(options) = options.0 {
        with_file_types = options
            .get("withFileTypes")
            .ok()
            .and_then(|file_types: Value| file_types.as_bool())
            .unwrap_or_default();

        with_recursive = options
            .get("recursive")
            .ok()
            .and_then(|recursive: Value| recursive.as_bool())
            .unwrap_or_default();
    };

    let skip_root_pos = {
        match path.as_str() {
            // . | ./
            "." | CURRENT_DIR_STR => CURRENT_DIR_STR.len(),
            // ./path
            _ if path.starts_with(CURRENT_DIR_STR) => path.len() + 1,
            // path
            _ => {
                if !is_absolute(path.clone()) {
                    path = [CURRENT_DIR_STR.to_string(), path].concat();
                }
                path.len() + 1
            }
        }
    };
    let mut items = Vec::with_capacity(64);

    let mut directory_walker = DirectoryWalker::new(PathBuf::from(path));

    async fn walk(
        walker: &mut DirectoryWalker,
        with_recursive: bool,
    ) -> io::Result<Option<PathBuf>> {
        if with_recursive {
            walker.walk_recursive().await
        } else {
            walker.walk().await
        }
    }

    // eat root
    walk(&mut directory_walker, with_recursive).await?;

    while let Some(child) = walk(&mut directory_walker, with_recursive).await? {
        let metadata = if with_file_types {
            Some(fs::metadata(&child).await?)
        } else {
            None
        };

        items.push((
            child.into_os_string().to_string_lossy()[skip_root_pos..].to_string(),
            metadata,
        ))
    }

    items.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());

    Ok(ReadDir { items })
}
