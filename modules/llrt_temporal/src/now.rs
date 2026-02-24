// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::time::{SystemTime, UNIX_EPOCH};

use rquickjs::{prelude::Opt, Ctx, Result};

use crate::instant::Instant;
use crate::zoned_date_time::ZonedDateTime;

pub(crate) fn instant(ctx: Ctx<'_>) -> Result<Instant> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i128)
        .unwrap_or_else(|e| -(e.duration().as_nanos() as i128));

    Instant::from_nanosecond(&ctx, nanos)
}

pub(crate) fn zoned_datetime_iso(ctx: Ctx<'_>, timezone: Opt<String>) -> Result<ZonedDateTime> {
    let ts = instant(ctx.clone())?;
    ZonedDateTime::from_timestamp(&ctx, &ts.timestamp(), &timezone)
}
