// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod access;
mod mkdir;
mod read_dir;
mod read_file;
mod rm;
mod stats;
mod write_file;

use crate::fs::access::access_sync;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Async, Func},
};
use rquickjs::{Class, Ctx, Object, Result};

use crate::module::export_default;

use self::access::access;
use self::read_dir::{read_dir, read_dir_sync, Dirent};
use self::read_file::read_file;
use self::rm::{rmdir, rmfile};
use self::stats::{stat_fn, Stat};
use self::write_file::write_file;
use crate::fs::mkdir::{mkdir, mkdir_sync, mkdtemp, mkdtemp_sync};

pub const CONSTANT_F_OK: u32 = 0;
pub const CONSTANT_R_OK: u32 = 4;
pub const CONSTANT_W_OK: u32 = 2;
pub const CONSTANT_X_OK: u32 = 1;

pub struct FsPromisesModule;

impl ModuleDef for FsPromisesModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("access")?;
        declare.declare("open")?;
        declare.declare("readFile")?;
        declare.declare("writeFile")?;
        declare.declare("appendFile")?;
        declare.declare("copyFile")?;
        declare.declare("rename")?;
        declare.declare("readdir")?;
        declare.declare("mkdir")?;
        declare.declare("mkdtemp")?;
        declare.declare("rm")?;
        declare.declare("rmdir")?;
        declare.declare("stat")?;
        declare.declare("constants")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        Class::<Dirent>::register(ctx)?;
        Class::<Stat>::register(ctx)?;

        export_default(ctx, exports, |default| {
            export_promises(ctx, default)?;

            Ok(())
        })
    }
}

pub struct FsModule;

impl ModuleDef for FsModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("promises")?;
        declare.declare("accessSync")?;
        declare.declare("mkdirSync")?;
        declare.declare("mkdtempSync")?;
        declare.declare("readdirSync")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        Class::<Dirent>::register(ctx)?;
        Class::<Stat>::register(ctx)?;

        export_default(ctx, exports, |default| {
            let promises = Object::new(ctx.clone())?;
            export_promises(ctx, &promises)?;

            default.set("promises", promises)?;
            default.set("accessSync", Func::from(access_sync))?;
            default.set("mkdirSync", Func::from(mkdir_sync))?;
            default.set("mkdtempSync", Func::from(mkdtemp_sync))?;
            default.set("readdirSync", Func::from(read_dir_sync))?;

            Ok(())
        })
    }
}

fn export_promises<'js>(ctx: &Ctx<'js>, exports: &Object<'js>) -> Result<()> {
    let constants = Object::new(ctx.clone())?;
    constants.set("F_OK", CONSTANT_F_OK)?;
    constants.set("R_OK", CONSTANT_R_OK)?;
    constants.set("W_OK", CONSTANT_W_OK)?;
    constants.set("X_OK", CONSTANT_X_OK)?;

    exports.set("readdir", Func::from(Async(read_dir)))?;
    exports.set("readFile", Func::from(Async(read_file)))?;
    exports.set("writeFile", Func::from(Async(write_file)))?;
    exports.set("mkdir", Func::from(Async(mkdir)))?;
    exports.set("mkdtemp", Func::from(Async(mkdtemp)))?;
    exports.set("rmdir", Func::from(Async(rmdir)))?;
    exports.set("rm", Func::from(Async(rmfile)))?;
    exports.set("stat", Func::from(Async(stat_fn)))?;
    exports.set("access", Func::from(Async(access)))?;

    exports.set("constants", constants)?;

    Ok(())
}
