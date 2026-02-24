// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::{Span, Timestamp, Zoned};
use llrt_utils::result::ResultExt;
use rquickjs::Object;
use rquickjs::{
    atom::PredefinedAtom, class::Trace, Class, Ctx, Exception, JsLifetime, Result, Value,
};

use crate::duration::Duration;
use crate::instant::Instant;
use crate::utils::{span::SpanExt, zoned::ZonedExt};

use super::extract_bigint_or_number;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub struct ZonedDateTime {
    #[qjs(skip_trace)]
    inner: Zoned,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl ZonedDateTime {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, nanos: Value<'_>, tz: Option<String>) -> Result<Self> {
        Self::from_epoch_nanoseconds(&ctx, &nanos, &tz)
    }

    // ---------------------- Static methods ----------------------
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
        if let Some(num) = info.as_number() {
            return Self::from_nanosecond(&ctx, num as i128, &None);
        }
        if let Some(str) = info.as_string().and_then(|s| s.to_string().ok()) {
            return Self::from_str(&ctx, &str);
        }
        Err(Exception::throw_type(
            &ctx,
            "Cannot convert value to Temporal.ZonedDateTime",
        ))
    }

    // ---------------------- Instance methods ----------------------
    fn add(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let span = Span::from_value(&ctx, &duration)?;
        let zoned = self.inner.checked_add(span).map_err_range(&ctx)?;
        Ok(Self { inner: zoned })
    }

    fn equals(&self, other: Self) -> bool {
        self.inner == other.inner
    }

    fn since(&self, other: Self) -> Duration {
        Duration::from_span(self.inner.clone() - other.inner)
    }

    fn subtract(&self, ctx: Ctx<'_>, duration: Value<'_>) -> Result<Self> {
        let span = Span::from_value(&ctx, &duration)?;
        let zoned = self.inner.checked_sub(span).map_err_range(&ctx)?;
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

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToString)]
    fn to_string(&self) -> String {
        self.inner.to_string()
    }

    fn until(&self, other: Self) -> Duration {
        Duration::from_span(other.inner - self.inner.clone())
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
        let zoned = self.inner.in_tz(&tz).map_err_range(&ctx)?;
        Ok(Self { inner: zoned })
    }

    // ---------------------- Instance properties ----------------------
    #[qjs(get)]
    fn day(&self) -> i8 {
        self.inner.day()
    }

    #[qjs(get)]
    fn epoch_milliseconds(&self) -> i64 {
        self.inner.timestamp().as_millisecond()
    }

    #[qjs(get)]
    fn epoch_nanoseconds(&self) -> f64 {
        self.inner.timestamp().as_nanosecond() as f64
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
        let ts = Timestamp::from_nanosecond(ns).map_err_range(ctx)?;
        Self::from_timestamp(ctx, &ts, tz)
    }

    pub fn from_timestamp(ctx: &Ctx<'_>, ts: &Timestamp, tz: &Option<String>) -> Result<Self> {
        let tz = tz.as_deref().unwrap_or("UTC");
        let zoned = ts.in_tz(tz).map_err_range(ctx)?;
        Ok(Self { inner: zoned })
    }

    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let zoned = Zoned::from_str(str).map_err_range(ctx)?;
        Ok(Self { inner: zoned })
    }

    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Self> {
        let zoned = Zoned::from_object(ctx, object)?;
        Ok(Self { inner: zoned })
    }
}
