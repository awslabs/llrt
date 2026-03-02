// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cmp::Ordering;

use jiff::Span;
use llrt_utils::result::{By, ResultExt};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, Class, Ctx, Exception, JsLifetime, Object, Result, Value,
};

use crate::utils::span::SpanExt;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub struct Duration {
    #[qjs(skip_trace)]
    inner: Span,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Duration {
    #[qjs(constructor)]
    fn new() -> Result<Self> {
        Ok(Self { inner: Span::new() })
    }

    #[qjs(static)]
    fn compare(ctx: Ctx<'_>, duration1: Self, duration2: Self) -> Result<i8> {
        match duration1.inner.compare(duration2.inner).or_throw(&ctx)? {
            Ordering::Less => Ok(-1),
            Ordering::Equal => Ok(0),
            Ordering::Greater => Ok(1),
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
            "Cannot convert value to Temporal.Duration",
        ))
    }

    fn abs(&self) -> Self {
        let span = self.inner.abs();
        Self { inner: span }
    }

    fn add(&self, ctx: Ctx<'_>, other: Value<'_>) -> Result<Self> {
        let span = Span::from_value(&ctx, &other)?;
        let span = self.inner.checked_add(span).or_throw_by(&ctx, By::Range)?;
        Ok(Self { inner: span })
    }

    fn negated(&self) -> Self {
        let span = self.inner.negate();
        Self { inner: span }
    }

    fn subtract(&self, ctx: Ctx<'_>, other: Value<'_>) -> Result<Self> {
        let span = Span::from_value(&ctx, &other)?;
        let span = self.inner.checked_sub(span).or_throw_by(&ctx, By::Range)?;
        Ok(Self { inner: span })
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

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert Duration to primitive type",
        ))
    }

    fn with(&self, ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        let span = self.inner.with(&ctx, &info)?;
        Ok(Self { inner: span })
    }

    #[qjs(get)]
    fn blank(&self) -> bool {
        self.inner.signum() == 0
    }

    #[qjs(get)]
    fn days(&self) -> i32 {
        self.inner.get_days()
    }

    #[qjs(get)]
    fn hours(&self) -> i32 {
        self.inner.get_hours()
    }

    #[qjs(get)]
    fn microseconds(&self) -> i64 {
        self.inner.get_microseconds()
    }

    #[qjs(get)]
    fn milliseconds(&self) -> i64 {
        self.inner.get_milliseconds()
    }

    #[qjs(get)]
    fn minutes(&self) -> i64 {
        self.inner.get_minutes()
    }

    #[qjs(get)]
    fn months(&self) -> i32 {
        self.inner.get_months()
    }

    #[qjs(get)]
    fn nanoseconds(&self) -> i64 {
        self.inner.get_nanoseconds()
    }

    #[qjs(get)]
    fn seconds(&self) -> i64 {
        self.inner.get_seconds()
    }

    #[qjs(get)]
    fn sign(&self) -> i8 {
        self.inner.signum()
    }

    #[qjs(get)]
    fn weeks(&self) -> i32 {
        self.inner.get_weeks()
    }

    #[qjs(get)]
    fn years(&self) -> i16 {
        self.inner.get_years()
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    fn to_string_tag(&self) -> &'static str {
        "Temporal.Duration"
    }
}

impl Duration {
    fn from_str(ctx: &Ctx<'_>, str: &str) -> Result<Self> {
        let span = str.parse().or_throw_by(ctx, By::Range)?;
        Ok(Self { inner: span })
    }

    fn from_object(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<Self> {
        let span = Span::from_object(ctx, object)?;
        Ok(Self { inner: span })
    }

    pub fn from_span(span: Span) -> Self {
        Self { inner: span }
    }
}
