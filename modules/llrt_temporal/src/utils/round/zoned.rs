// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{RoundMode, Unit, ZonedRound};
use rquickjs::{Ctx, Exception, Result};

use super::{RoundBuilder, RoundOption};

impl RoundBuilder for ZonedRound {
    fn new() -> Self {
        ZonedRound::new()
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
            "day" => Ok(Unit::Day),
            "hour" => Ok(Unit::Hour),
            "minute" => Ok(Unit::Minute),
            "second" => Ok(Unit::Second),
            "millisecond" => Ok(Unit::Millisecond),
            "microsecond" => Ok(Unit::Microsecond),
            "nanosecond" => Ok(Unit::Nanosecond),
            _ => Err(Exception::throw_type(
                ctx,
                "smallestUnit is invalid for ZonedRound",
            )),
        }
    }
}

pub(crate) type ZonedRoundOption = RoundOption<ZonedRound>;
