// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::ModuleInfo;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Opt,
    Ctx, Exception, Function, Result, Type, Value,
};

fn ok(ctx: Ctx, value: Value, message: Opt<Value>) -> Result<()> {
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
        Type::Array
        | Type::BigInt
        | Type::Constructor
        | Type::Exception
        | Type::Function
        | Type::Symbol
        | Type::Object => {
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
        "AssertionError: The expression was evaluated to a falsy value",
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
        let ok_function = Function::new(ctx.clone(), ok)?.with_name("ok")?;
        ok_function.set("ok", ok_function.clone())?;

        exports.export("ok", ok_function.clone())?;
        exports.export("default", ok_function)?;
        Ok(())
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
