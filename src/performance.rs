// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::atomic::Ordering;

use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Object, Result, Value,
};

use crate::module::export_default;

use chrono::Utc;

use crate::vm::TIME_ORIGIN;

fn get_time_origin() -> f64 {
    let time_origin = TIME_ORIGIN.load(Ordering::Relaxed) as f64;

    time_origin / 1e6
}

fn now() -> f64 {
    let now = Utc::now().timestamp_nanos_opt().unwrap_or_default() as f64;
    let started = TIME_ORIGIN.load(Ordering::Relaxed) as f64;

    (now - started) / 1e6
}

fn to_json(ctx: Ctx<'_>) -> Result<Object<'_>> {
    let obj = Object::new(ctx.clone())?;

    obj.set("timeOrigin", get_time_origin())?;

    Ok(obj)
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let performance = Object::new(ctx.clone())?;

    performance.set("timeOrigin", get_time_origin())?;
    performance.set("now", Func::from(now))?;
    performance.set(PredefinedAtom::ToJSON, Func::from(to_json))?;

    globals.set("performance", performance)?;

    Ok(())
}

pub struct PerformanceModule;

impl ModuleDef for PerformanceModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("timeOrigin")?;
        declare.declare("now")?;
        declare.declare("toJSON")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let performance: Object = globals.get("performance")?;

        export_default(ctx, exports, |default| {
            for name in performance.keys::<String>() {
                let name = name?;
                let value: Value = performance.get(&name)?;
                default.set(name, value)?;
            }

            Ok(())
        })?;

        Ok(())
    }
}
