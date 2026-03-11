// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{RoundMode, TimestampRound, Unit};
use rquickjs::{Ctx, Exception, Result};

use super::{RoundBuilder, RoundOption};

impl RoundBuilder for TimestampRound {
    fn new() -> Self {
        TimestampRound::new()
    }

    fn smallest(self, unit: Unit) -> Self {
        self.smallest(unit)
    }

    fn mode(self, mode: RoundMode) -> Self {
        self.mode(mode)
    }

    fn increment(self, increment: i64) -> Self {
        self.increment(increment)
    }

    fn parse_unit(ctx: &Ctx, unit: &str) -> Result<Unit> {
        match unit {
            "hour" => Ok(Unit::Hour),
            "minute" => Ok(Unit::Minute),
            "second" => Ok(Unit::Second),
            "millisecond" => Ok(Unit::Millisecond),
            "microsecond" => Ok(Unit::Microsecond),
            "nanosecond" => Ok(Unit::Nanosecond),
            _ => Err(Exception::throw_type(
                ctx,
                "smallestUnit is invalid for TimestampRound",
            )),
        }
    }
}

pub(crate) type TimestampRoundOption = RoundOption<TimestampRound>;
