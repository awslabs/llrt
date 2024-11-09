// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    Ctx, Exception, Result, Type, Value,
};

fn assert(ctx: Ctx, value: Value, message: Opt<Value>) -> Result<()> {
    match value.type_of() {
        Type::Bool => {
            if value.as_bool().unwrap() {
                return Ok(());
            }
        },
        Type::Float | Type::Int => {
            if value.as_number().unwrap() != 0.0 {
                return Ok(());
            }
        },
        Type::String => {
            if !value.as_string().unwrap().to_string().unwrap().is_empty() {
                return Ok(());
            }
        },
        Type::Array | Type::Object => {
            return Ok(());
        },
        _ => {},
    }

    if let Some(obj) = message.0 {
        match obj.type_of() {
            Type::String => {
                let msg = obj.as_string().unwrap().to_string().unwrap();
                return Err(Exception::throw_message(&ctx, &msg));
            },
            Type::Exception => return Err(obj.as_exception().cloned().unwrap().throw()),
            _ => {},
        };
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
            default.set("ok", Func::from(assert))?;

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
