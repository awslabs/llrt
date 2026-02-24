// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{Timestamp, Zoned};
use llrt_utils::result::ResultExt;
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
    let tz = obj.get::<_, String>("timeZone").map_err_type(ctx)?;
    ts.in_tz(&tz).map_err_range(ctx)
}

fn into_zoned(ctx: &Ctx<'_>, zoned: &Zoned, obj: &Object<'_>) -> Result<Zoned> {
    let mut zoned = zoned.with();

    macro_rules! apply {
        ($key:literal, $ty:ty, $method:ident) => {
            if let Ok(v) = obj.get::<_, $ty>($key) {
                zoned = zoned.$method(v);
            }
        };
    }

    apply!("day", i8, day);
    apply!("hour", i8, hour);
    apply!("microsecond", i16, microsecond);
    apply!("millisecond", i16, millisecond);
    apply!("minute", i8, minute);
    apply!("month", i8, month);
    apply!("nanosecond", i16, nanosecond);
    apply!("second", i8, second);
    apply!("year", i16, year);
    zoned.build().map_err_range(ctx)
}
