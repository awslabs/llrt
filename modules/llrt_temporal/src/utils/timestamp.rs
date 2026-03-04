// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{civil::Date, Timestamp};
use llrt_utils::result::{By, ResultExt};
use rquickjs::{Ctx, Object, Result};

pub trait TimestampExt {
    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Timestamp>;
}

impl TimestampExt for Timestamp {
    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Self> {
        from_obj(ctx, object)
    }
}

fn from_obj(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Timestamp> {
    let year = obj.get::<_, i16>("year").or_throw_by(ctx, By::Range)?;
    let month = obj.get::<_, i8>("month").or_throw_by(ctx, By::Range)?;
    let day = obj.get::<_, i8>("day").or_throw_by(ctx, By::Range)?;

    let hour = obj.get::<_, i8>("hour").unwrap_or_default();
    let minute = obj.get::<_, i8>("minute").unwrap_or_default();
    let second = obj.get::<_, i8>("second").unwrap_or_default();

    let millis = obj.get::<_, i32>("millisecond").unwrap_or_default();
    let micros = obj.get::<_, i32>("microsecond").unwrap_or_default();
    let nanos = obj.get::<_, i32>("nanosecond").unwrap_or_default();
    let subsec_ns = nanos + micros * 1_000 + millis * 1_000_000;

    let date = Date::new(year, month, day).or_throw_by(ctx, By::Range)?;
    let dt = date.at(hour, minute, second, subsec_ns);

    let ts = dt.in_tz("UTC").or_throw_by(ctx, By::Range)?;
    Ok(ts.timestamp())
}
