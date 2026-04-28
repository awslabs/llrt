// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{Unit, Zoned, ZonedDifference};
use rquickjs::{Ctx, Result, Value};

use crate::utils::{get_duration_unit, get_round_mode};

pub trait ZonedDifferenceExt<'a> {
    fn from_value(ctx: &Ctx<'_>, zoned: &'a Zoned, value: &Value) -> Result<ZonedDifference<'a>>;
}

impl<'a> ZonedDifferenceExt<'a> for ZonedDifference<'a> {
    fn from_value(ctx: &Ctx<'_>, zoned: &'a Zoned, value: &Value) -> Result<ZonedDifference<'a>> {
        if let Some(obj) = value.as_object() {
            let largest_unit = obj.get::<_, String>("largestUnit").ok();
            let increment = obj.get::<_, i64>("roundingIncrement").ok();
            let mode = obj.get::<_, String>("roundingMode").ok();
            let smallest_unit = obj.get::<_, String>("smallestUnit").ok();
            return into_zoned_deffierence(
                ctx.clone(),
                zoned,
                largest_unit,
                increment,
                mode,
                smallest_unit,
            );
        }

        let unit = value.as_string().and_then(|s| s.to_string().ok());
        into_zoned_deffierence(ctx.clone(), zoned, None, None, None, unit)
    }
}

fn into_zoned_deffierence<'a>(
    ctx: Ctx,
    zoned: &'a Zoned,
    largest_unit: Option<String>,
    increment: Option<i64>,
    mode: Option<String>,
    smallest_unit: Option<String>,
) -> Result<ZonedDifference<'a>> {
    let largest_unit = get_duration_unit(&ctx, &largest_unit)?.unwrap_or(Unit::Hour);
    let increment = increment.unwrap_or(1);
    let mode = get_round_mode(&ctx, &mode)?;
    let smallest_unit = get_duration_unit(&ctx, &smallest_unit)?.unwrap_or(Unit::Nanosecond);

    let zoned_deffierence = ZonedDifference::new(zoned)
        .mode(mode)
        .increment(increment)
        .largest(largest_unit)
        .smallest(smallest_unit);
    Ok(zoned_deffierence)
}
