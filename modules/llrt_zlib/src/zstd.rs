// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::Read;

use llrt_buffer::Buffer;
use llrt_context::CtxExtension;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{
    prelude::{Opt, Rest},
    Ctx, Error, Exception, Function, IntoJs, Null, Result, Value,
};

use super::{define_cb_function, define_sync_function};

enum ZstdCommand {
    Compress,
    Decompress,
}

fn zstd_converter<'js>(
    ctx: Ctx<'js>,
    bytes: ObjectBytes<'js>,
    options: Opt<Value<'js>>,
    command: ZstdCommand,
) -> Result<Value<'js>> {
    let src = bytes.as_bytes(&ctx)?;

    let mut level = llrt_compression::zstd::DEFAULT_COMPRESSION_LEVEL;
    if let Some(options) = options.0 {
        if let Some(opt) = options.get_optional("level")? {
            level = opt;
        }
    }

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        ZstdCommand::Compress => {
            llrt_compression::zstd::encoder(src, level)?.read_to_end(&mut dst)?
        },
        ZstdCommand::Decompress => llrt_compression::zstd::decoder(src)?.read_to_end(&mut dst)?,
    };

    Buffer(dst).into_js(&ctx)
}

define_cb_function!(zstd_comp, zstd_converter, ZstdCommand::Compress);
define_sync_function!(zstd_comp_sync, zstd_converter, ZstdCommand::Compress);

define_cb_function!(zstd_decomp, zstd_converter, ZstdCommand::Decompress);
define_sync_function!(zstd_decomp_sync, zstd_converter, ZstdCommand::Decompress);
