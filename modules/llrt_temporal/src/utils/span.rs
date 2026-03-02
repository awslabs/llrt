// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::Span;
use llrt_utils::result::{By, ResultExt};
use rquickjs::{Ctx, Exception, Object, Result, Value};

pub trait SpanExt {
    fn from_value(ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Span>;
    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Span>;
    fn with(self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Span>;
}

impl SpanExt for Span {
    fn from_value(ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        if let Some(obj) = value.as_object() {
            return into_span(ctx, None, obj);
        }
        Err(Exception::throw_type(ctx, "Expected object"))
    }

    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Self> {
        into_span(ctx, None, object)
    }

    fn with(self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        if let Some(obj) = value.as_object() {
            return into_span(ctx, Some(self), obj);
        }
        Err(Exception::throw_type(ctx, "Expected object"))
    }
}

fn into_span(ctx: &Ctx<'_>, span: Option<Span>, obj: &Object<'_>) -> Result<Span> {
    let mut span = span.unwrap_or_default();
    if let Ok(v) = obj.get::<_, i64>("days") {
        span = span.try_days(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("hours") {
        span = span.try_hours(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("microseconds") {
        span = span.try_microseconds(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("milliseconds") {
        span = span.try_milliseconds(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("minutes") {
        span = span.try_minutes(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("months") {
        span = span.try_months(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("nanoseconds") {
        span = span.try_nanoseconds(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("seconds") {
        span = span.try_seconds(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("weeks") {
        span = span.try_weeks(v).or_throw_by(ctx, By::Range)?;
    }
    if let Ok(v) = obj.get::<_, i64>("years") {
        span = span.try_years(v).or_throw_by(ctx, By::Range)?;
    }
    Ok(span)
}
