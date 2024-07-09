// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::utils::result::ResultExt;

use rquickjs::{Ctx, Result};
use tokio::fs;

use std::{
    fs::Metadata,
    time::{Duration, SystemTime},
};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Stat {
    #[qjs(skip_trace)]
    metadata: Metadata,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Stat {
    #[qjs(skip)]
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }

    #[qjs(get, enumerable)]
    pub fn dev(&self) -> u64 {
        self.metadata.dev()
    }

    #[qjs(get, enumerable)]
    pub fn ino(&self) -> u64 {
        self.metadata.ino()
    }

    #[qjs(get, enumerable)]
    pub fn mode(&self) -> u32 {
        self.metadata.mode()
    }

    #[qjs(get, enumerable)]
    pub fn nlink(&self) -> u64 {
        self.metadata.nlink()
    }

    #[qjs(get, enumerable)]
    pub fn uid(&self) -> u32 {
        self.metadata.uid()
    }

    #[qjs(get, enumerable)]
    pub fn gid(&self) -> u32 {
        self.metadata.gid()
    }

    #[qjs(get, enumerable)]
    pub fn rdev(&self) -> u64 {
        self.metadata.rdev()
    }

    #[qjs(get, enumerable)]
    pub fn size(&self) -> u64 {
        self.metadata.size()
    }

    #[qjs(get, enumerable)]
    pub fn blksize(&self) -> u64 {
        self.metadata.blksize()
    }

    #[qjs(get, enumerable)]
    pub fn blocks(&self) -> u64 {
        self.metadata.blocks()
    }

    #[cfg(unix)]
    #[qjs(get, enumerable)]
    pub fn atime_ms(&self) -> i64 {
        self.metadata.atime_nsec() / 1e6 as i64
    }

    #[cfg(unix)]
    #[qjs(get, enumerable)]
    pub fn mtime_ms(&self) -> i64 {
        self.metadata.mtime_nsec() / 1e6 as i64
    }

    #[cfg(unix)]
    #[qjs(get, enumerable)]
    pub fn ctime_ms(&self) -> i64 {
        self.metadata.ctime_nsec() / 1e6 as i64
    }

    #[cfg(unix)]
    #[qjs(get, enumerable)]
    pub fn birthtime_ms(&self, ctx: Ctx<'_>) -> Result<u64> {
        self.metadata
            .created()
            .or_throw(&ctx)
            .and_then(|c| c.elapsed().or_throw(&ctx))
            .map(|d| d.as_millis() as u64)
    }

    #[qjs(get, enumerable)]
    pub fn atime(&self, ctx: Ctx<'_>) -> Result<SystemTime> {
        self.metadata.accessed().or_throw(&ctx)
    }

    #[qjs(get, enumerable)]
    pub fn mtime(&self, ctx: Ctx<'_>) -> Result<SystemTime> {
        self.metadata.modified().or_throw(&ctx)
    }

    #[qjs(get, enumerable)]
    pub fn ctime(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_nanos(self.metadata.ctime_nsec() as u64)
    }

    #[qjs(get, enumerable)]
    pub fn birthtime(&self, ctx: Ctx<'_>) -> Result<SystemTime> {
        self.metadata.created().or_throw(&ctx)
    }

    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    pub fn is_symlink(&self) -> bool {
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

pub async fn stat_fn(ctx: Ctx<'_>, path: String) -> Result<Stat> {
    let metadata = fs::metadata(&path)
        .await
        .or_throw_msg(&ctx, &["Can't stat \"", &path, "\""].concat())?;

    let stats = Stat::new(metadata);

    Ok(stats)
}

pub fn stat_fn_sync(ctx: Ctx<'_>, path: String) -> Result<Stat> {
    let metadata =
        std::fs::metadata(&path).or_throw_msg(&ctx, &["Can't stat \"", &path, "\""].concat())?;

    let stats = Stat::new(metadata);

    Ok(stats)
}
