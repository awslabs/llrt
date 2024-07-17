// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use llrt_utils::module::export_default;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result,
};

#[cfg(unix)]
use self::unix::{get_release, get_type, get_version};
#[cfg(windows)]
use self::windows::{get_release, get_type, get_version};
use crate::module_info::ModuleInfo;
use crate::process::get_platform;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

fn get_tmp_dir() -> String {
    env::temp_dir().to_string_lossy().to_string()
}

pub struct OsModule;

impl ModuleDef for OsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("type")?;
        declare.declare("release")?;
        declare.declare("tmpdir")?;
        declare.declare("platform")?;
        declare.declare("version")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("type", Func::from(get_type))?;
            default.set("release", Func::from(get_release))?;
            default.set("tmpdir", Func::from(get_tmp_dir))?;
            default.set("platform", Func::from(get_platform))?;
            default.set("version", Func::from(get_version))?;

            Ok(())
        })
    }
}

impl From<OsModule> for ModuleInfo<OsModule> {
    fn from(val: OsModule) -> Self {
        ModuleInfo {
            name: "os",
            module: val,
        }
    }
}
