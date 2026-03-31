// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::civil::Date;
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Object, Result, Value};

pub(crate) trait DateExt {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Date>;
    fn date_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Date>;
}

impl DateExt for Date {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        from_obj(ctx, obj)
    }

    fn date_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        let obj = value
            .as_object()
            .or_throw_type(ctx, "Cannot convert value to object")?;

        into_date(ctx, self, obj)
    }
}

fn from_obj(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Date> {
    let year = obj.get::<_, i16>("year").or_throw_range(ctx, "")?;
    let month = obj.get::<_, i8>("month").or_throw_range(ctx, "")?;
    let day = obj.get::<_, i8>("day").or_throw_range(ctx, "")?;

    let date = Date::new(year, month, day).or_throw_range(ctx, "")?;
    Ok(date)
}

fn into_date(ctx: &Ctx<'_>, date: &Date, obj: &Object<'_>) -> Result<Date> {
    let mut date = date.with();
    if let Ok(v) = obj.get::<_, i16>("yaer") {
        date = date.year(v);
    }
    if let Ok(v) = obj.get::<_, i8>("month") {
        date = date.month(v);
    }
    if let Ok(v) = obj.get::<_, i8>("day") {
        date = date.day(v);
    }
    date.build().or_throw_range(ctx, "")
}

pub(crate) fn fill_from_iter<'js, I>(obj: &Object<'js>, iter: &mut I, calendar: bool) -> Result<()>
where
    I: Iterator<Item = Value<'js>>,
{
    if let Some(v) = iter.next() {
        obj.set("year", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("month", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("day", v)?;
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
        obj.set("years", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("months", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("weeks", v)?;
    }
    if let Some(v) = iter.next() {
        obj.set("days", v)?;
    }
    Ok(())
}
