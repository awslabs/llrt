// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::{
    civil::{Date, DateTime},
    Span, Timestamp,
};
use llrt_utils::result::ResultExt;
use rquickjs::{
    atom::PredefinedAtom,
    class::Trace,
    prelude::{Opt, Rest},
    Class, Ctx, Exception, JsLifetime, Object, Result, Value,
};

use crate::duration::Duration;
use crate::plain_date_time::PlainDateTime;
use crate::utils::date::{fill_from_iter, DateExt};
use crate::utils::span::SpanExt;
use crate::zoned_date_time::ZonedDateTime;

use super::{extract_time, extract_time_and_timezone};

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct PlainDate {
    #[qjs(skip_trace)]
    inner: Date,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl PlainDate {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<Self> {
        let obj = Self::fill_object(&ctx, &args)?;
        Self::from_object(&ctx, &obj)
    }

    #[qjs(static)]
    fn compare(date1: Self, date2: Self) -> i8 {
        match date1.inner.cmp(&date2.inner) {
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
            if let Some(cls) = Class::<PlainDateTime>::from_object(obj) {
                let pdt = cls.borrow().clone();
                return Ok(pdt.to_plain_date());
            }
            if let Some(cls) = Class::<ZonedDateTime>::from_object(obj) {
                let zdt = cls.borrow().clone();
                return Ok(zdt.to_plain_date());
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
        let span = Span::from_value(&ctx, &duration)?;
        let zoned = self.inner.checked_add(span).or_throw_range(&ctx, "")?;
        Ok(Self { inner: zoned })
    }

    fn equals(&self, other: Self) -> bool {
        self.inner == other.inner
    }

    fn since(&self, other: Self) -> Duration {
        Duration::new_object(self.inner - other.inner)
    }

    fn subtract(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let span = Span::from_value(&ctx, &duration)?;
        let date = self.inner.checked_sub(span).or_throw_range(&ctx, "")?;
        Ok(Self { inner: date })
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToJSON)]
    fn to_json(&self) -> String {
        self.inner.to_string()
    }

    fn to_plain_date_time(&self, ctx: Ctx<'_>, value: Opt<Value<'_>>) -> Result<PlainDateTime> {
        let time = extract_time(&ctx, &value.0)?;
        let dt = DateTime::from_parts(self.inner, time);
        Ok(PlainDateTime::new_object(dt))
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToString)]
    fn to_string(&self) -> String {
        self.inner.to_string()
    }

    fn to_zoned_date_time(&self, ctx: Ctx<'_>, value: Value<'_>) -> Result<ZonedDateTime> {
        let (time, tz) = extract_time_and_timezone(&ctx, &value)?;
        let dt = DateTime::from_parts(self.inner, time);
        let zoned = dt.in_tz(&tz).or_throw_range(&ctx, "")?;
        Ok(ZonedDateTime::new_object(zoned))
    }

    fn until(&self, other: Self) -> Duration {
        Duration::new_object(other.inner - self.inner)
    }

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert PlainDate to primitive type",
        ))
    }

    fn with(&self, ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        let date = self.inner.date_with(&ctx, &info)?;
        Ok(Self { inner: date })
    }

    #[qjs(get)]
    fn calendar_id() -> &'static str {
        "iso8601"
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
    fn in_leap_year(&self) -> bool {
        self.inner.in_leap_year()
    }

    #[qjs(get)]
    fn month(&self) -> i8 {
        self.inner.month()
    }

    #[qjs(get)]
    fn year(&self) -> i16 {
        self.inner.year()
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    fn to_string_tag(&self) -> &'static str {
        "Temporal.PlainDate"
    }
}

impl PlainDate {
    fn fill_object<'js>(ctx: &Ctx<'js>, args: &Rest<Value<'js>>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        let mut iter = args.0.iter().cloned();
        fill_from_iter(&obj, &mut iter, true)?;
        Ok(obj)
    }

    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        let date = Date::from_object(ctx, obj)?;
        Ok(Self { inner: date })
    }

    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let date = Date::from_str(str).or_throw_range(ctx, "")?;
        Ok(Self { inner: date })
    }

    pub(crate) fn from_ts_tz(ctx: &Ctx<'_>, ts: &Timestamp, tz: &str) -> Result<Self> {
        let zoned = ts.in_tz(tz).or_throw_range(ctx, "")?;
        let zdt = ZonedDateTime::new_object(zoned);
        Ok(zdt.to_plain_date())
    }

    pub(crate) fn new_object(date: Date) -> Self {
        Self { inner: date }
    }
}
