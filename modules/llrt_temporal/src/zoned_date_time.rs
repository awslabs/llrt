// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::{Timestamp, Zoned};
use llrt_utils::result::ResultExt;
use rquickjs::Object;
use rquickjs::{
    atom::PredefinedAtom, class::Trace, prelude::Opt, BigInt, Class, Ctx, Exception, JsLifetime,
    Result, Value,
};

use crate::duration::Duration;
use crate::instant::Instant;
use crate::plain_date::PlainDate;
use crate::plain_date_time::PlainDateTime;
use crate::plain_time::PlainTime;
use crate::utils::zoned::round::ZonedRoundOption;
use crate::utils::zoned::ZonedExt;

use super::extract_bigint_or_number;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct ZonedDateTime {
    #[qjs(skip_trace)]
    inner: Zoned,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl ZonedDateTime {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'_>, nanos: Value<'_>, tz: Opt<String>) -> Result<Self> {
        Self::from_epoch_nanoseconds(&ctx, &nanos, &tz)
    }

    #[qjs(static)]
    fn compare(datetime1: Self, datetime2: Self) -> i8 {
        match datetime1.inner.cmp(&datetime2.inner) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
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
        Ok(Self { inner: zoned })
    }

    fn equals(&self, other: Self) -> bool {
        self.inner == other.inner
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
        Self::new_instance(&ctx, zoned)
    }

    fn round(&self, ctx: Ctx<'_>, options: Value<'_>) -> Result<Self> {
        let round = ZonedRoundOption::from_value(&ctx, &options)?;
        let round = round.into_inner();
        let zoned = self.inner.round(round).or_throw_range(&ctx, "")?;
        Ok(Self { inner: zoned })
    }

    fn since(&self, other: Self) -> Duration {
        Duration::new_object(self.inner.clone() - other.inner)
    }

    fn subtract(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let duration = Duration::from_value(&ctx, &duration)?;
        let span = duration.into_inner();
        let zoned = self.inner.checked_sub(span).or_throw_range(&ctx, "")?;
        Ok(Self { inner: zoned })
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
        PlainDate::new_object(date)
    }

    pub(crate) fn to_plain_date_time(&self) -> PlainDateTime {
        let dt = self.inner.datetime();
        PlainDateTime::new_object(dt)
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

    fn until(&self, other: Self) -> Duration {
        Duration::new_object(other.inner - self.inner.clone())
    }

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert ZonedDateTime to primitive type",
        ))
    }

    fn with(&self, ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        let zoned = self.inner.zoned_with(&ctx, &info)?;
        Ok(Self { inner: zoned })
    }

    fn with_time_zone(&self, ctx: Ctx<'_>, tz: String) -> Result<Self> {
        let zoned = self.inner.in_tz(&tz).or_throw_range(&ctx, "")?;
        Ok(Self { inner: zoned })
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
            _ => offset,
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

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    fn to_string_tag(&self) -> &'static str {
        "Temporal.ZonedDateTime"
    }
}

impl ZonedDateTime {
    fn from_epoch_nanoseconds(ctx: &Ctx<'_>, ns: &Value<'_>, tz: &Option<String>) -> Result<Self> {
        let nanos = extract_bigint_or_number(ctx, ns)?;
        Self::from_nanosecond(ctx, nanos, tz)
    }

    fn from_nanosecond(ctx: &Ctx<'_>, ns: i128, tz: &Option<String>) -> Result<Self> {
        let ts = Timestamp::from_nanosecond(ns).or_throw_range(ctx, "")?;
        let tz = tz.as_deref().unwrap_or("UTC");
        Self::from_ts_tz(ctx, &ts, tz)
    }

    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        let zoned = Zoned::from_object(ctx, obj)?;
        Ok(Self { inner: zoned })
    }

    pub(crate) fn from_ts_tz(ctx: &Ctx<'_>, ts: &Timestamp, tz: &str) -> Result<Self> {
        let zoned = ts.in_tz(tz).or_throw_range(ctx, "")?;
        Ok(Self { inner: zoned })
    }

    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let zoned = Zoned::from_str(str).or_throw_range(ctx, "")?;
        Ok(Self { inner: zoned })
    }

    pub(crate) fn into_inner(self) -> Zoned {
        self.inner
    }

    fn new_instance<'js>(ctx: &Ctx<'js>, zoned: Zoned) -> Result<Value<'js>> {
        let zdt = Class::instance(ctx.clone(), Self { inner: zoned })?;
        Ok(zdt.into_value())
    }

    pub(crate) fn new_object(zoned: Zoned) -> Self {
        Self { inner: zoned }
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
