// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod date_time;
pub mod span;
pub mod time;
pub mod timestamp;
pub mod zoned;

use jiff::{RoundMode, Unit};
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result, Value};

use crate::utils::{get_round_mode, get_unit};

pub(crate) trait RoundBuilder: Sized {
    fn new() -> Self;
    fn smallest(self, unit: Unit) -> Self;
    fn mode(self, mode: RoundMode) -> Self;
    fn increment(self, increment: i64) -> Self;
}

pub(crate) struct RoundOption<T> {
    inner: T,
}

impl<T: RoundBuilder> RoundOption<T> {
    pub(crate) fn from_value(ctx: &Ctx<'_>, value: &Value<'_>) -> Result<Self> {
        if let Some(obj) = value.as_object() {
            let unit = obj
                .get::<_, String>("smallestUnit")
                .or_throw_range(ctx, "")?;
            let mode = obj.get::<_, String>("roundingMode").ok();
            let increment = obj.get::<_, i64>("roundingIncrement").ok();
            let round = Self::from(ctx, &unit, &mode, &increment)?;
            return Ok(Self { inner: round });
        }

        let unit = value
            .as_string()
            .and_then(|s| s.to_string().ok())
            .or_throw_type(ctx, "Cannot convert value to string")?;
        let round = Self::from(ctx, &unit, &None, &None)?;
        Ok(Self { inner: round })
    }

    pub(crate) fn into_inner(self) -> T {
        self.inner
    }

    fn from(ctx: &Ctx, unit: &str, mode: &Option<String>, increment: &Option<i64>) -> Result<T> {
        let unit = get_unit(ctx, unit).or_throw_range(ctx, "")?;
        let mode = get_round_mode(ctx, mode)?;
        let increment = increment.unwrap_or(1);
        Ok(T::new().smallest(unit).mode(mode).increment(increment))
    }
}
