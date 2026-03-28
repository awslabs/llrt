// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod round;

use jiff::civil::{Date, DateTime, Time};
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Object, Result, Value};

use super::date::DateExt;
use super::time::TimeExt;

pub trait DateTimeExt {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<DateTime>;
    fn date_time_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<DateTime>;
}

impl DateTimeExt for DateTime {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        from_obj(ctx, obj)
    }

    fn date_time_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        let obj = value
            .as_object()
            .or_throw_type(ctx, "Cannot convert value to object")?;

        into_date_time(ctx, self, obj)
    }
}

fn from_obj(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<DateTime> {
    let date = Date::from_object(ctx, obj)?;
    let time = Time::from_object(ctx, obj)?;
    let dt = DateTime::from_parts(date, time);
    Ok(dt)
}

fn into_date_time(ctx: &Ctx<'_>, dt: &DateTime, obj: &Object<'_>) -> Result<DateTime> {
    let mut dt = dt.with();
    if let Ok(v) = obj.get::<_, i16>("yaer") {
        dt = dt.year(v);
    }
    if let Ok(v) = obj.get::<_, i8>("month") {
        dt = dt.month(v);
    }
    if let Ok(v) = obj.get::<_, i8>("day") {
        dt = dt.day(v);
    }
    if let Ok(v) = obj.get::<_, i8>("hour") {
        dt = dt.hour(v);
    }
    if let Ok(v) = obj.get::<_, i8>("minute") {
        dt = dt.minute(v);
    }
    if let Ok(v) = obj.get::<_, i8>("second") {
        dt = dt.second(v);
    }
    if let Ok(v) = obj.get::<_, i16>("millisecond") {
        dt = dt.millisecond(v);
    }
    if let Ok(v) = obj.get::<_, i16>("microsecond") {
        dt = dt.microsecond(v);
    }
    if let Ok(v) = obj.get::<_, i16>("nanosecond") {
        dt = dt.nanosecond(v);
    }
    dt.build().or_throw_range(ctx, "")
}
