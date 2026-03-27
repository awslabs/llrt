// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{civil::DateTimeRound, RoundMode, Unit};

use crate::utils::{RoundBuilder, RoundOption};

impl RoundBuilder for DateTimeRound {
    fn new() -> Self {
        DateTimeRound::new()
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

pub(crate) type DateTimeRoundOption = RoundOption<DateTimeRound>;
