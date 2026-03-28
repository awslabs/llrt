// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod round;

use jiff::civil::Time;
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Object, Result, Value};

pub trait TimeExt {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Time>;
    fn time_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Time>;
}

impl TimeExt for Time {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        from_obj(ctx, obj)
    }

    fn time_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        let obj = value
            .as_object()
            .or_throw_type(ctx, "Cannot convert value to object")?;

        into_time(ctx, self, obj)
    }
}

fn from_obj(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Time> {
    let hour = obj.get::<_, i8>("hour").unwrap_or_default();
    let minute = obj.get::<_, i8>("minute").unwrap_or_default();
    let second = obj.get::<_, i8>("second").unwrap_or_default();

    let millis = obj.get::<_, i32>("millisecond").unwrap_or_default();
    let micros = obj.get::<_, i32>("microsecond").unwrap_or_default();
    let nanos = obj.get::<_, i32>("nanosecond").unwrap_or_default();
    let subsec_ns = nanos + micros * 1_000 + millis * 1_000_000;

    let time = Time::new(hour, minute, second, subsec_ns).or_throw_range(ctx, "")?;
    Ok(time)
}

fn into_time(ctx: &Ctx<'_>, time: &Time, obj: &Object<'_>) -> Result<Time> {
    let mut time = time.with();
    if let Ok(v) = obj.get::<_, i8>("hour") {
        time = time.hour(v);
    }
    if let Ok(v) = obj.get::<_, i8>("minute") {
        time = time.minute(v);
    }
    if let Ok(v) = obj.get::<_, i8>("second") {
        time = time.second(v);
    }
    if let Ok(v) = obj.get::<_, i16>("millisecond") {
        time = time.millisecond(v);
    }
    if let Ok(v) = obj.get::<_, i16>("microsecond") {
        time = time.microsecond(v);
    }
    if let Ok(v) = obj.get::<_, i16>("nanosecond") {
        time = time.nanosecond(v);
    }
    time.build().or_throw_range(ctx, "")
}

pub(crate) fn fill_from_iter<'js, I>(obj: &Object<'js>, iter: &mut I, calendar: bool) -> Result<()>
where
    I: Iterator<Item = Value<'js>>,
{
    if let Some(v) = iter.next() {
        obj.set("hour", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("minute", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("second", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("millisecond", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("microsecond", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("nanosecond", v)?;
    }
    if calendar {
        if let Some(v) = iter.next() {
            obj.set("calendar", v)?;
        }
    }
    Ok(())
}

pub(crate) fn fill_duration_from_iter<'js, I>(obj: &Object<'js>, iter: &mut I) -> Result<()>
where
    I: Iterator<Item = Value<'js>>,
{
    if let Some(v) = iter.next() {
        obj.set("hours", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("minutes", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("seconds", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("milliseconds", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("microseconds", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("nanoseconds", v)?;
    }
    Ok(())
}
