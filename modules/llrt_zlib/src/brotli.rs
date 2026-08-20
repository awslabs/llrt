// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_buffer::Buffer;
use llrt_context::CtxExtension;
use llrt_utils::{bytes::ObjectBytes, result::ResultExt};
use rquickjs::{
    prelude::{Opt, Rest},
    Ctx, Error, Exception, Function, IntoJs, Null, Result, Value,
};

use super::{define_cb_function, define_sync_function, max_output_length, read_to_end_limited};

enum BrotliCommand {
    Compress,
    Decompress,
}

fn brotli_converter<'js>(
    ctx: Ctx<'js>,
    bytes: ObjectBytes<'js>,
    options: Opt<Value<'js>>,
    command: BrotliCommand,
) -> Result<Value<'js>> {
    let src = bytes.as_bytes(&ctx)?;
    let limit = max_output_length(&options)?;

    let dst = match command {
        BrotliCommand::Compress => read_to_end_limited(
            &ctx,
            llrt_compression::brotli::encoder(src),
            limit,
            src.len(),
        )?,
        BrotliCommand::Decompress => read_to_end_limited(
            &ctx,
            llrt_compression::brotli::decoder(src),
            limit,
            src.len(),
        )?,
    };

    Buffer(dst).into_js(&ctx)
}

define_cb_function!(br_comp, brotli_converter, BrotliCommand::Compress);
define_sync_function!(br_comp_sync, brotli_converter, BrotliCommand::Compress);

define_cb_function!(br_decomp, brotli_converter, BrotliCommand::Decompress);
define_sync_function!(br_decomp_sync, brotli_converter, BrotliCommand::Decompress);
