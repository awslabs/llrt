// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub use llrt_utils::module::export_default;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result, Value,
};

use crate::module_builder::ModuleInfo;

pub struct ModuleModule;

fn create_require(ctx: Ctx<'_>) -> Result<Value<'_>> {
    ctx.globals().get("require")
}

impl ModuleDef for ModuleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createRequire")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("createRequire", Func::from(create_require))?;

            Ok(())
        })?;

        Ok(())
    }
}

impl From<ModuleModule> for ModuleInfo<ModuleModule> {
    fn from(val: ModuleModule) -> Self {
        ModuleInfo {
            name: "module",
            module: val,
        }
    }
}
