// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use jiff::{RoundMode, Unit, ZonedRound};

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
}

pub(crate) type ZonedRoundOption = RoundOption<ZonedRound>;
