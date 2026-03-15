// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{Timestamp, Zoned};
use rquickjs::{
    atom::PredefinedAtom,
    prelude::{Func, Opt},
    Ctx, Object, Result,
};

use crate::instant::Instant;
use crate::plain_date::PlainDate;
use crate::plain_date_time::PlainDateTime;
use crate::plain_time::PlainTime;
use crate::zoned_date_time::ZonedDateTime;

pub(crate) fn define_object<'a>(ctx: &Ctx<'a>) -> Result<Object<'a>> {
    let obj = Object::new(ctx.clone())?;
    obj.set("instant", Func::from(Instant::now))?;
    obj.set("plainDateISO", Func::from(plain_date_iso))?;
    obj.set("plainDateTimeISO", Func::from(plain_datetime_iso))?;
    obj.set("plainTimeISO", Func::from(plain_time_iso))?;
    obj.set("zonedDateTimeISO", Func::from(zoned_datetime_iso))?;
    obj.set(PredefinedAtom::SymbolToStringTag, "Temporal.Now")?;
    Ok(obj)
}

fn plain_date_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<PlainDate> {
    let (ts, tz) = parts_of_zoned_now(timezone);
    PlainDate::from_ts_tz(&ctx, &ts, &tz)
}

fn plain_datetime_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<PlainDateTime> {
    let (ts, tz) = parts_of_zoned_now(timezone);
    PlainDateTime::from_ts_tz(&ctx, &ts, &tz)
}

fn plain_time_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<PlainTime> {
    let (ts, tz) = parts_of_zoned_now(timezone);
    PlainTime::from_ts_tz(&ctx, &ts, &tz)
}

fn zoned_datetime_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<ZonedDateTime> {
    let (ts, tz) = parts_of_zoned_now(timezone);
    ZonedDateTime::from_ts_tz(&ctx, &ts, &tz)
}

fn parts_of_zoned_now(timezone: Opt<String>) -> (Timestamp, String) {
    let zoned = Zoned::now();
    let ts = zoned.timestamp();
    let tz = zoned.time_zone().iana_name().unwrap_or("UTC");
    let tz = timezone.0.unwrap_or(tz.to_string());
    (ts, tz)
}
