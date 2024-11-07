// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    Ctx, Exception, Result, Value,
};

fn ok(ctx: Ctx, value: Value, message: Opt<Value>) -> Result<()> {
    if value.as_bool().unwrap_or(false) {
        return Ok(());
    }
    if value.as_number().unwrap_or(0.0) != 0.0 {
        return Ok(());
    }
    if value.is_string() && !value.as_string().unwrap().to_string().unwrap().is_empty() {
        return Ok(());
    }
    if value.is_array() || value.is_object() {
        return Ok(());
    }

    if let Some(obj) = message.0 {
        if let Some(msg) = obj.as_string() {
            let msg = msg.to_string().unwrap();
            return Err(Exception::throw_message(&ctx, &msg));
        }
        if let Some(err) = obj.as_exception() {
            return Err(err.clone().throw());
        }
    }

    Err(Exception::throw_message(
        &ctx,
        "The expression was evaluated to a falsy value",
    ))
}

pub struct AssertModule;

impl ModuleDef for AssertModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("ok")?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("ok", Func::from(ok))?;

            Ok(())
        })
    }
}

impl From<AssertModule> for ModuleInfo<AssertModule> {
    fn from(val: AssertModule) -> Self {
        ModuleInfo {
            name: "assert",
            module: val,
        }
    }
}
