// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::Read;

use brotlic::{CompressorReader as BrotliEncoder, DecompressorReader as BrotliDecoder};
use flate2::read::{DeflateDecoder, GzDecoder, ZlibDecoder};
use llrt_utils::{bytes::get_array_bytes, ctx::CtxExtension, module::export_default};
use rquickjs::function::Func;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Ctx, Error, Exception, FromJs, Function, IntoJs, Null, Result, Value,
};

use crate::{utils::array_buffer::ArrayBufferView, ModuleInfo};

use super::buffer::Buffer;

macro_rules! define_sync_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub fn $fn_name<'js>(ctx: Ctx<'js>, value: Value<'js>) -> Result<Value<'js>> {
            $converter(ctx.clone(), value, $command)
        }
    };
}

macro_rules! define_async_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub fn $fn_name<'js>(ctx: Ctx<'js>, value: Value<'js>, cb: Function<'js>) -> Result<()> {
            ctx.clone().spawn_exit(async move {
                match $converter(ctx.clone(), value, $command) {
                    Ok(obj) => {
                        () = cb.call((Null.into_js(&ctx), obj))?;
                        Ok::<_, Error>(())
                    },
                    Err(err) => {
                        () = cb.call((Exception::from_message(ctx, &err.to_string()),))?;
                        Ok(())
                    },
                }
            })?;
            Ok(())
        }
    };
}

enum ZlibCommand {
    Deflate,
    DeflateRaw,
    Gunzip,
}

fn zlib_converter<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    command: ZlibCommand,
) -> Result<Value<'js>> {
    let src = if value.is_string() {
        let string = value.as_string().unwrap().to_string()?;
        string.as_bytes().to_vec()
    } else if value.is_array() {
        get_array_bytes(&ctx, &value, 0, None)?.unwrap()
    } else {
        let buffer = ArrayBufferView::from_js(&ctx, value)?;
        buffer.as_bytes().unwrap().to_vec()
    };

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        ZlibCommand::Deflate => ZlibDecoder::new(&src[..]).read_to_end(&mut dst)?,
        ZlibCommand::DeflateRaw => DeflateDecoder::new(&src[..]).read_to_end(&mut dst)?,
        ZlibCommand::Gunzip => GzDecoder::new(&src[..]).read_to_end(&mut dst)?,
    };

    Buffer(dst).into_js(&ctx)
}

define_sync_function!(deflate_sync, zlib_converter, ZlibCommand::Deflate);
define_sync_function!(deflate_raw_sync, zlib_converter, ZlibCommand::DeflateRaw);
define_sync_function!(gunzip_sync, zlib_converter, ZlibCommand::Gunzip);

define_async_function!(deflate, zlib_converter, ZlibCommand::Deflate);
define_async_function!(deflate_raw, zlib_converter, ZlibCommand::DeflateRaw);
define_async_function!(gunzip, zlib_converter, ZlibCommand::Gunzip);

enum BrotliCommand {
    Compress,
    Decompress,
}

fn brotli_converter<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    command: BrotliCommand,
) -> Result<Value<'js>> {
    let src = if value.is_string() {
        let string = value.as_string().unwrap().to_string()?;
        string.as_bytes().to_vec()
    } else if value.is_array() {
        get_array_bytes(&ctx, &value, 0, None)?.unwrap()
    } else {
        let buffer = ArrayBufferView::from_js(&ctx, value)?;
        buffer.as_bytes().unwrap().to_vec()
    };

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        BrotliCommand::Compress => BrotliEncoder::new(&src[..]).read_to_end(&mut dst)?,
        BrotliCommand::Decompress => BrotliDecoder::new(&src[..]).read_to_end(&mut dst)?,
    };

    Buffer(dst).into_js(&ctx)
}

define_sync_function!(compress_sync, brotli_converter, BrotliCommand::Compress);
define_sync_function!(decompress_sync, brotli_converter, BrotliCommand::Decompress);

define_async_function!(compress, brotli_converter, BrotliCommand::Compress);
define_async_function!(decompress, brotli_converter, BrotliCommand::Decompress);

pub struct ZlibModule;

impl ModuleDef for ZlibModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("deflate")?;
        declare.declare("deflateSync")?;
        declare.declare("deflateRaw")?;
        declare.declare("deflateRawSync")?;
        declare.declare("gunzip")?;
        declare.declare("gunzipSync")?;
        declare.declare("brotliCompress")?;
        declare.declare("brotliDecompress")?;
        declare.declare("brotliCompressSync")?;
        declare.declare("brotliDecompressSync")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("deflate", Func::from(deflate))?;
            default.set("deflateSync", Func::from(deflate_sync))?;
            default.set("deflateRaw", Func::from(deflate_raw))?;
            default.set("deflateRawSync", Func::from(deflate_raw_sync))?;
            default.set("gunzip", Func::from(gunzip))?;
            default.set("gunzipSync", Func::from(gunzip_sync))?;
            default.set("brotliCompress", Func::from(compress))?;
            default.set("brotliDecompress", Func::from(decompress))?;
            default.set("brotliCompressSync", Func::from(compress_sync))?;
            default.set("brotliDecompressSync", Func::from(decompress_sync))?;
            Ok(())
        })
    }
}

impl From<ZlibModule> for ModuleInfo<ZlibModule> {
    fn from(val: ZlibModule) -> Self {
        ModuleInfo {
            name: "zlib",
            module: val,
        }
    }
}
