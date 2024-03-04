// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{fs::Metadata, path::PathBuf};

use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Class, Ctx, IntoJs, Object, Result, Value,
};

use crate::{
    path::{is_absolute, CURRENT_DIR_STR},
    utils::io::DirectoryWalker,
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
    _ctx: Ctx<'js>,
    path: String,
    options: Opt<Object<'js>>,
) -> Result<ReadDir> {
    let mut path = path;

    let mut with_file_types = false;
    let mut is_recursive = false;

    if let Some(options) = options.0 {
        with_file_types = options
            .get("withFileTypes")
            .ok()
            .and_then(|file_types: Value| file_types.as_bool())
            .unwrap_or_default();

        is_recursive = options
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

    let mut directory_walker = DirectoryWalker::new(PathBuf::from(path), |_| true);

    if is_recursive {
        directory_walker.set_recursive(true);
    }

    while let Some((child, metadata)) = directory_walker.walk().await? {
        let metadata = if with_file_types {
            Some(metadata)
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
