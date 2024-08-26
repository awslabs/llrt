// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::{fs::Metadata, path::PathBuf};

use llrt_utils::fs::DirectoryWalker;
use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Class, Ctx, IntoJs, Object, Result, Value,
};

use crate::modules::path::{is_absolute, CURRENT_DIR_STR};

#[derive(rquickjs::class::Trace)]
#[rquickjs::class]
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

    pub fn is_symbolic_link(&self) -> bool {
        self.metadata.is_symlink()
    }

    #[qjs(rename = "isFIFO")]
    pub fn is_fifo(&self) -> bool {
        #[cfg(unix)]
        {
            self.metadata.file_type().is_fifo()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    pub fn is_block_device(&self) -> bool {
        #[cfg(unix)]
        {
            self.metadata.file_type().is_block_device()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    pub fn is_character_device(&self) -> bool {
        #[cfg(unix)]
        {
            self.metadata.file_type().is_char_device()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }

    pub fn is_socket(&self) -> bool {
        #[cfg(unix)]
        {
            self.metadata.file_type().is_socket()
        }
        #[cfg(not(unix))]
        {
            false
        }
    }
}

struct ReadDirItem {
    name: String,
    metadata: Option<Metadata>,
}

pub struct ReadDir {
    items: Vec<ReadDirItem>,
    root: String,
}

impl<'js> IntoJs<'js> for ReadDir {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let arr = Array::new(ctx.clone())?;
        for (index, item) in self.items.into_iter().enumerate() {
            if let Some(metadata) = item.metadata {
                let dirent = Dirent { metadata };

                let dirent = Class::instance(ctx.clone(), dirent)?;
                dirent.set(PredefinedAtom::Name, item.name)?;
                dirent.set("parentPath", &self.root)?;
                arr.set(index, dirent)?;
            } else {
                arr.set(index, item.name)?;
            }
        }
        arr.into_js(ctx)
    }
}

pub async fn read_dir<'js>(mut path: String, options: Opt<Object<'js>>) -> Result<ReadDir> {
    let (with_file_types, skip_root_pos, mut directory_walker) =
        process_options_and_create_directory_walker(&mut path, options);

    let mut items = Vec::with_capacity(64);

    while let Some((child, metadata)) = directory_walker.walk().await? {
        append_directory_and_metadata_to_vec(
            with_file_types,
            skip_root_pos,
            &mut items,
            child,
            metadata,
        );
    }

    items.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

    Ok(ReadDir { items, root: path })
}

pub fn read_dir_sync(mut path: String, options: Opt<Object<'_>>) -> Result<ReadDir> {
    let (with_file_types, skip_root_pos, mut directory_walker) =
        process_options_and_create_directory_walker(&mut path, options);

    let mut items = Vec::with_capacity(64);
    while let Some((child, metadata)) = directory_walker.walk_sync()? {
        append_directory_and_metadata_to_vec(
            with_file_types,
            skip_root_pos,
            &mut items,
            child,
            metadata,
        );
    }

    items.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

    Ok(ReadDir { items, root: path })
}

type OptionsAndDirectoryWalker = (bool, usize, DirectoryWalker<fn(&str) -> bool>);

fn process_options_and_create_directory_walker(
    path: &mut String,
    options: Opt<Object>,
) -> OptionsAndDirectoryWalker {
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
                    path.insert_str(0, CURRENT_DIR_STR);
                }
                path.len() + 1
            },
        }
    };

    let mut directory_walker: DirectoryWalker<fn(&str) -> bool> =
        DirectoryWalker::new(PathBuf::from(&path), |_| true);

    if is_recursive {
        directory_walker.set_recursive(true);
    }
    (with_file_types, skip_root_pos, directory_walker)
}

fn append_directory_and_metadata_to_vec(
    with_file_types: bool,
    skip_root_pos: usize,
    items: &mut Vec<ReadDirItem>,
    child: PathBuf,
    metadata: Metadata,
) {
    let metadata = if with_file_types {
        Some(metadata)
    } else {
        None
    };

    let name = child.into_os_string().to_string_lossy()[skip_root_pos..].to_string();

    items.push(ReadDirItem { name, metadata })
}
