// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{SpanRound, Unit};
use rquickjs::{Ctx, Result, Value};

use super::span::into_span_relative_to;
use super::{get_duration_unit, get_round_mode};

pub(crate) trait SpanRoundExt<'a> {
    fn from_value(ctx: &Ctx<'_>, value: &Value<'a>) -> Result<SpanRound<'a>>;
}

impl<'a> SpanRoundExt<'a> for SpanRound<'a> {
    fn from_value(ctx: &Ctx<'_>, value: &Value<'a>) -> Result<Self> {
        if let Some(obj) = value.as_object() {
            let largest_unit = obj.get::<_, String>("largestUnit").ok();
            let relative_to = obj.get::<_, Value>("relativeTo").ok();
            let increment = obj.get::<_, i64>("roundingIncrement").ok();
            let mode = obj.get::<_, String>("roundingMode").ok();
            let smallest_unit = obj.get::<_, String>("smallestUnit").ok();
            return into_span_round(
                ctx,
                &largest_unit,
                &relative_to,
                &increment,
                &mode,
                &smallest_unit,
            );
        }

        let unit = value.as_string().and_then(|s| s.to_string().ok());
        into_span_round(ctx, &None, &None, &None, &None, &unit)
    }
}

fn into_span_round<'a>(
    ctx: &Ctx,
    largest_unit: &Option<String>,
    relative_to: &Option<Value<'a>>,
    increment: &Option<i64>,
    mode: &Option<String>,
    smallest_unit: &Option<String>,
) -> Result<SpanRound<'a>> {
    let largest_unit = get_duration_unit(ctx, largest_unit)?;
    let relative_to = into_span_relative_to(relative_to);
    let increment = increment.unwrap_or(1);
    let mode = get_round_mode(ctx, mode)?;
    let smallest_unit = get_duration_unit(ctx, smallest_unit)?;

    let mut span_round = SpanRound::new().mode(mode).increment(increment);
    if let Some(largest_unit) = largest_unit {
        span_round = span_round.largest(largest_unit);
    }
    if let Some(smallest_unit) = smallest_unit {
        span_round = span_round.smallest(smallest_unit);
    }
    if let Some(relative_to) = relative_to {
        span_round = span_round.relative(relative_to);
    } else if largest_unit.is_some_and(|u| u < Unit::Day) {
        span_round = span_round.days_are_24_hours();
    }
    Ok(span_round)
}
