// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod date;
pub mod date_time;
pub mod round;
pub mod span;
pub mod time;
pub mod total;
pub mod zoned;

use jiff::{RoundMode, Unit};
use rquickjs::{Ctx, Exception, Result};

pub(crate) fn get_unit(ctx: &Ctx, unit: &str) -> Result<Unit> {
    let unit = match unit {
        "day" => Unit::Day,
        "hour" => Unit::Hour,
        "minute" => Unit::Minute,
        "second" => Unit::Second,
        "millisecond" => Unit::Millisecond,
        "microsecond" => Unit::Microsecond,
        "nanosecond" => Unit::Nanosecond,
        _ => return Err(Exception::throw_type(ctx, "Cannot convert to unit")),
    };
    Ok(unit)
}

pub(crate) fn get_duration_unit(ctx: &Ctx, unit: &Option<String>) -> Result<Option<Unit>> {
    let Some(unit) = unit else {
        return Ok(None);
    };
    let unit = match unit.as_str() {
        "years" => Unit::Year,
        "months" => Unit::Month,
        "weeks" => Unit::Week,
        "days" => Unit::Day,
        "hours" => Unit::Hour,
        "minutes" => Unit::Minute,
        "seconds" => Unit::Second,
        "milliseconds" => Unit::Millisecond,
        "microseconds" => Unit::Microsecond,
        "nanoseconds" => Unit::Nanosecond,
        _ => return Err(Exception::throw_type(ctx, "Cannot convert to unit")),
    };
    Ok(Some(unit))
}

pub(crate) fn get_round_mode(ctx: &Ctx, mode: &Option<String>) -> Result<RoundMode> {
    let mode = match mode.clone().unwrap_or_else(|| "halfExpand".into()).as_ref() {
        "ceil" => RoundMode::Ceil,
        "floor" => RoundMode::Floor,
        "expand" => RoundMode::Expand,
        "trunc" => RoundMode::Trunc,
        "halfCeil" => RoundMode::HalfCeil,
        "halfFloor" => RoundMode::HalfFloor,
        "halfExpand" => RoundMode::HalfExpand,
        "halfTrunc" => RoundMode::HalfTrunc,
        "halfEven" => RoundMode::HalfEven,
        _ => return Err(Exception::throw_type(ctx, "Cannot convert to RoundMode")),
    };
    Ok(mode)
}
