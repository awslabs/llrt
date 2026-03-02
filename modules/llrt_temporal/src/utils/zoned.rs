// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{Timestamp, Zoned};
use llrt_utils::result::{By, ResultExt};
use rquickjs::{Ctx, Exception, Object, Result, Value};

use super::timestamp::TimestampExt;

pub trait ZonedExt {
    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Zoned>;
    fn zoned_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Zoned>;
}

impl ZonedExt for Zoned {
    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Self> {
        from_obj(ctx, object)
    }

    fn zoned_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        if let Some(obj) = value.as_object() {
            return into_zoned(ctx, self, obj);
        }
        Err(Exception::throw_type(ctx, "Expected object"))
    }
}

fn from_obj(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Zoned> {
    let ts = Timestamp::from_object(ctx, obj)?;
    let tz = obj
        .get::<_, String>("timeZone")
        .or_throw_by(ctx, By::Type)?;
    ts.in_tz(&tz).or_throw_by(ctx, By::Range)
}

fn into_zoned(ctx: &Ctx<'_>, zoned: &Zoned, obj: &Object<'_>) -> Result<Zoned> {
    let mut zoned = zoned.with();
    if let Ok(v) = obj.get::<_, i8>("day") {
        zoned = zoned.day(v);
    }
    if let Ok(v) = obj.get::<_, i8>("hour") {
        zoned = zoned.hour(v);
    }
    if let Ok(v) = obj.get::<_, i16>("microsecond") {
        zoned = zoned.microsecond(v);
    }
    if let Ok(v) = obj.get::<_, i16>("millisecond") {
        zoned = zoned.millisecond(v);
    }
    if let Ok(v) = obj.get::<_, i8>("minute") {
        zoned = zoned.minute(v);
    }
    if let Ok(v) = obj.get::<_, i8>("month") {
        zoned = zoned.month(v);
    }
    if let Ok(v) = obj.get::<_, i16>("nanosecond") {
        zoned = zoned.nanosecond(v);
    }
    if let Ok(v) = obj.get::<_, i8>("second") {
        zoned = zoned.second(v);
    }
    if let Ok(v) = obj.get::<_, i16>("year") {
        zoned = zoned.year(v);
    }
    zoned.build().or_throw_by(ctx, By::Range)
}
