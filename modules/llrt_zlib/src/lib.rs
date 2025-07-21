// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    function::Func,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};

mod brotli;
mod zlib;
mod zstd;

use self::brotli::{br_comp, br_comp_sync, br_decomp, br_decomp_sync};
use self::zlib::{
    deflate, deflate_raw, deflate_raw_sync, deflate_sync, gunzip, gunzip_sync, gzip, gzip_sync,
    inflate, inflate_raw, inflate_raw_sync, inflate_sync,
};
use self::zstd::{zstd_comp, zstd_comp_sync, zstd_decomp, zstd_decomp_sync};

#[macro_export]
macro_rules! define_sync_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub(crate) fn $fn_name<'js>(
            ctx: Ctx<'js>,
            value: ObjectBytes<'js>,
            options: Opt<Value<'js>>,
        ) -> Result<Value<'js>> {
            $converter(ctx.clone(), value, options, $command)
        }
    };
}

#[macro_export]
macro_rules! define_cb_function {
    ($fn_name:ident, $converter:expr, $command:expr) => {
        pub(crate) fn $fn_name<'js>(
            ctx: Ctx<'js>,
            value: ObjectBytes<'js>,
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

        declare.declare("zstdCompress")?;
        declare.declare("zstdCompressSync")?;

        declare.declare("zstdDecompress")?;
        declare.declare("zstdDecompressSync")?;

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

            default.set("zstdCompress", Func::from(zstd_comp))?;
            default.set("zstdCompressSync", Func::from(zstd_comp_sync))?;

            default.set("zstdDecompress", Func::from(zstd_decomp))?;
            default.set("zstdDecompressSync", Func::from(zstd_decomp_sync))?;

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
