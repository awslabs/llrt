// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{SpanTotal, Unit};
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result, Value};

use super::get_duration_unit;
use super::relative_to::RelativeTo;

pub(crate) trait SpanTotalExt {
    fn from_value(
        ctx: &Ctx<'_>,
        value: &Value<'_>,
    ) -> Result<(SpanTotal<'static>, Option<RelativeTo>)>;
}

impl SpanTotalExt for SpanTotal<'static> {
    fn from_value(ctx: &Ctx<'_>, value: &Value<'_>) -> Result<(Self, Option<RelativeTo>)> {
        if let Some(obj) = value.as_object() {
            let relative_to = Some(&obj.get::<_, Value>("relativeTo")?);
            let unit = obj.get::<_, String>("unit").ok();
            return into_span_total(ctx, relative_to, unit.as_ref());
        }

        let unit = value
            .as_string()
            .and_then(|s| s.to_string().ok())
            .or_throw_type(ctx, "Cannot convert value to string")?;

        into_span_total(ctx, None, Some(&unit))
    }
}

fn into_span_total(
    ctx: &Ctx,
    relative_to: Option<&Value<'_>>,
    unit: Option<&String>,
) -> Result<(SpanTotal<'static>, Option<RelativeTo>)> {
    let relative_to = RelativeTo::from(ctx, relative_to)?;
    let unit = get_duration_unit(ctx, unit)?;
    let unit = unit.or_throw_range(ctx, "Invalid unit")?;

    if relative_to.is_none() && unit < Unit::Day {
        let span_total: SpanTotal = unit.into();
        Ok((span_total.days_are_24_hours(), relative_to))
    } else {
        Ok((unit.into(), relative_to))
    }
}
