// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{Span, SpanCompare};
use llrt_utils::result::ResultExt;
use rquickjs::{
    prelude::{Coerced, Opt},
    Ctx, Exception, FromJs, Object, Result, Value,
};

use super::relative_to::RelativeTo;

pub(crate) trait SpanExt {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Span>;
    fn from_object_with_fields(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Span>;
    fn span_with(self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Span>;
    fn into_span_compare(
        ctx: &Ctx<'_>,
        span: &Span,
        value: &Opt<Value<'_>>,
    ) -> Result<(SpanCompare<'static>, Option<RelativeTo>)>;
}

impl SpanExt for Span {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        into_span(ctx, None, obj, false)
    }

    fn from_object_with_fields(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        into_span(ctx, None, obj, true)
    }

    fn span_with(self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        let obj = value
            .as_object()
            .or_throw_type(ctx, "Cannot convert value to object")?;

        into_span(ctx, Some(self), obj, false)
    }

    fn into_span_compare(
        ctx: &Ctx<'_>,
        span: &Span,
        value: &Opt<Value<'_>>,
    ) -> Result<(SpanCompare<'static>, Option<RelativeTo>)> {
        let Some(ref value) = value.0 else {
            return Ok((span.into(), None));
        };
        if let Some(object) = value.as_object() {
            let relative = object.get::<_, Value>("relativeTo")?;
            return Ok((span.into(), RelativeTo::from(ctx, Some(&relative))?));
        }
        Ok((span.into(), None))
    }
}

fn into_span(ctx: &Ctx<'_>, span: Option<Span>, obj: &Object<'_>, required: bool) -> Result<Span> {
    let mut span = span.unwrap_or_default();
    let mut has_field = false;
    macro_rules! field {
        ($name:literal, $method:ident) => {
            if let Some(value) = duration_field(ctx, obj, $name)? {
                has_field = true;
                span = span.$method(value).or_throw_range(ctx, "")?;
            }
        };
    }
    field!("days", try_days);
    field!("hours", try_hours);
    field!("microseconds", try_microseconds);
    field!("milliseconds", try_milliseconds);
    field!("minutes", try_minutes);
    field!("months", try_months);
    field!("nanoseconds", try_nanoseconds);
    field!("seconds", try_seconds);
    field!("weeks", try_weeks);
    field!("years", try_years);
    if required && !has_field {
        return Err(Exception::throw_type(
            ctx,
            "Duration must have at least one field",
        ));
    }
    Ok(span)
}

fn duration_field(ctx: &Ctx<'_>, obj: &Object<'_>, name: &str) -> Result<Option<i64>> {
    let value = obj.get::<_, Value>(name)?;
    if value.is_undefined() {
        return Ok(None);
    }
    let number = Coerced::<f64>::from_js(obj.ctx(), value)?.0;
    if !number.is_finite()
        || number.fract() != 0.0
        || number < i64::MIN as f64
        || number >= -(i64::MIN as f64)
    {
        return Err(Exception::throw_range(ctx, "Invalid duration field"));
    }
    Ok(Some(number as i64))
}
