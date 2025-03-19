// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_logging::get_dimensions;
use llrt_utils::object::ObjectExt;
use rquickjs::{
    context::EvalOptions,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    Ctx, Object, Result, Value,
};

use crate::{module_builder::ModuleInfo, modules::module::export_default};

fn load<'js>(ctx: Ctx<'js>, filename: String, options: Opt<Object<'js>>) -> Result<Value<'js>> {
    let mut eval_options = EvalOptions::default();
    eval_options.strict = false;
    eval_options.promise = true;

    if let Some(options) = options.0 {
        if let Some(global) = options.get_optional("global")? {
            eval_options.global = global;
        }

        if let Some(strict) = options.get_optional("strict")? {
            eval_options.strict = strict;
        }
    }

    ctx.eval_file_with_options(filename, eval_options)
}

fn print(value: String, stdout: Opt<bool>) {
    if stdout.0.unwrap_or_default() {
        println!("{value}");
    } else {
        eprintln!("{value}")
    }
}

pub struct LlrtUtilModule;

impl ModuleDef for LlrtUtilModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("dimensions")?;
        declare.declare("load")?;
        declare.declare("print")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("dimensions", Func::from(get_dimensions))?;
            default.set("load", Func::from(load))?;
            default.set("print", Func::from(print))?;
            Ok(())
        })
    }
}

impl From<LlrtUtilModule> for ModuleInfo<LlrtUtilModule> {
    fn from(val: LlrtUtilModule) -> Self {
        ModuleInfo {
            name: "llrt:util",
            module: val,
        }
    }
}
