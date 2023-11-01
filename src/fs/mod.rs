mod access;
mod mkdir;
mod read_dir;
mod read_file;
mod rm;
mod stats;
mod write_file;

use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Async, Func},
};
use rquickjs::{Class, Ctx, Object, Result};

use crate::util::export_default;

use self::access::access;
use self::read_dir::{read_dir, Dirent};
use self::read_file::read_file;
use self::rm::{rmdir, rmfile};
use self::stats::{stat_fn, Stat};
use self::write_file::write_file;
use crate::fs::mkdir::{mkdir, mkdtemp};

pub const CONSTANT_F_OK: u32 = 0;
pub const CONSTANT_R_OK: u32 = 4;
pub const CONSTANT_W_OK: u32 = 2;
pub const CONSTANT_X_OK: u32 = 1;

pub struct FsPromisesModule;

impl ModuleDef for FsPromisesModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        delarations(declare)?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        Class::<Dirent>::register(ctx)?;
        Class::<Stat>::register(ctx)?;

        export_default(ctx, exports, |default| {
            let constants = Object::new(ctx.clone())?;
            constants.set("F_OK", CONSTANT_F_OK)?;
            constants.set("R_OK", CONSTANT_R_OK)?;
            constants.set("W_OK", CONSTANT_W_OK)?;
            constants.set("X_OK", CONSTANT_X_OK)?;

            default.set("readdir", Func::from(Async(read_dir)))?;
            default.set("readFile", Func::from(Async(read_file)))?;
            default.set("writeFile", Func::from(Async(write_file)))?;
            default.set("mkdir", Func::from(Async(mkdir)))?;
            default.set("mkdtemp", Func::from(Async(mkdtemp)))?;
            default.set("rmdir", Func::from(Async(rmdir)))?;
            default.set("rm", Func::from(Async(rmfile)))?;
            default.set("stat", Func::from(Async(stat_fn)))?;
            default.set("access", Func::from(Async(access)))?;

            default.set("constants", constants)?;

            Ok(())
        })
    }
}

fn delarations(declare: &mut Declarations) -> Result<()> {
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
