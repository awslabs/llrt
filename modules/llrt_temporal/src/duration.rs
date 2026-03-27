// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::Ordering, str::FromStr};

use jiff::{Span, SpanCompare};
use llrt_utils::result::ResultExt;
use rquickjs::{
    atom::PredefinedAtom,
    class::Trace,
    prelude::{Opt, Rest},
    Class, Ctx, Exception, JsLifetime, Object, Result, Value,
};

use crate::utils::date::fill_duration_from_iter as fill_date_from_iter;
use crate::utils::span::round::SpanRoundOption;
use crate::utils::span::total::SpanTotalOption;
use crate::utils::span::SpanExt;
use crate::utils::time::fill_duration_from_iter as fill_time_from_iter;

#[derive(Clone, JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct Duration {
    #[qjs(skip_trace)]
    inner: Span,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Duration {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<Self> {
        let obj = Self::fill_object(&ctx, &args)?;
        Self::from_object(&ctx, &obj)
    }

    #[qjs(static)]
    fn compare(ctx: Ctx<'_>, duration1: Self, duration2: Self, opt: Opt<Value<'_>>) -> Result<i8> {
        let sc = Self::into_span_compare(&duration2.inner, &opt);
        match duration1.inner.compare(sc).or_throw_range(&ctx, "")? {
            Ordering::Less => Ok(-1),
            Ordering::Equal => Ok(0),
            Ordering::Greater => Ok(1),
        }
    }

    #[qjs(static)]
    fn from(ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        Self::from_value(&ctx, &info)
    }

    fn abs(&self) -> Self {
        let span = self.inner.abs();
        Self { inner: span }
    }

    fn add(&self, ctx: Ctx<'_>, other: Value<'_>) -> Result<Self> {
        let duration = Self::from_value(&ctx, &other)?;
        let span = duration.into_inner();
        let span = self.inner.checked_add(span).or_throw_range(&ctx, "")?;
        Ok(Self { inner: span })
    }

    fn negated(&self) -> Self {
        let span = self.inner.negate();
        Self { inner: span }
    }

    fn round(&self, ctx: Ctx<'_>, options: Value<'_>) -> Result<Self> {
        let round = SpanRoundOption::from_value(&ctx, &options)?;
        let round = round.into_inner();
        let dt = self.inner.round(round).or_throw_range(&ctx, "")?;
        Ok(Self { inner: dt })
    }

    fn subtract(&self, ctx: Ctx<'_>, other: Value<'_>) -> Result<Self> {
        let duration = Self::from_value(&ctx, &other)?;
        let span = duration.into_inner();
        let span = self.inner.checked_sub(span).or_throw_range(&ctx, "")?;
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

    fn total(&self, ctx: Ctx<'_>, options: Value<'_>) -> Result<f64> {
        let total = SpanTotalOption::from_value(&ctx, &options)?;
        let total = total.into_inner();
        let num = self.inner.total(total).or_throw_range(&ctx, "")?;
        Ok(num)
    }

    fn value_of(&self, ctx: Ctx<'_>) -> Result<()> {
        Err(Exception::throw_type(
            &ctx,
            "can't convert Duration to primitive type",
        ))
    }

    fn with(&self, ctx: Ctx<'_>, info: Value<'_>) -> Result<Self> {
        let span = self.inner.span_with(&ctx, &info)?;
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
    fn fill_object<'js>(ctx: &Ctx<'js>, args: &Rest<Value<'js>>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        let mut iter = args.0.iter().cloned();
        fill_date_from_iter(&obj, &mut iter)?;
        fill_time_from_iter(&obj, &mut iter)?;
        Ok(obj)
    }

    fn from_object(ctx: &Ctx<'_>, obj: &Object<'_>) -> Result<Self> {
        let span = Span::from_object(ctx, obj)?;
        Ok(Self { inner: span })
    }

    pub(crate) fn from_value(ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        if let Some(obj) = value.as_object() {
            if let Some(cls) = Class::<Self>::from_object(obj) {
                return Ok(cls.borrow().clone());
            }
            return Self::from_object(ctx, obj);
        }

        let str = value
            .as_string()
            .and_then(|s| s.to_string().ok())
            .or_throw_type(ctx, "Cannot convert value to string")?;

        let span = Span::from_str(&str).or_throw_range(ctx, "")?;
        Ok(Self { inner: span })
    }

    pub(crate) fn into_inner(self) -> Span {
        self.inner
    }

    fn into_span_compare<'a>(span: &Span, value: &Opt<Value<'a>>) -> SpanCompare<'a> {
        Span::into_span_compare(span, value)
    }

    pub(crate) fn new_object(span: Span) -> Self {
        Self { inner: span }
    }
}
