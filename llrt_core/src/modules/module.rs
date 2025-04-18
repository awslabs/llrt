// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    object::Accessor,
    prelude::Func,
    Ctx, Error, Exception, Function, Object, Result, Value,
};

use crate::libs::utils::module::{export_default, ModuleInfo};
use crate::modules::{
    require::{require, RequireState, CJS_IMPORT_PREFIX},
    ModuleNames,
};
use crate::utils::ctx::CtxExt;

pub struct ModuleModule;

fn create_require(ctx: Ctx<'_>) -> Result<Value<'_>> {
    ctx.globals()
        .get::<_, Function>("require")
        .map(|f| f.into())
        .map_err(|_| Exception::throw_reference(&ctx, "create_require is not supported"))
}

fn is_builtin(ctx: Ctx<'_>, name: String) -> bool {
    let binding = ctx.userdata::<ModuleNames>().unwrap();
    let module_list = binding.get_list();

    module_list.contains(&name)
}

impl ModuleDef for ModuleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("builtinModules")?;
        declare.declare("createRequire")?;
        declare.declare("isBuiltin")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let binding = ctx.userdata::<ModuleNames>().unwrap();
            let module_list = binding.get_list();

            default.set("builtinModules", module_list)?;
            default.set("createRequire", Func::from(create_require))?;
            default.set("isBuiltin", Func::from(is_builtin))?;

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

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    let _ = ctx.store_userdata(RefCell::new(RequireState::default()));

    let exports_accessor = Accessor::new(
        |ctx| {
            struct Args<'js>(Ctx<'js>);
            let Args(ctx) = Args(ctx);
            let name = ctx.get_script_or_module_name()?;
            let name = name.trim_start_matches(CJS_IMPORT_PREFIX);

            let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
            let mut state = binding.borrow_mut();

            if let Some(value) = state.exports.get(name) {
                Ok::<_, Error>(value.clone())
            } else {
                let obj = Object::new(ctx.clone())?.into_value();
                state.exports.insert(name.into(), obj.clone());
                Ok::<_, Error>(obj)
            }
        },
        |ctx, exports| {
            struct Args<'js>(Ctx<'js>, Value<'js>);
            let Args(ctx, exports) = Args(ctx, exports);
            let name = ctx.get_script_or_module_name()?;
            let name = name.trim_start_matches(CJS_IMPORT_PREFIX);
            let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
            let mut state = binding.borrow_mut();
            state.exports.insert(name.into(), exports);
            Ok::<_, Error>(())
        },
    )
    .configurable()
    .enumerable();

    globals.prop("exports", exports_accessor)?;
    globals.set("require", Func::from(require))?;

    let module = Object::new(ctx.clone())?;
    module.prop("exports", exports_accessor)?;
    globals.prop("module", module)?;

    Ok(())
}
