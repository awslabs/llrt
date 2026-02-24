// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::Span;
use llrt_utils::result::ResultExt;
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

    macro_rules! apply {
        ($key:literal, $ty:ty, $method:ident) => {
            if let Ok(v) = obj.get::<_, $ty>($key) {
                span = span.$method(v).map_err_range(ctx)?;
            }
        };
    }

    apply!("days", i64, try_days);
    apply!("hours", i64, try_hours);
    apply!("microseconds", i64, try_microseconds);
    apply!("milliseconds", i64, try_milliseconds);
    apply!("minutes", i64, try_minutes);
    apply!("months", i64, try_months);
    apply!("nanoseconds", i64, try_nanoseconds);
    apply!("seconds", i64, try_seconds);
    apply!("weeks", i64, try_weeks);
    apply!("years", i64, try_years);
    Ok(span)
}
