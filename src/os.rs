// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result,
};

use crate::{module::export_default, process::get_platform};

static OS_INFO: Lazy<(String, String, String)> = Lazy::new(|| {
    if let Ok(uts) = uname::uname() {
        return (uts.sysname, uts.release, uts.version);
    }
    (
        String::from("n/a"),
        String::from("n/a"),
        String::from("n/a"),
    )
});

fn get_type() -> &'static str {
    &OS_INFO.0
}

fn get_release() -> &'static str {
    &OS_INFO.1
}

fn get_version() -> &'static str {
    &OS_INFO.2
}

fn get_tmp_dir() -> String {
    env::temp_dir().to_string_lossy().to_string()
}

pub struct OsModule;

impl ModuleDef for OsModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("type")?;
        declare.declare("release")?;
        declare.declare("tmpdir")?;
        declare.declare("platform")?;
        declare.declare("version")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
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
