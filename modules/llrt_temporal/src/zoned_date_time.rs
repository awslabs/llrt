// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned, ZonedDifference};
use llrt_utils::result::ResultExt;
use rquickjs::Object;
use rquickjs::{
    atom::PredefinedAtom, class::Trace, prelude::Opt, BigInt, Class, Ctx, Exception, JsLifetime,
    Result, Value,
};

use crate::utils::calendar::{split_calendar_annotation, Calendar};
use crate::utils::parse_temporal_calendar;
use crate::utils::time_zone::{parse_temporal_time_zone, timezone_rules};
use crate::utils::zoned::ZonedExt;
use crate::utils::zoned_difference::ZonedDifferenceExt;
use crate::utils::zoned_round::ZonedRoundOption;

use super::duration::Duration;
use super::extract_bigint_or_number;
use super::instant::Instant;
use super::plain_date::PlainDate;
use super::plain_date_time::PlainDateTime;
use super::plain_time::PlainTime;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct ZonedDateTime {
    #[qjs(skip_trace)]
    inner: Zoned,
    #[qjs(skip_trace)]
    ca: Calendar,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl ZonedDateTime {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'_>, nanos: Value<'_>, tz: Opt<String>, ca: Opt<String>) -> Result<Self> {
        Self::from_epoch_nanoseconds(&ctx, &nanos, tz.0.as_ref(), ca.0.as_ref())
    }

    #[qjs(static)]
    fn compare(ctx: Ctx<'_>, datetime1: Value<'_>, datetime2: Value<'_>) -> Result<i8> {
        let zdt1 = Self::from(ctx.clone(), datetime1)?;
        let zdt2 = Self::from(ctx.clone(), datetime2)?;
        Ok(match zdt1.inner.cmp(&zdt2.inner) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        })
    }

    #[qjs(static)]
    fn from(ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        if let Some(obj) = info.as_object() {
            if let Some(cls) = Class::<Self>::from_object(obj) {
                return Ok(cls.borrow().clone());
            }
            return Self::from_object(&ctx, obj);
        }

        let str = info
            .as_string()
            .and_then(|s| s.to_string().ok())
            .or_throw_type(&ctx, "Cannot convert value to string")?;

        Self::from_str(&ctx, &str)
    }

    fn add(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let duration = Duration::from_value(&ctx, &duration)?;
        let span = duration.into_inner();
        let zoned = self.inner.checked_add(span).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(Self { inner: zoned, ca })
    }

    fn equals(&self, ctx: Ctx<'_>, other: Self) -> Result<bool> {
        if self.inner.timestamp() != other.inner.timestamp() {
            return Ok(false);
        }

        let tz1 = self.inner.time_zone();
        let tz2 = other.inner.time_zone();

        let timezone_equal = match (tz1.iana_name(), tz2.iana_name()) {
            (Some(tz1), Some(tz2)) => timezone_rules(&ctx, tz1)? == timezone_rules(&ctx, tz2)?,
            (None, None) => tz1 == tz2,
            _ => false,
        };

        Ok(timezone_equal && self.ca == other.ca)
    }

    #[qjs(rename = "getTimeZoneTransition")]
    fn get_tz_transition<'js>(&self, ctx: Ctx<'js>, options: Value<'_>) -> Result<Value<'js>> {
        let tz = self.inner.time_zone();
        let tzt = match get_timezone_direction(&ctx, &options)? {
            Direction::Next => tz.following(self.inner.timestamp()).next(),
            Direction::Previous => tz.preceding(self.inner.timestamp()).next(),
        };
        let Some(tzt) = tzt else {
            return Ok(Value::new_null(ctx.clone()));
        };
        let zoned = tzt.timestamp().to_zoned(tz.clone());
        let ca = self.ca.clone();
        Self::new_instance(&ctx, zoned, ca)
    }

    fn round(&self, ctx: Ctx<'_>, options: Value<'_>) -> Result<Self> {
        let round = ZonedRoundOption::from_value(&ctx, &options)?;
        let round = round.into_inner();
        let zoned = self.inner.round(round).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(Self { inner: zoned, ca })
    }

    fn since(&self, ctx: Ctx<'_>, other: &Self, opt: Opt<Value<'_>>) -> Result<Duration> {
        let span = if let Some(ref opt) = opt.0 {
            let diff = ZonedDifference::from_value(&ctx, &other.inner, opt)?;
            self.inner.since(diff).or_throw_range(&ctx, "")?
        } else {
            self.inner.clone() - other.inner.clone()
        };
        Ok(Duration::new_object(span))
    }

    fn subtract(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let duration = Duration::from_value(&ctx, &duration)?;
        let span = duration.into_inner();
        let zoned = self.inner.checked_sub(span).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(Self { inner: zoned, ca })
    }

    fn to_instant(&self) -> Instant {
        Instant::from_zoned(&self.inner)
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToJSON)]
    fn to_json(&self) -> String {
        self.inner.to_string()
    }

    pub(crate) fn to_plain_date(&self) -> PlainDate {
        let date = self.inner.date();
        let ca = self.ca.clone();
        PlainDate::new_object(date, ca)
    }

    pub(crate) fn to_plain_date_time(&self) -> PlainDateTime {
        let dt = self.inner.datetime();
        let ca = self.ca.clone();
        PlainDateTime::new_object(dt, ca)
    }

    pub(crate) fn to_plain_time(&self) -> PlainTime {
        let time = self.inner.time();
        PlainTime::new_object(time)
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToString)]
    fn to_string(&self) -> String {
        self.inner.to_string()
    }

    fn until(&self, ctx: Ctx<'_>, other: &Self, opt: Opt<Value<'_>>) -> Result<Duration> {
        let span = if let Some(ref opt) = opt.0 {
            let diff = ZonedDifference::from_value(&ctx, &other.inner, opt)?;
            self.inner.until(diff).or_throw_range(&ctx, "")?
        } else {
            other.inner.clone() - self.inner.clone()
        };
        Ok(Duration::new_object(span))
    }

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert ZonedDateTime to primitive type",
        ))
    }

    fn with(&self, ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        let zoned = self.inner.zoned_with(&ctx, &info)?;
        let ca = self.ca.clone();
        Ok(Self { inner: zoned, ca })
    }

    fn with_time_zone(&self, ctx: Ctx<'_>, tz: String) -> Result<Self> {
        let zoned = self.inner.in_tz(&tz).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(Self { inner: zoned, ca })
    }

    #[qjs(get)]
    fn calendar_id(&self) -> &'static str {
        self.ca.id()
    }

    #[qjs(get)]
    fn day(&self) -> i8 {
        self.inner.day()
    }

    #[qjs(get)]
    fn day_of_year(&self) -> i16 {
        self.inner.day_of_year()
    }

    #[qjs(get)]
    fn days_in_month(&self) -> i8 {
        self.inner.days_in_month()
    }

    #[qjs(get)]
    fn days_in_year(&self) -> i16 {
        self.inner.days_in_year()
    }

    #[qjs(get)]
    fn epoch_milliseconds(&self) -> i64 {
        self.inner.timestamp().as_millisecond()
    }

    #[qjs(get)]
    fn epoch_nanoseconds<'js>(&self, ctx: Ctx<'js>) -> Result<BigInt<'js>> {
        let ns = self.inner.timestamp().as_nanosecond();
        let ns = ns.try_into().or_throw_range(&ctx, "")?;
        BigInt::from_i64(ctx, ns)
    }

    #[qjs(get)]
    fn hour(&self) -> i8 {
        self.inner.hour()
    }

    #[qjs(get)]
    fn in_leap_year(&self) -> bool {
        self.inner.in_leap_year()
    }

    #[qjs(get)]
    fn microsecond(&self) -> i16 {
        self.inner.microsecond()
    }

    #[qjs(get)]
    fn millisecond(&self) -> i16 {
        self.inner.millisecond()
    }

    #[qjs(get)]
    fn minute(&self) -> i8 {
        self.inner.minute()
    }

    #[qjs(get)]
    fn month(&self) -> i8 {
        self.inner.month()
    }

    #[qjs(get)]
    fn nanosecond(&self) -> i16 {
        self.inner.nanosecond()
    }

    #[qjs(get)]
    fn offset(&self) -> String {
        let offset = self.inner.offset().to_string();
        match offset.len() {
            3 => [&offset, ":00"].concat(),
            _ => offset.to_string(),
        }
    }

    #[qjs(get)]
    fn second(&self) -> i8 {
        self.inner.second()
    }

    #[qjs(get)]
    fn time_zone_id(&self) -> String {
        self.inner
            .time_zone()
            .iana_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Etc/Unknown".to_string())
    }

    #[qjs(get)]
    fn year(&self) -> i16 {
        self.inner.year()
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    fn to_string_tag() -> &'static str {
        "Temporal.ZonedDateTime"
    }
}

impl ZonedDateTime {
    fn from_epoch_nanoseconds(
        ctx: &Ctx<'_>,
        ns: &Value<'_>,
        tz: Option<&String>,
        ca: Option<&String>,
    ) -> Result<Self> {
        let nanos = extract_bigint_or_number(ctx, ns)?;
        Self::from_nanosecond(ctx, nanos, tz, ca)
    }

    fn from_nanosecond(
        ctx: &Ctx<'_>,
        ns: i128,
        tz: Option<&String>,
        ca: Option<&String>,
    ) -> Result<Self> {
        let ts = Timestamp::from_nanosecond(ns).or_throw_range(ctx, "")?;
        let tz = parse_temporal_time_zone(ctx, tz)?;
        let ca = parse_temporal_calendar(ctx, ca.cloned())?;
        Self::from_parts(&ts, tz, ca)
    }

    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        let zoned = Zoned::from_object(ctx, obj)?;
        let ca = Calendar::from_object(ctx, obj)?;
        Ok(Self { inner: zoned, ca })
    }

    pub(crate) fn from_parts(ts: &Timestamp, tz: TimeZone, ca: Calendar) -> Result<Self> {
        let zoned = ts.to_zoned(tz);
        Ok(Self { inner: zoned, ca })
    }

    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let (str, ca) = split_calendar_annotation(str);
        let ca = parse_temporal_calendar(ctx, ca)?;
        let zoned = Zoned::from_str(&str).or_throw_range(ctx, "")?;
        Ok(Self { inner: zoned, ca })
    }

    pub(crate) fn into_inner(self) -> Zoned {
        self.inner
    }

    fn new_instance<'js>(ctx: &Ctx<'js>, zoned: Zoned, ca: Calendar) -> Result<Value<'js>> {
        let zdt = Class::instance(ctx.clone(), Self { inner: zoned, ca })?;
        Ok(zdt.into_value())
    }

    pub(crate) fn new_object(zoned: Zoned, ca: Calendar) -> Self {
        Self { inner: zoned, ca }
    }
}

enum Direction {
    Next,
    Previous,
}

fn get_timezone_direction(ctx: &Ctx<'_>, val: &Value<'_>) -> Result<Direction> {
    fn matching(ctx: &Ctx, str: &str) -> Result<Direction> {
        match str {
            "next" => Ok(Direction::Next),
            "previous" => Ok(Direction::Previous),
            _ => Err(Exception::throw_type(ctx, "Invalid direction")),
        }
    }

    if let Some(str) = val.as_string() {
        if let Ok(v) = str.to_string() {
            return matching(ctx, v.as_str());
        }
    } else if let Some(obj) = val.as_object() {
        if let Ok(v) = obj.get::<_, String>("direction") {
            return matching(ctx, v.as_str());
        }
    }
    Err(Exception::throw_type(ctx, "Invalid direction"))
}
