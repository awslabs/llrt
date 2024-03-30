// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Object, Result, Value,
};

use crate::module::export_default;

use crate::STARTED;

use once_cell::sync::Lazy;

use chrono::Utc;

static TIME_ORIGIN: Lazy<f64> = Lazy::new(|| (Utc::now().timestamp_micros() as f64) / 1e3);

fn get_time_origin() -> f64 {
    *TIME_ORIGIN
}

fn now() -> f64 {
    let started = unsafe { STARTED.assume_init() };
    let elapsed = started.elapsed();

    elapsed.as_secs_f64() + (elapsed.as_micros() as f64) / 1e3
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let performance = Object::new(ctx.clone())?;

    performance.set("timeOrigin", get_time_origin())?;
    performance.set("now", Func::from(now))?;

    globals.set("performance", performance)?;

    Ok(())
}

pub struct PerformanceModule;

impl ModuleDef for PerformanceModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("timeOrigin")?;
        declare.declare("now")?;
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
