// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod duration;
pub mod instant;
mod now;
mod utils;
pub mod zoned_date_time;

use rquickjs::{atom::PredefinedAtom, prelude::Func, Class, Ctx, Exception, Object, Result, Value};

use crate::duration::Duration;
use crate::instant::Instant;
use crate::zoned_date_time::ZonedDateTime;

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let now = Object::new(ctx.clone())?;
    now.set("instant", Func::from(now::instant))?;
    now.set("zonedDateTimeISO", Func::from(now::zoned_datetime_iso))?;
    now.set(PredefinedAtom::SymbolToStringTag, "Temporal.Now")?;

    let temporal = Object::new(ctx.clone())?;
    Class::<Duration>::define(&temporal)?;
    Class::<Instant>::define(&temporal)?;
    Class::<ZonedDateTime>::define(&temporal)?;
    temporal.set("Now", now)?;

    ctx.globals().set("Temporal", temporal)?;
    Ok(())
}

pub fn extract_bigint_or_number(ctx: &Ctx<'_>, value: &Value<'_>) -> Result<i128> {
    if let Some(num) = value.as_number() {
        if !num.is_finite() {
            return Err(Exception::throw_message(ctx, "Invalid value"));
        }
        Ok(num as i128)
    } else if let Some(bigint) = value.as_big_int() {
        match bigint.clone().to_i64() {
            Ok(v) => Ok(v as i128),
            Err(_) => Err(Exception::throw_message(ctx, "BigInt value out of range")),
        }
    } else {
        Err(Exception::throw_message(ctx, "Expected number or BigInt"))
    }
}
