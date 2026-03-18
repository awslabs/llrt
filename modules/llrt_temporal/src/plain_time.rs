// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::{civil::Time, Timestamp};
use llrt_utils::result::ResultExt;
use rquickjs::{
    atom::PredefinedAtom, class::Trace, prelude::Rest, Class, Ctx, Exception, JsLifetime, Object,
    Result, Value,
};

use crate::duration::Duration;
use crate::plain_date_time::PlainDateTime;
use crate::utils::round::time::TimeRoundOption;
use crate::utils::time::{fill_from_iter, TimeExt};
use crate::zoned_date_time::ZonedDateTime;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct PlainTime {
    #[qjs(skip_trace)]
    inner: Time,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl PlainTime {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<Self> {
        let obj = Self::fill_object(&ctx, &args)?;
        Self::from_object(&ctx, &obj)
    }

    #[qjs(static)]
    fn compare(time1: Self, time2: Self) -> i8 {
        match time1.inner.cmp(&time2.inner) {
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
                return Ok(pdt.to_plain_time());
            }
            if let Some(cls) = Class::<ZonedDateTime>::from_object(obj) {
                let zdt = cls.borrow().clone();
                return Ok(zdt.to_plain_time());
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
        let time = self.inner.checked_add(span).or_throw_range(&ctx, "")?;
        Ok(Self { inner: time })
    }

    fn equals(&self, other: Self) -> bool {
        self.inner == other.inner
    }

    fn round(&self, ctx: Ctx<'_>, options: Value<'_>) -> Result<Self> {
        let round = TimeRoundOption::from_value(&ctx, &options)?;
        let round = round.into_inner();
        let time = self.inner.round(round).or_throw_range(&ctx, "")?;
        Ok(Self { inner: time })
    }

    fn since(&self, other: Self) -> Duration {
        Duration::new_object(self.inner - other.inner)
    }

    fn subtract(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let duration = Duration::from_value(&ctx, &duration)?;
        let span = duration.into_inner();
        let time = self.inner.checked_sub(span).or_throw_range(&ctx, "")?;
        Ok(Self { inner: time })
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToJSON)]
    fn to_json(&self) -> String {
        self.inner.to_string()
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToString)]
    fn to_string(&self) -> String {
        self.inner.to_string()
    }

    fn until(&self, other: Self) -> Duration {
        Duration::new_object(other.inner - self.inner)
    }

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert PlainTime to primitive type",
        ))
    }

    fn with(&self, ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        let time = self.inner.time_with(&ctx, &info)?;
        Ok(Self { inner: time })
    }

    #[qjs(get)]
    fn hour(&self) -> i8 {
        self.inner.hour()
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
    fn nanosecond(&self) -> i16 {
        self.inner.nanosecond()
    }

    #[qjs(get)]
    fn second(&self) -> i8 {
        self.inner.second()
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    fn to_string_tag(&self) -> &'static str {
        "Temporal.PlainTime"
    }
}

impl PlainTime {
    fn fill_object<'js>(ctx: &Ctx<'js>, args: &Rest<Value<'js>>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        let mut iter = args.0.iter().cloned();
        fill_from_iter(&obj, &mut iter, true)?;
        Ok(obj)
    }

    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        let time = Time::from_object(ctx, obj)?;
        Ok(Self { inner: time })
    }

    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let time = Time::from_str(str).or_throw_range(ctx, "")?;
        Ok(Self { inner: time })
    }

    pub(crate) fn from_ts_tz(ctx: &Ctx<'_>, ts: &Timestamp, tz: &str) -> Result<Self> {
        let zoned = ts.in_tz(tz).or_throw_range(ctx, "")?;
        let zdt = ZonedDateTime::new_object(zoned);
        Ok(zdt.to_plain_time())
    }

    pub(crate) fn into_inner(self) -> Time {
        self.inner
    }

    pub(crate) fn new_object(time: Time) -> Self {
        Self { inner: time }
    }
}
