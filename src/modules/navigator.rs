// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Ctx, Object, Result, Value,
};

use crate::modules::module::export_default;

use crate::VERSION;

fn get_user_agent() -> String {
    format!("llrt {}", VERSION)
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let navigator = Object::new(ctx.clone())?;

    navigator.set("userAgent", get_user_agent())?;

    globals.set("navigator", navigator)?;

    Ok(())
}

pub struct NavigatorModule;

impl ModuleDef for NavigatorModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("userAgent")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let navigator: Object = globals.get("navigator")?;

        export_default(ctx, exports, |default| {
            for name in navigator.keys::<String>() {
                let name = name?;
                let value: Value = navigator.get(&name)?;
                default.set(name, value)?;
            }

            Ok(())
        })?;

        Ok(())
    }
}
