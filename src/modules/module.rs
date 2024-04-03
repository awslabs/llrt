// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Object, Result, Value,
};

pub fn export_default<'js, F>(ctx: &Ctx<'js>, exports: &mut Exports<'js>, f: F) -> Result<()>
where
    F: FnOnce(&Object<'js>) -> Result<()>,
{
    let default = Object::new(ctx.clone())?;
    f(&default)?;

    for name in default.keys::<String>() {
        let name = name?;
        let value: Value = default.get(&name)?;
        exports.export(name, value)?;
    }

    exports.export("default", default)?;

    Ok(())
}

pub struct ModuleModule;

fn create_require(ctx: Ctx<'_>) -> Result<Value<'_>> {
    ctx.globals().get("require")
}

impl ModuleDef for ModuleModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("createRequire")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("createRequire", Func::from(create_require))?;

            Ok(())
        })?;

        Ok(())
    }
}
