// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod duration;
mod instant;
mod now;
mod plain_date;
mod plain_date_time;
mod plain_time;
mod utils;
mod zoned_date_time;

use std::str::FromStr;

use jiff::civil::Time;
use llrt_utils::result::ResultExt;
use rquickjs::{Class, Ctx, Exception, Object, Result, Value};

use crate::duration::Duration;
use crate::instant::Instant;
use crate::plain_date::PlainDate;
use crate::plain_date_time::PlainDateTime;
use crate::plain_time::PlainTime;
use crate::zoned_date_time::ZonedDateTime;

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let temporal = Object::new(ctx.clone())?;

    Class::<Duration>::define(&temporal)?;
    Class::<Instant>::define(&temporal)?;
    Class::<PlainDate>::define(&temporal)?;
    Class::<PlainDateTime>::define(&temporal)?;
    Class::<PlainTime>::define(&temporal)?;
    Class::<ZonedDateTime>::define(&temporal)?;
    temporal.set("Now", now::define_object(ctx)?)?;

    ctx.globals().set("Temporal", temporal)?;
    Ok(())
}

pub(crate) fn extract_bigint_or_number(ctx: &Ctx<'_>, value: &Value<'_>) -> Result<i128> {
    if let Some(num) = value.as_number() {
        if !num.is_finite() {
            return Err(Exception::throw_message(ctx, "Invalid value"));
        }
        Ok(num as i128)
    } else if let Some(bigint) = value.as_big_int() {
        match bigint.clone().to_i64() {
            Ok(v) => Ok(v as i128),
            Err(_) => Err(Exception::throw_message(ctx, "BigInt value out of range")),
        }
    } else {
        Err(Exception::throw_message(ctx, "Expected number or BigInt"))
    }
}

pub(crate) fn extract_time(ctx: &Ctx<'_>, val: &Option<Value<'_>>) -> Result<Time> {
    let mut tm = Time::MIN;

    let Some(val) = &val else {
        return Ok(tm);
    };

    if let Some(str) = val.as_string() {
        if let Ok(v) = str.to_string() {
            tm = Time::from_str(&v).or_throw_range(ctx, "")?;
        }
    } else if let Some(obj) = val.as_object() {
        if let Some(v) = Class::<PlainTime>::from_object(obj) {
            let pt = v.borrow().clone();
            tm = pt.into_inner();
        }
    }
    Ok(tm)
}

pub(crate) fn extract_time_and_timezone(ctx: &Ctx<'_>, val: &Value<'_>) -> Result<(Time, String)> {
    let (mut tm, mut tz) = (Time::MIN, None);

    if let Some(str) = val.as_string() {
        if let Ok(v) = str.to_string() {
            tz = Some(v);
        }
    } else if let Some(obj) = val.as_object() {
        if let Ok(v) = obj.get::<_, String>("plainTime") {
            tm = Time::from_str(&v).or_throw_range(ctx, "")?;
        } else if let Ok(v) = obj.get::<_, PlainTime>("plainTime") {
            tm = v.into_inner();
        }

        if let Ok(v) = obj.get::<_, String>("timeZone") {
            tz = Some(v);
        } else if let Ok(v) = obj.get::<_, ZonedDateTime>("timeZone") {
            tz = Some(v.into_inner().time_zone().iana_name().unwrap().into());
        }
    }

    let tz = tz.or_throw_type(
        ctx,
        "timeZone is not a string or a Temporal.ZonedDateTime instance",
    )?;
    Ok((tm, tz))
}
