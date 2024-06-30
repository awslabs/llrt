// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::atomic::Ordering;

use rquickjs::{atom::PredefinedAtom, prelude::Func, Ctx, Object, Result};

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
