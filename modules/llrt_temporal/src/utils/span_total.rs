// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{SpanTotal, Unit};
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result, Value};

use super::get_duration_unit;
use super::span::into_span_relative_to;

pub(crate) trait SpanTotalExt<'a> {
    fn from_value(ctx: &Ctx<'_>, value: &Value<'a>) -> Result<SpanTotal<'a>>;
}

impl<'a> SpanTotalExt<'a> for SpanTotal<'a> {
    fn from_value(ctx: &Ctx<'_>, value: &Value<'a>) -> Result<Self> {
        if let Some(obj) = value.as_object() {
            let relative_to = obj.get::<_, Value>("relativeTo").ok();
            let unit = obj.get::<_, String>("unit").ok();
            return into_span_total(ctx, &relative_to, &unit);
        }

        let unit = value.as_string().and_then(|s| s.to_string().ok());
        into_span_total(ctx, &None, &unit)
    }
}

fn into_span_total<'a>(
    ctx: &Ctx,
    relative_to: &Option<Value<'a>>,
    unit: &Option<String>,
) -> Result<SpanTotal<'a>> {
    let relative_to = into_span_relative_to(relative_to);
    let unit = get_duration_unit(ctx, unit)?;
    let unit = unit.or_throw_range(ctx, "Invalid unit")?;

    if let Some(relative_to) = relative_to {
        Ok((unit, relative_to).into())
    } else if unit < Unit::Day {
        let span_total: SpanTotal = unit.into();
        Ok(span_total.days_are_24_hours())
    } else {
        Ok(unit.into())
    }
}
