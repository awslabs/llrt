// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{civil::DateTime, ZonedDateTime as Zoned};
use llrt_utils::{object::ObjectExt, result::ResultExt};
use rquickjs::{Ctx, Exception, Object, Result, Value};

use super::date_time::DateTimeExt;
use super::time_zone::{parse_temporal_time_zone, parse_two_digits};

pub(crate) trait ZonedExt {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Zoned>;
    fn zoned_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Zoned>;
}

impl ZonedExt for Zoned {
    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        from_obj(ctx, obj)
    }

    fn zoned_with(&self, ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        let obj = value
            .as_object()
            .or_throw_type(ctx, "Cannot convert value to object")?;

        into_zoned(ctx, self, obj)
    }
}

fn from_obj(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Zoned> {
    let dt = DateTime::from_object(ctx, obj)?;
    let tz = obj.get_optional::<_, String>("timeZone")?;
    let tz = parse_temporal_time_zone(ctx, tz.as_ref())?;
    let zoned = dt.to_zoned_datetime(tz).or_throw_range(ctx, "")?;

    if let Some(offset) = obj.get_optional::<_, String>("offset")? {
        let offset_seconds = parse_property_offset_seconds(&offset)
            .or_throw_range(ctx, "Invalid or rounded offset in property bag")?;

        if zoned.offset().seconds() != offset_seconds {
            return Err(Exception::throw_range(
                ctx,
                "Offset does not match time zone",
            ));
        }
    }
    Ok(zoned)
}

fn parse_property_offset_seconds(offset: &str) -> Option<i32> {
    let bytes = offset.as_bytes();

    if bytes.len() == 6 {
        return None;
    }

    if bytes.len() != 9 || bytes[3] != b':' || bytes[6] != b':' {
        return None;
    }

    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours = parse_two_digits(&bytes[1..3])?;
    let minutes = parse_two_digits(&bytes[4..6])?;
    let seconds = parse_two_digits(&bytes[7..9])?;

    if hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }

    Some(sign * ((hours * 60 + minutes) * 60 + seconds))
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
    zoned.build().or_throw_range(ctx, "")
}
