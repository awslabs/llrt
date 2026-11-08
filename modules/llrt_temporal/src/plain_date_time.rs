// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::{civil::DateTime, tz::TimeZone, Timestamp};
use llrt_utils::result::ResultExt;
use rquickjs::{
    atom::PredefinedAtom,
    class::Trace,
    prelude::{Opt, Rest},
    Class, Ctx, Exception, JsLifetime, Object, Result, Value,
};

use crate::utils::calendar::{split_calendar_annotation, Calendar};
use crate::utils::date::fill_from_iter as fill_date_from_iter;
use crate::utils::date_time::DateTimeExt;
use crate::utils::date_time_round::DateTimeRoundOption;
use crate::utils::parse_temporal_calendar;
use crate::utils::time::fill_from_iter as fill_time_from_iter;

use super::duration::Duration;
use super::extract_time;
use super::plain_date::PlainDate;
use super::plain_time::PlainTime;
use super::zoned_date_time::ZonedDateTime;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct PlainDateTime {
    #[qjs(skip_trace)]
    inner: DateTime,
    #[qjs(skip_trace)]
    ca: Calendar,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl PlainDateTime {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<Self> {
        let obj = Self::fill_object(&ctx, &args)?;
        Self::from_object(&ctx, &obj)
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
        let dt = self.inner.checked_add(span).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(Self { inner: dt, ca })
    }

    fn equals(&self, other: Self) -> bool {
        self.inner == other.inner
    }

    fn round(&self, ctx: Ctx<'_>, options: Value<'_>) -> Result<Self> {
        let round = DateTimeRoundOption::from_value(&ctx, &options)?;
        let round = round.into_inner();
        let dt = self.inner.round(round).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(Self { inner: dt, ca })
    }

    fn since(&self, other: Self) -> Duration {
        Duration::new_object(self.inner - other.inner)
    }

    fn subtract(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let duration = Duration::from_value(&ctx, &duration)?;
        let span = duration.into_inner();
        let dt = self.inner.checked_sub(span).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(Self { inner: dt, ca })
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

    pub(crate) fn to_plain_time(&self) -> PlainTime {
        let time = self.inner.time();
        PlainTime::new_object(time)
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToString)]
    fn to_string(&self) -> String {
        self.inner.to_string()
    }

    fn to_zoned_date_time(&self, ctx: Ctx<'_>, tz: String) -> Result<ZonedDateTime> {
        let zoned = self.inner.in_tz(&tz).or_throw_range(&ctx, "")?;
        let ca = self.ca.clone();
        Ok(ZonedDateTime::new_object(zoned, ca))
    }

    fn until(&self, other: Self) -> Duration {
        Duration::new_object(other.inner - self.inner)
    }

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert PlainDateTime to primitive type",
        ))
    }

    fn with(&self, ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        let dt = self.inner.date_time_with(&ctx, &info)?;
        let ca = self.ca.clone();
        Ok(Self { inner: dt, ca })
    }

    fn with_plain_time(&self, ctx: Ctx<'_>, val: Opt<Value<'_>>) -> Result<Self> {
        let time = extract_time(&ctx, &val.0)?;
        let dt = DateTime::from_parts(self.inner.date(), time);
        let ca = self.ca.clone();
        Ok(Self { inner: dt, ca })
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
    fn second(&self) -> i8 {
        self.inner.second()
    }

    #[qjs(get)]
    fn year(&self) -> i16 {
        self.inner.year()
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    fn to_string_tag() -> &'static str {
        "Temporal.PlainDateTime"
    }
}

impl PlainDateTime {
    fn fill_object<'js>(ctx: &Ctx<'js>, args: &Rest<Value<'js>>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        let mut iter = args.0.iter().cloned();
        fill_date_from_iter(&obj, &mut iter, false)?;
        fill_time_from_iter(&obj, &mut iter, true)?;
        Ok(obj)
    }

    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        let dt = DateTime::from_object(ctx, obj)?;
        let ca = Calendar::from_object(ctx, obj)?;
        Ok(Self { inner: dt, ca })
    }

    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let (str, ca) = split_calendar_annotation(str);
        let ca = parse_temporal_calendar(ctx, ca)?;
        let dt = DateTime::from_str(&str).or_throw_range(ctx, "")?;
        Ok(Self { inner: dt, ca })
    }

    pub(crate) fn from_parts(ts: &Timestamp, tz: TimeZone, ca: Calendar) -> Result<Self> {
        let zoned = ts.to_zoned(tz);
        let zdt = ZonedDateTime::new_object(zoned, ca);
        Ok(zdt.to_plain_date_time())
    }

    pub(crate) fn into_inner(self) -> DateTime {
        self.inner
    }

    pub(crate) fn new_object(dt: DateTime, ca: Calendar) -> Self {
        Self { inner: dt, ca }
    }
}
