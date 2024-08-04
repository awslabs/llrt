// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::Read;

use brotlic::{CompressorReader as BrotliEncoder, DecompressorReader as BrotliDecoder};
use flate2::{
    read::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder},
    Compression,
};
use llrt_utils::{ctx::CtxExtension, module::export_default, object::ObjectExt};
use rquickjs::function::Func;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Opt,
    Ctx, Error, Exception, FromJs, Function, IntoJs, Null, Object, Result, Value,
};
use zstd::{
    stream::read::{Decoder as ZstdDecoder, Encoder as ZstdEncoder},
    zstd_safe::CompressionLevel,
};

use crate::{utils::array_buffer::ArrayBufferView, ModuleInfo};

use super::buffer::Buffer;

macro_rules! define_sync_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub fn $fn_name<'js>(
            ctx: Ctx<'js>,
            value: Value<'js>,
            options: Opt<Object<'js>>,
        ) -> Result<Value<'js>> {
            $converter(ctx.clone(), value, options, $command)
        }
    };
}

macro_rules! define_async_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub fn $fn_name<'js>(
            ctx: Ctx<'js>,
            value: Value<'js>,
            options: Opt<Object<'js>>,
            cb: Function<'js>,
        ) -> Result<()> {
            ctx.clone().spawn_exit(async move {
                match $converter(ctx.clone(), value, options, $command) {
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
    Inflate,
    InflateRaw,
    Gzip,
    Deflate,
    DeflateRaw,
    Gunzip,
}

fn zlib_converter<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    options: Opt<Object<'js>>,
    command: ZlibCommand,
) -> Result<Value<'js>> {
    let src = if value.is_string() {
        let string = value.as_string().unwrap().to_string()?;
        string.as_bytes().to_vec()
    } else {
        let buffer = ArrayBufferView::from_js(&ctx, value)?;
        buffer.as_bytes().unwrap().to_vec()
    };

    let mut level = Compression::default();
    if let Some(options) = options.0 {
        if let Some(opt) = options.get_optional("level")? {
            level = Compression::new(opt);
        }
    }

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        ZlibCommand::Inflate => ZlibEncoder::new(&src[..], level).read_to_end(&mut dst)?,
        ZlibCommand::InflateRaw => DeflateEncoder::new(&src[..], level).read_to_end(&mut dst)?,
        ZlibCommand::Gzip => GzEncoder::new(&src[..], level).read_to_end(&mut dst)?,
        ZlibCommand::Deflate => ZlibDecoder::new(&src[..]).read_to_end(&mut dst)?,
        ZlibCommand::DeflateRaw => DeflateDecoder::new(&src[..]).read_to_end(&mut dst)?,
        ZlibCommand::Gunzip => GzDecoder::new(&src[..]).read_to_end(&mut dst)?,
    };

    Buffer(dst).into_js(&ctx)
}

define_async_function!(inflate, zlib_converter, ZlibCommand::Inflate);
define_sync_function!(inflate_sync, zlib_converter, ZlibCommand::Inflate);

define_async_function!(inflate_raw, zlib_converter, ZlibCommand::InflateRaw);
define_sync_function!(inflate_raw_sync, zlib_converter, ZlibCommand::InflateRaw);

define_async_function!(gzip, zlib_converter, ZlibCommand::Gzip);
define_sync_function!(gzip_sync, zlib_converter, ZlibCommand::Gzip);

define_async_function!(deflate, zlib_converter, ZlibCommand::Deflate);
define_sync_function!(deflate_sync, zlib_converter, ZlibCommand::Deflate);

define_async_function!(deflate_raw, zlib_converter, ZlibCommand::DeflateRaw);
define_sync_function!(deflate_raw_sync, zlib_converter, ZlibCommand::DeflateRaw);

define_async_function!(gunzip, zlib_converter, ZlibCommand::Gunzip);
define_sync_function!(gunzip_sync, zlib_converter, ZlibCommand::Gunzip);

enum BrotliCommand {
    Compress,
    Decompress,
}

fn brotli_converter<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    _options: Opt<Object<'js>>,
    command: BrotliCommand,
) -> Result<Value<'js>> {
    let src = if value.is_string() {
        let string = value.as_string().unwrap().to_string()?;
        string.as_bytes().to_vec()
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

define_async_function!(br_compress, brotli_converter, BrotliCommand::Compress);
define_sync_function!(br_compress_sync, brotli_converter, BrotliCommand::Compress);

define_async_function!(br_decompress, brotli_converter, BrotliCommand::Decompress);
define_sync_function!(
    br_decompress_sync,
    brotli_converter,
    BrotliCommand::Decompress
);

enum ZstandardCommand {
    Compress,
    Decompress,
}

fn zstandard_converter<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    options: Opt<Object<'js>>,
    command: ZstandardCommand,
) -> Result<Value<'js>> {
    let src = if value.is_string() {
        let string = value.as_string().unwrap().to_string()?;
        string.as_bytes().to_vec()
    } else {
        let buffer = ArrayBufferView::from_js(&ctx, value)?;
        buffer.as_bytes().unwrap().to_vec()
    };

    let mut level = CompressionLevel::default();
    if let Some(options) = options.0 {
        if let Some(opt) = options.get_optional("level")? {
            level = opt;
        }
    }

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        ZstandardCommand::Compress => ZstdEncoder::new(&src[..], level)?.read_to_end(&mut dst)?,
        ZstandardCommand::Decompress => ZstdDecoder::new(&src[..])?.read_to_end(&mut dst)?,
    };

    Buffer(dst).into_js(&ctx)
}

define_async_function!(
    zstd_compress,
    zstandard_converter,
    ZstandardCommand::Compress
);
define_sync_function!(
    zstd_compress_sync,
    zstandard_converter,
    ZstandardCommand::Compress
);

define_async_function!(
    zstd_decompress,
    zstandard_converter,
    ZstandardCommand::Decompress
);
define_sync_function!(
    zstd_decompress_sync,
    zstandard_converter,
    ZstandardCommand::Decompress
);

pub struct ZlibModule;

impl ModuleDef for ZlibModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("inflate")?;
        declare.declare("inflateSync")?;

        declare.declare("inflateRaw")?;
        declare.declare("inflateRawSync")?;

        declare.declare("gzip")?;
        declare.declare("gzipSync")?;

        declare.declare("deflate")?;
        declare.declare("deflateSync")?;

        declare.declare("deflateRaw")?;
        declare.declare("deflateRawSync")?;

        declare.declare("gunzip")?;
        declare.declare("gunzipSync")?;

        declare.declare("brotliCompress")?;
        declare.declare("brotliCompressSync")?;

        declare.declare("brotliDecompress")?;
        declare.declare("brotliDecompressSync")?;

        declare.declare("zstandardCompress")?;
        declare.declare("zstandardCompressSync")?;

        declare.declare("zstandardDecompress")?;
        declare.declare("zstandardDecompressSync")?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("inflate", Func::from(inflate))?;
            default.set("inflateSync", Func::from(inflate_sync))?;

            default.set("inflateRaw", Func::from(inflate_raw))?;
            default.set("inflateRawSync", Func::from(inflate_raw_sync))?;

            default.set("gzip", Func::from(gzip))?;
            default.set("gzipSync", Func::from(gzip_sync))?;

            default.set("deflate", Func::from(deflate))?;
            default.set("deflateSync", Func::from(deflate_sync))?;

            default.set("deflateRaw", Func::from(deflate_raw))?;
            default.set("deflateRawSync", Func::from(deflate_raw_sync))?;

            default.set("gunzip", Func::from(gunzip))?;
            default.set("gunzipSync", Func::from(gunzip_sync))?;

            default.set("brotliCompress", Func::from(br_compress))?;
            default.set("brotliCompressSync", Func::from(br_compress_sync))?;

            default.set("brotliDecompress", Func::from(br_decompress))?;
            default.set("brotliDecompressSync", Func::from(br_decompress_sync))?;

            default.set("zstandardCompress", Func::from(zstd_compress))?;
            default.set("zstandardCompressSync", Func::from(zstd_compress_sync))?;

            default.set("zstandardDecompress", Func::from(zstd_decompress))?;
            default.set("zstandardDecompressSync", Func::from(zstd_decompress_sync))?;

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
