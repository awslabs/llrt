// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{tz::TimeZone, Timestamp, Zoned};
use rquickjs::{
    atom::PredefinedAtom,
    prelude::{Func, Opt},
    Ctx, Object, Result,
};

use crate::utils::parse_temporal_calendar;
use crate::utils::time_zone::parse_temporal_time_zone;

use super::instant::Instant;
use super::plain_date::PlainDate;
use super::plain_date_time::PlainDateTime;
use super::plain_time::PlainTime;
use super::zoned_date_time::ZonedDateTime;

pub(crate) fn define_object<'a>(ctx: &Ctx<'a>) -> Result<Object<'a>> {
    let obj = Object::new(ctx.clone())?;
    obj.set("instant", Func::from(Instant::now))?;
    obj.set("plainDateISO", Func::from(plain_date_iso))?;
    obj.set("plainDateTimeISO", Func::from(plain_datetime_iso))?;
    obj.set("plainTimeISO", Func::from(plain_time_iso))?;
    obj.set("timeZoneId", Func::from(time_zone_id))?;
    obj.set("zonedDateTimeISO", Func::from(zoned_datetime_iso))?;
    obj.set(PredefinedAtom::SymbolToStringTag, "Temporal.Now")?;
    Ok(obj)
}

fn plain_date_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<PlainDate> {
    let (ts, tz) = parts_of_zoned_now(&ctx, timezone.0.as_ref())?;
    let ca = parse_temporal_calendar(&ctx, None)?;
    PlainDate::from_parts(&ts, tz, ca)
}

fn plain_datetime_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<PlainDateTime> {
    let (ts, tz) = parts_of_zoned_now(&ctx, timezone.0.as_ref())?;
    let ca = parse_temporal_calendar(&ctx, None)?;
    PlainDateTime::from_parts(&ts, tz, ca)
}

fn plain_time_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<PlainTime> {
    let (ts, tz) = parts_of_zoned_now(&ctx, timezone.0.as_ref())?;
    let ca = parse_temporal_calendar(&ctx, None)?;
    PlainTime::from_parts(&ts, tz, ca)
}

fn time_zone_id() -> String {
    let tz: TimeZone = TimeZone::system();
    tz.iana_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "UTC".to_string())
}

fn zoned_datetime_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<ZonedDateTime> {
    let (ts, tz) = parts_of_zoned_now(&ctx, timezone.0.as_ref())?;
    let ca = parse_temporal_calendar(&ctx, None)?;
    ZonedDateTime::from_parts(&ts, tz, ca)
}

fn parts_of_zoned_now(ctx: &Ctx<'_>, timezone: Option<&String>) -> Result<(Timestamp, TimeZone)> {
    let zoned = Zoned::now();
    let ts = zoned.timestamp();
    let tz = parse_temporal_time_zone(ctx, timezone)?;
    Ok((ts, tz))
}
