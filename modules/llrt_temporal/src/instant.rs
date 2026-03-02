// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::{Span, Timestamp, Zoned};
use llrt_utils::result::{By, ResultExt};
use rquickjs::Class;
use rquickjs::{
    atom::PredefinedAtom, class::Trace, Ctx, Exception, JsLifetime, Object, Result, Value,
};

use crate::duration::Duration;
use crate::utils::{span::SpanExt, timestamp::TimestampExt};
use crate::zoned_date_time::ZonedDateTime;

use super::extract_bigint_or_number;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub struct Instant {
    #[qjs(skip_trace)]
    inner: Timestamp,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Instant {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'_>, nanos: Value<'_>) -> Result<Self> {
        Self::from_epoch_nanoseconds(ctx, nanos)
    }

    #[qjs(static)]
    fn compare(instant1: Self, instant2: Self) -> i8 {
        match instant1.inner.cmp(&instant2.inner) {
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
        if let Some(str) = info.as_string().and_then(|s| s.to_string().ok()) {
            return Self::from_str(&ctx, &str);
        }
        Err(Exception::throw_type(
            &ctx,
            "Cannot convert value to Temporal.Instant",
        ))
    }

    #[qjs(static)]
    fn from_epoch_milliseconds(ctx: Ctx<'_>, ms: f64) -> Result<Self> {
        let nanos = (ms * 1_000_000.0) as i128;
        Self::from_nanosecond(&ctx, nanos)
    }

    #[qjs(static)]
    fn from_epoch_nanoseconds(ctx: Ctx<'_>, nanos: Value<'_>) -> Result<Self> {
        let nanos = extract_bigint_or_number(&ctx, &nanos)?;
        Self::from_nanosecond(&ctx, nanos)
    }

    fn add(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let span = Span::from_value(&ctx, &duration)?;
        let ts = self.inner.checked_add(span).or_throw_by(&ctx, By::Range)?;
        Ok(Self { inner: ts })
    }

    fn equals(&self, other: Self) -> bool {
        self.inner == other.inner
    }

    fn since(&self, other: Self) -> Duration {
        Duration::from_span(self.inner - other.inner)
    }

    fn subtract(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let span = Span::from_value(&ctx, &duration)?;
        let ts = self.inner.checked_sub(span).or_throw_by(&ctx, By::Range)?;
        Ok(Self { inner: ts })
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

    #[qjs(rename = "toZonedDateTimeISO")]
    fn to_zoned_dt_iso(&self, ctx: Ctx<'_>, timezone: Option<String>) -> Result<ZonedDateTime> {
        ZonedDateTime::from_timestamp(&ctx, &self.inner, &timezone)
    }

    fn until(&self, other: Self) -> Duration {
        Duration::from_span(other.inner - self.inner)
    }

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert Instant to primitive type",
        ))
    }

    #[qjs(get)]
    fn epoch_milliseconds(&self) -> i64 {
        self.inner.as_millisecond()
    }

    #[qjs(get)]
    fn epoch_nanoseconds(&self) -> f64 {
        self.inner.as_nanosecond() as f64
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    fn to_string_tag(&self) -> &'static str {
        "Temporal.Instant"
    }
}

impl Instant {
    pub fn from_zoned(zoned: &Zoned) -> Self {
        let ts = zoned.timestamp();
        Self { inner: ts }
    }

    fn from_nanosecond(ctx: &Ctx<'_>, ns: i128) -> Result<Self> {
        let ts = Timestamp::from_nanosecond(ns).or_throw_by(ctx, By::Range)?;
        Ok(Self { inner: ts })
    }

    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let ts = Timestamp::from_str(str).or_throw_by(ctx, By::Range)?;
        Ok(Self { inner: ts })
    }

    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Self> {
        let ts = Timestamp::from_object(ctx, object)?;
        Ok(Self { inner: ts })
    }

    pub fn now() -> Self {
        let ts = Timestamp::now();
        Self { inner: ts }
    }

    pub fn into_inner(&self) -> Timestamp {
        self.inner
    }
}
