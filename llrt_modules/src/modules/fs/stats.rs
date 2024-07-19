// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[allow(unused_imports)]
use std::{
    fs::Metadata,
    time::{Duration, SystemTime},
};

use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result};
use tokio::fs;

// The Stats implementation is very much based on Unix. The Windows implementation
// tries its best to mimic the implementation of libuv since it is the standard.
// See: https://github.com/libuv/libuv/blob/90648ea3e55125a5a819b32106da6462da310da6/src/win/fs.c
//
// By comparison, the Deno implementation is very basic and doesn't even try much.
// See: https://github.com/denoland/deno/blob/c9da27e147d0681724dd647593abbaa46417feb7/ext/io/fs.rs#L114-L182
//
// This implementation doesn't handle files created before UNIX_EPOCH.

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Stats {
    #[qjs(skip_trace)]
    metadata: Metadata,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Stats {
    #[qjs(skip)]
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }

    #[qjs(get, enumerable)]
    pub fn dev(&self) -> u64 {
        #[cfg(unix)]
        {
            self.metadata.dev()
        }
        #[cfg(not(unix))]
        {
            // Unstable feature, see https://github.com/rust-lang/rust/issues/63010
            0
        }
    }

    #[qjs(get, enumerable)]
    pub fn ino(&self) -> u64 {
        #[cfg(unix)]
        {
            self.metadata.ino()
        }
        #[cfg(not(unix))]
        {
            // Unstable feature, see https://github.com/rust-lang/rust/issues/63010
            0
        }
    }

    #[qjs(get, enumerable)]
    pub fn mode(&self) -> u32 {
        #[cfg(unix)]
        {
            self.metadata.mode()
        }
        #[cfg(not(unix))]
        {
            0o666
        }
    }

    #[qjs(get, enumerable)]
    pub fn nlink(&self) -> u64 {
        #[cfg(unix)]
        {
            self.metadata.nlink()
        }
        #[cfg(not(unix))]
        {
            // Unstable feature, see https://github.com/rust-lang/rust/issues/63010
            1
        }
    }

    #[qjs(get, enumerable)]
    pub fn uid(&self) -> u32 {
        #[cfg(unix)]
        {
            self.metadata.uid()
        }
        #[cfg(not(unix))]
        {
            0
        }
    }

    #[qjs(get, enumerable)]
    pub fn gid(&self) -> u32 {
        #[cfg(unix)]
        {
            self.metadata.gid()
        }
        #[cfg(not(unix))]
        {
            0
        }
    }

    #[qjs(get, enumerable)]
    pub fn rdev(&self) -> u64 {
        #[cfg(unix)]
        {
            self.metadata.rdev()
        }
        #[cfg(not(unix))]
        {
            0
        }
    }

    #[qjs(get, enumerable)]
    pub fn size(&self) -> u64 {
        #[cfg(unix)]
        {
            self.metadata.size()
        }
        #[cfg(windows)]
        {
            if self.metadata.is_dir() {
                0
            } else {
                self.metadata.file_size()
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            0
        }
    }

    #[qjs(get, enumerable)]
    pub fn blksize(&self) -> u64 {
        #[cfg(unix)]
        {
            self.metadata.blksize()
        }
        #[cfg(not(unix))]
        {
            4096
        }
    }

    #[qjs(get, enumerable)]
    pub fn blocks(&self) -> u64 {
        #[cfg(unix)]
        {
            self.metadata.blocks()
        }
        #[cfg(not(unix))]
        {
            0
        }
    }

    #[qjs(get, enumerable)]
    pub fn atime_ms(&self, ctx: Ctx<'_>) -> Result<u64> {
        #[cfg(unix)]
        {
            _ = ctx;
            Ok(self.metadata.atime_nsec() as u64 / 1e6 as u64)
        }
        #[cfg(not(unix))]
        {
            self.metadata.accessed().map(to_msec).or_throw(&ctx)
        }
    }

    #[qjs(get, enumerable)]
    pub fn mtime_ms(&self, ctx: Ctx<'_>) -> Result<u64> {
        #[cfg(unix)]
        {
            _ = ctx;
            Ok(self.metadata.mtime_nsec() as u64 / 1e6 as u64)
        }
        #[cfg(not(unix))]
        {
            self.metadata.modified().map(to_msec).or_throw(&ctx)
        }
    }

    #[qjs(get, enumerable)]
    pub fn ctime_ms(&self, ctx: Ctx<'_>) -> Result<u64> {
        #[cfg(unix)]
        {
            _ = ctx;
            Ok(self.metadata.ctime_nsec() as u64 / 1e6 as u64)
        }
        #[cfg(not(unix))]
        {
            self.metadata.modified().map(to_msec).or_throw(&ctx)
        }
    }

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
    pub fn ctime(&self, ctx: Ctx<'_>) -> Result<SystemTime> {
        #[cfg(unix)]
        {
            _ = ctx;
            Ok(SystemTime::UNIX_EPOCH + Duration::from_nanos(self.metadata.ctime_nsec() as u64))
        }
        #[cfg(not(unix))]
        {
            self.metadata.modified().or_throw(&ctx)
        }
    }

    #[qjs(get, enumerable)]
    pub fn birthtime(&self, ctx: Ctx<'_>) -> Result<SystemTime> {
        self.metadata.created().or_throw(&ctx)
    }

    pub fn is_file(&self) -> bool {
        self.metadata.is_file()
    }

    /// @deprecated Use `is_directory` instead
    pub fn is_dir(&self) -> bool {
        self.metadata.is_dir()
    }

    pub fn is_directory(&self) -> bool {
        self.metadata.is_dir()
    }

    /// @deprecated Use `is_symbolic_link` instead
    pub fn is_symlink(&self) -> bool {
        self.metadata.is_symlink()
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

pub async fn stat_fn(ctx: Ctx<'_>, path: String) -> Result<Stats> {
    let metadata = fs::metadata(&path)
        .await
        .or_throw_msg(&ctx, &["Can't stat \"", &path, "\""].concat())?;

    let stats = Stats::new(metadata);

    Ok(stats)
}

pub fn stat_fn_sync(ctx: Ctx<'_>, path: String) -> Result<Stats> {
    let metadata =
        std::fs::metadata(&path).or_throw_msg(&ctx, &["Can't stat \"", &path, "\""].concat())?;

    let stats = Stats::new(metadata);

    Ok(stats)
}

#[allow(dead_code)]
#[inline(always)]
fn to_msec(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|t| t.as_millis() as u64)
        .unwrap_or_else(|err| err.duration().as_millis() as u64)
}
