// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::Read;

use llrt_buffer::Buffer;
use llrt_context::CtxExtension;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{
    prelude::{Opt, Rest},
    Ctx, Error, Exception, Function, IntoJs, Null, Result, Value,
};

use crate::{define_cb_function, define_sync_function};

enum BrotliCommand {
    Compress,
    Decompress,
}

fn brotli_converter<'js>(
    ctx: Ctx<'js>,
    bytes: ObjectBytes<'js>,
    _options: Opt<Value<'js>>,
    command: BrotliCommand,
) -> Result<Value<'js>> {
    let src = bytes.as_bytes(&ctx)?;

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        BrotliCommand::Compress => llrt_compression::brotli::encoder(src).read_to_end(&mut dst)?,
        BrotliCommand::Decompress => {
            llrt_compression::brotli::decoder(src).read_to_end(&mut dst)?
        },
    };

    Buffer(dst).into_js(&ctx)
}

define_cb_function!(br_comp, brotli_converter, BrotliCommand::Compress);
define_sync_function!(br_comp_sync, brotli_converter, BrotliCommand::Compress);

define_cb_function!(br_decomp, brotli_converter, BrotliCommand::Decompress);
define_sync_function!(br_decomp_sync, brotli_converter, BrotliCommand::Decompress);
