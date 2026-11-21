// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::str::FromStr;

use jiff::{
    tz::{Offset, TimeZone},
    ZonedDateTime as Zoned,
};
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result};

pub(crate) fn parse_temporal_time_zone(
    ctx: &Ctx<'_>,
    timezone: Option<&String>,
) -> Result<TimeZone> {
    let tz = match timezone {
        Some(s) => {
            if let Some(offset) = parse_temporal_offset(s) {
                TimeZone::fixed(offset)
            } else {
                match TimeZone::get(s) {
                    Ok(tz) => tz,
                    Err(_) => Zoned::from_str(s)
                        .or_throw_range(ctx, "")?
                        .time_zone()
                        .clone(),
                }
            }
        },
        None => TimeZone::UTC,
    };

    Ok(tz)
}

fn parse_temporal_offset(s: &str) -> Option<Offset> {
    Offset::from_seconds(parse_offset_seconds(s, false)?).ok()
}

pub(crate) fn parse_offset_seconds(s: &str, with_seconds: bool) -> Option<i32> {
    let bytes = s.as_bytes();
    let expected_len = if with_seconds { 9 } else { 6 };

    if bytes.len() != expected_len {
        return None;
    }

    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };

    if bytes[3] != b':' || (with_seconds && bytes[6] != b':') {
        return None;
    }

    let hours = parse_two_digits(&bytes[1..3])?;
    let minutes = parse_two_digits(&bytes[4..6])?;
    let seconds = if with_seconds {
        parse_two_digits(&bytes[7..9])?
    } else {
        0
    };

    if hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }

    Some(sign * ((hours * 60 + minutes) * 60 + seconds))
}

fn parse_two_digits(bytes: &[u8]) -> Option<i32> {
    let [a, b] = *bytes else {
        return None;
    };

    if !a.is_ascii_digit() || !b.is_ascii_digit() {
        return None;
    }

    Some(i32::from(a - b'0') * 10 + i32::from(b - b'0'))
}
