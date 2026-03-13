// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{RoundMode, TimestampRound, Unit};

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
}

pub(crate) type TimestampRoundOption = RoundOption<TimestampRound>;
