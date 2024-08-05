// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::Read;

use brotlic::{CompressorReader as BrotliEncoder, DecompressorReader as BrotliDecoder};
use flate2::{
    read::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder},
    Compression,
};
use llrt_utils::{
    bytes::get_bytes, ctx::CtxExtension, module::export_default, object::ObjectExt,
    result::ResultExt,
};
use rquickjs::function::Func;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Opt, Rest},
    Ctx, Error, Exception, Function, IntoJs, Null, Result, Value,
};

use crate::ModuleInfo;

use super::buffer::Buffer;

macro_rules! define_sync_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub fn $fn_name<'js>(
            ctx: Ctx<'js>,
            value: Value<'js>,
            options: Opt<Value<'js>>,
        ) -> Result<Value<'js>> {
            $converter(ctx.clone(), value, options, $command)
        }
    };
}

macro_rules! define_cb_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub fn $fn_name<'js>(
            ctx: Ctx<'js>,
            value: Value<'js>,
            args: Rest<Value<'js>>,
        ) -> Result<()> {
            let mut args_iter = args.0.into_iter().rev();
            let cb: Function = args_iter
                .next()
                .and_then(|v| v.into_function())
                .or_throw_msg(&ctx, "Callback parameter is not a function")?;
            let options = match args_iter.next() {
                Some(v) => Opt(Some(v)),
                None => Opt(None),
            };

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
    Deflate,
    DeflateRaw,
    Gzip,
    Inflate,
    InflateRaw,
    Gunzip,
}

fn zlib_converter<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    options: Opt<Value<'js>>,
    command: ZlibCommand,
) -> Result<Value<'js>> {
    let src = get_bytes(&ctx, value)?;

    let mut level = Compression::default();
    if let Some(options) = options.0 {
        if let Some(opt) = options.get_optional("level")? {
            level = Compression::new(opt);
        }
    }

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        ZlibCommand::Deflate => ZlibEncoder::new(&src[..], level).read_to_end(&mut dst)?,
        ZlibCommand::DeflateRaw => DeflateEncoder::new(&src[..], level).read_to_end(&mut dst)?,
        ZlibCommand::Gzip => GzEncoder::new(&src[..], level).read_to_end(&mut dst)?,
        ZlibCommand::Inflate => ZlibDecoder::new(&src[..]).read_to_end(&mut dst)?,
        ZlibCommand::InflateRaw => DeflateDecoder::new(&src[..]).read_to_end(&mut dst)?,
        ZlibCommand::Gunzip => GzDecoder::new(&src[..]).read_to_end(&mut dst)?,
    };

    Buffer(dst).into_js(&ctx)
}

define_cb_function!(deflate, zlib_converter, ZlibCommand::Deflate);
define_sync_function!(deflate_sync, zlib_converter, ZlibCommand::Deflate);

define_cb_function!(deflate_raw, zlib_converter, ZlibCommand::DeflateRaw);
define_sync_function!(deflate_raw_sync, zlib_converter, ZlibCommand::DeflateRaw);

define_cb_function!(gzip, zlib_converter, ZlibCommand::Gzip);
define_sync_function!(gzip_sync, zlib_converter, ZlibCommand::Gzip);

define_cb_function!(inflate, zlib_converter, ZlibCommand::Inflate);
define_sync_function!(inflate_sync, zlib_converter, ZlibCommand::Inflate);

define_cb_function!(inflate_raw, zlib_converter, ZlibCommand::InflateRaw);
define_sync_function!(inflate_raw_sync, zlib_converter, ZlibCommand::InflateRaw);

define_cb_function!(gunzip, zlib_converter, ZlibCommand::Gunzip);
define_sync_function!(gunzip_sync, zlib_converter, ZlibCommand::Gunzip);

enum BrotliCommand {
    Compress,
    Decompress,
}

fn brotli_converter<'js>(
    ctx: Ctx<'js>,
    value: Value<'js>,
    _options: Opt<Value<'js>>,
    command: BrotliCommand,
) -> Result<Value<'js>> {
    let src = get_bytes(&ctx, value)?;

    let mut dst: Vec<u8> = Vec::with_capacity(src.len());

    let _ = match command {
        BrotliCommand::Compress => BrotliEncoder::new(&src[..]).read_to_end(&mut dst)?,
        BrotliCommand::Decompress => BrotliDecoder::new(&src[..]).read_to_end(&mut dst)?,
    };

    Buffer(dst).into_js(&ctx)
}

define_cb_function!(br_comp, brotli_converter, BrotliCommand::Compress);
define_sync_function!(br_comp_sync, brotli_converter, BrotliCommand::Compress);

define_cb_function!(br_decomp, brotli_converter, BrotliCommand::Decompress);
define_sync_function!(br_decomp_sync, brotli_converter, BrotliCommand::Decompress);

pub struct ZlibModule;

impl ModuleDef for ZlibModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("deflate")?;
        declare.declare("deflateSync")?;

        declare.declare("deflateRaw")?;
        declare.declare("deflateRawSync")?;

        declare.declare("gzip")?;
        declare.declare("gzipSync")?;

        declare.declare("inflate")?;
        declare.declare("inflateSync")?;

        declare.declare("inflateRaw")?;
        declare.declare("inflateRawSync")?;

        declare.declare("gunzip")?;
        declare.declare("gunzipSync")?;

        declare.declare("brotliCompress")?;
        declare.declare("brotliCompressSync")?;

        declare.declare("brotliDecompress")?;
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

            default.set("gzip", Func::from(gzip))?;
            default.set("gzipSync", Func::from(gzip_sync))?;

            default.set("inflate", Func::from(inflate))?;
            default.set("inflateSync", Func::from(inflate_sync))?;

            default.set("inflateRaw", Func::from(inflate_raw))?;
            default.set("inflateRawSync", Func::from(inflate_raw_sync))?;

            default.set("gunzip", Func::from(gunzip))?;
            default.set("gunzipSync", Func::from(gunzip_sync))?;

            default.set("brotliCompress", Func::from(br_comp))?;
            default.set("brotliCompressSync", Func::from(br_comp_sync))?;

            default.set("brotliDecompress", Func::from(br_decomp))?;
            default.set("brotliDecompressSync", Func::from(br_decomp_sync))?;

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
