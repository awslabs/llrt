// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Chrono-compatible timezone wrapper.
//!
//! This module provides a `Tz` type that implements chrono's `TimeZone` trait,
//! allowing seamless integration with chrono's datetime types.

use std::fmt;
use std::str::FromStr;

use chrono::{FixedOffset, LocalResult, NaiveDate, NaiveDateTime, TimeZone};

/// A timezone that can be used with chrono.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tz {
    /// UTC timezone (no offset, no DST)
    Utc,
    /// A named IANA timezone
    Named(TzInner),
}

/// Inner timezone data (index into TZ_VARIANTS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TzInner {
    /// Index into TZ_VARIANTS array
    index: u16,
}

impl Tz {
    /// Get the IANA name of this timezone.
    pub fn name(&self) -> &'static str {
        match self {
            Tz::Utc => "UTC",
            Tz::Named(inner) => crate::TZ_VARIANTS
                .get(inner.index as usize)
                .map(|tz| tz.name)
                .unwrap_or("UTC"),
        }
    }

    /// Get the UTC offset in minutes at a given Unix timestamp (in seconds).
    pub fn offset_at_timestamp(&self, timestamp_secs: i64) -> i16 {
        match self {
            Tz::Utc => 0,
            Tz::Named(inner) => crate::TZ_VARIANTS
                .get(inner.index as usize)
                .map(|tz| tz.offset_at(timestamp_secs))
                .unwrap_or(0),
        }
    }

    /// Get the UTC offset in seconds at a given Unix timestamp (in seconds).
    pub fn offset_seconds_at(&self, timestamp_secs: i64) -> i32 {
        self.offset_at_timestamp(timestamp_secs) as i32 * 60
    }
}

impl fmt::Display for Tz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for Tz {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "UTC" || s == "Etc/UTC" || s == "Etc/GMT" {
            return Ok(Tz::Utc);
        }

        // Find the timezone in our list
        match crate::TZ_VARIANTS.binary_search_by(|tz| tz.name.cmp(s)) {
            Ok(idx) => Ok(Tz::Named(TzInner { index: idx as u16 })),
            Err(_) => Err(ParseError {
                name: s.to_string(),
            }),
        }
    }
}

/// Error returned when parsing an invalid timezone name.
#[derive(Debug, Clone)]
pub struct ParseError {
    name: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid timezone: {}", self.name)
    }
}

impl std::error::Error for ParseError {}

/// A fixed UTC offset returned by timezone calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TzOffset {
    /// Offset in seconds from UTC
    offset_secs: i32,
}

impl TzOffset {
    /// Create a new offset from seconds.
    pub fn from_seconds(secs: i32) -> Self {
        Self { offset_secs: secs }
    }

    /// Get the offset in seconds (positive = east of UTC).
    pub fn local_minus_utc(&self) -> i32 {
        self.offset_secs
    }

    /// Get the offset in minutes.
    pub fn local_minus_utc_minutes(&self) -> i16 {
        (self.offset_secs / 60) as i16
    }
}

impl fmt::Display for TzOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_mins = self.offset_secs / 60;
        let hours = total_mins / 60;
        let mins = (total_mins % 60).abs();
        if mins == 0 {
            write!(f, "{:+03}:00", hours)
        } else {
            write!(f, "{:+03}:{:02}", hours, mins)
        }
    }
}

// Implement chrono's Offset trait for TzOffset
impl chrono::Offset for TzOffset {
    fn fix(&self) -> FixedOffset {
        FixedOffset::east_opt(self.offset_secs).unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
    }
}

// Implement chrono's TimeZone trait for Tz
impl TimeZone for Tz {
    type Offset = TzOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        // We can't reconstruct the exact timezone from just an offset,
        // so we return UTC if offset is 0, otherwise this is a limitation
        if offset.offset_secs == 0 {
            Tz::Utc
        } else {
            // This is a fallback - in practice, offsets come from our own timezones
            Tz::Utc
        }
    }

    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<Self::Offset> {
        // Use noon on the given date to determine offset
        let noon = local.and_hms_opt(12, 0, 0).unwrap();
        self.offset_from_local_datetime(&noon)
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        // For local -> UTC conversion, we need to estimate the UTC timestamp
        // This is tricky around DST transitions, but we use a simple approximation
        let approx_utc_ts = local.and_utc().timestamp();
        let offset_secs = self.offset_seconds_at(approx_utc_ts);
        LocalResult::Single(TzOffset::from_seconds(offset_secs))
    }

    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset {
        let noon = utc.and_hms_opt(12, 0, 0).unwrap();
        self.offset_from_utc_datetime(&noon)
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset {
        let timestamp = utc.and_utc().timestamp();
        let offset_secs = self.offset_seconds_at(timestamp);
        TzOffset::from_seconds(offset_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_utc() {
        assert_eq!("UTC".parse::<Tz>().unwrap(), Tz::Utc);
        assert_eq!("Etc/UTC".parse::<Tz>().unwrap(), Tz::Utc);
    }

    #[test]
    fn test_parse_named() {
        let tz: Tz = "America/New_York".parse().unwrap();
        assert_eq!(tz.name(), "America/New_York");
    }

    #[test]
    fn test_parse_invalid() {
        assert!("Invalid/Zone".parse::<Tz>().is_err());
    }

    #[test]
    fn test_utc_offset() {
        let tz = Tz::Utc;
        assert_eq!(tz.offset_at_timestamp(0), 0);
        assert_eq!(tz.offset_at_timestamp(1700000000), 0);
    }

    #[test]
    fn test_named_offset() {
        let tz: Tz = "America/New_York".parse().unwrap();
        // Winter (EST = UTC-5)
        let jan_2024 = 1704067200; // 2024-01-01 00:00:00 UTC
        assert_eq!(tz.offset_at_timestamp(jan_2024), -300);

        // Summer (EDT = UTC-4)
        let jul_2024 = 1720000000; // July 3, 2024
        assert_eq!(tz.offset_at_timestamp(jul_2024), -240);
    }
}
