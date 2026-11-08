// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::str::FromStr;

use jiff::{
    tz::{Offset, TimeZone},
    Zoned,
};
use llrt_utils::result::ResultExt;
use rquickjs::{Ctx, Result};

pub(crate) fn parse_temporal_time_zone(
    ctx: &Ctx<'_>,
    timezone: &Option<String>,
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
    let bytes = s.as_bytes();

    if bytes.len() != 6 {
        return None;
    }

    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };

    if bytes[3] != b':' {
        return None;
    }

    let hours = parse_two_digits(&bytes[1..3])?;
    let minutes = parse_two_digits(&bytes[4..6])?;

    if hours > 23 || minutes > 59 {
        return None;
    }

    let seconds = sign * (hours * 60 + minutes) * 60;

    Offset::from_seconds(seconds).ok()
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

pub(crate) fn timezone_rules(name: &str) -> Option<&[u8]> {
    let name = normalize_time_zone_id(name);
    jiff_tzdb::get(name).map(|(_, offset)| offset)
}

fn normalize_time_zone_id(name: &str) -> &str {
    match name {
        "UTC" | "Etc/UTC" | "Etc/GMT" | "Etc/GMT0" | "Etc/GMT+0" | "Etc/GMT-0" | "GMT" | "GMT0"
        | "GMT+0" | "GMT-0" | "Etc/UCT" | "UCT" | "Etc/Universal" | "Universal" | "Etc/Zulu"
        | "Zulu" | "Greenwich" | "Etc/Greenwich" => "UTC",
        _ => name,
    }
}
