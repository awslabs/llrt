// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Compact timezone representation using DST rules instead of transition tables.
//!
//! Each timezone is ~20 bytes instead of kilobytes of historical transitions.

use crate::historical;

// Time constants
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: u32 = 3600;
const SECONDS_PER_DAY: i64 = 86400;

// Calendar constants for Howard Hinnant's date algorithms
/// Days from March 1, year 0 to Unix epoch (January 1, 1970)
const DAYS_FROM_CIVIL_EPOCH_TO_UNIX_EPOCH: i32 = 719468;
/// Days in a 400-year era (146097 = 400*365 + 97 leap days)
const DAYS_PER_ERA: i32 = 146097;

/// A compact timezone with DST rules.
#[derive(Debug, Clone, Copy)]
pub struct Timezone {
    /// IANA timezone name (e.g., "America/New_York")
    pub name: &'static str,
    /// Standard time UTC offset in minutes (e.g., -300 for UTC-5)
    pub std_offset: i16,
    /// DST rule, if this timezone observes daylight saving time
    pub dst_rule: Option<DstRule>,
    /// Unix timestamp when current rules became valid.
    /// For dates before this, we need historical data.
    pub rules_valid_from: i64,
}

/// Daylight Saving Time rule.
#[derive(Debug, Clone, Copy)]
pub struct DstRule {
    /// When DST starts
    pub start: TransitionRule,
    /// When DST ends (returns to standard time)
    pub end: TransitionRule,
    /// Additional offset during DST in minutes (typically +60)
    pub dst_offset_delta: i16,
}

/// Rule for when a DST transition occurs.
/// Encodes patterns like "second Sunday of March at 2:00 AM".
#[derive(Debug, Clone, Copy)]
pub struct TransitionRule {
    /// Month (1-12)
    pub month: u8,
    /// Week of month (1-4 for "first" through "fourth", 5 for "last")
    pub week: u8,
    /// Day of week (0 = Sunday, 1 = Monday, ..., 6 = Saturday)
    pub weekday: u8,
    /// Time of day in minutes from midnight (e.g., 120 for 2:00 AM)
    pub time: u16,
}

impl Timezone {
    /// Get the UTC offset in minutes for a given Unix timestamp (in seconds).
    ///
    /// Returns positive values for timezones ahead of UTC (e.g., +60 for UTC+1)
    /// and negative values for timezones behind UTC (e.g., -300 for UTC-5).
    #[inline]
    pub fn offset_at(&self, timestamp_secs: i64) -> i16 {
        // Check if we need historical data
        if timestamp_secs < self.rules_valid_from {
            return historical::get_historical_offset(self.name, timestamp_secs)
                .unwrap_or(self.std_offset);
        }

        // Use current rules
        match &self.dst_rule {
            Some(rule) => {
                if self.is_dst_active(timestamp_secs, rule) {
                    self.std_offset + rule.dst_offset_delta
                } else {
                    self.std_offset
                }
            },
            None => self.std_offset,
        }
    }

    /// Check if DST is active at the given timestamp.
    fn is_dst_active(&self, timestamp_secs: i64, rule: &DstRule) -> bool {
        // Convert timestamp to year in local standard time
        let (year, _month, _day, _hour, _minute) =
            timestamp_to_local(timestamp_secs, self.std_offset);

        // Calculate DST transition timestamps (in UTC) for this year
        // DST start: transition happens at local standard time
        let dst_start_utc = transition_timestamp_utc(year, &rule.start, self.std_offset);
        // DST end: transition happens at local DST time
        let dst_end_utc =
            transition_timestamp_utc(year, &rule.end, self.std_offset + rule.dst_offset_delta);

        // Handle northern vs southern hemisphere (start < end vs start > end)
        if dst_start_utc < dst_end_utc {
            // Northern hemisphere: DST is between start and end
            timestamp_secs >= dst_start_utc && timestamp_secs < dst_end_utc
        } else {
            // Southern hemisphere: DST is outside the range (wraps around year end)
            timestamp_secs >= dst_start_utc || timestamp_secs < dst_end_utc
        }
    }
}

/// Convert a Unix timestamp to local datetime components.
fn timestamp_to_local(timestamp_secs: i64, offset_minutes: i16) -> (i32, u8, u8, u8, u8) {
    let local_secs = timestamp_secs + (offset_minutes as i64) * SECONDS_PER_MINUTE;

    // Days since Unix epoch
    let days = (local_secs / SECONDS_PER_DAY) as i32;
    let remaining_secs = (local_secs % SECONDS_PER_DAY) as u32;

    // Convert days to year/month/day (shift to March 1, year 0 for algorithm)
    let (year, month, day) = days_to_ymd(days + DAYS_FROM_CIVIL_EPOCH_TO_UNIX_EPOCH);

    let hour = (remaining_secs / SECONDS_PER_HOUR) as u8;
    let minute = ((remaining_secs % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE as u32) as u8;

    (year, month, day, hour, minute)
}

/// Convert days since March 1, year 0 to year/month/day.
/// Based on Howard Hinnant's date algorithms.
fn days_to_ymd(days: i32) -> (i32, u8, u8) {
    let era = if days >= 0 {
        days
    } else {
        days - (DAYS_PER_ERA - 1)
    } / DAYS_PER_ERA;
    let doe = (days - era * DAYS_PER_ERA) as u32; // day of era
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year
    let mp = (5 * doy + 2) / 153; // month index (0 = Mar, 11 = Feb)
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Convert year/month/day to days since March 1, year 0.
fn ymd_to_days(year: i32, month: u8, day: u8) -> i32 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = month as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as u32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * DAYS_PER_ERA + doe as i32
}

/// Calculate the UTC Unix timestamp of a DST transition.
/// The rule.time is in local time (minutes from midnight), and offset_minutes
/// is the offset that is active at the moment of transition.
fn transition_timestamp_utc(year: i32, rule: &TransitionRule, offset_minutes: i16) -> i64 {
    // Find the first day of the target month (convert to Unix days)
    let first_of_month = ymd_to_days(year, rule.month, 1) - DAYS_FROM_CIVIL_EPOCH_TO_UNIX_EPOCH;

    // Calculate day of week for first of month (0 = Thursday for Unix epoch)
    let first_dow = ((first_of_month % 7 + 4 + 7) % 7) as u8;

    // Find the nth occurrence of the target weekday
    let target_day = if rule.week == 5 {
        // "Last" occurrence - find last weekday in month
        let days_in_month = days_in_month(year, rule.month);
        let last_of_month = first_of_month + days_in_month as i32 - 1;
        let last_dow = ((last_of_month % 7 + 4 + 7) % 7) as u8;
        let days_back = (last_dow + 7 - rule.weekday) % 7;
        last_of_month - days_back as i32
    } else {
        // Nth occurrence
        let days_forward = (rule.weekday + 7 - first_dow) % 7;
        first_of_month + days_forward as i32 + (rule.week as i32 - 1) * 7
    };

    // Convert to UTC timestamp:
    // target_day * SECONDS_PER_DAY gives us midnight UTC on that day
    // rule.time is local time in minutes from midnight
    // We need to convert local time to UTC by subtracting the offset
    let local_timestamp =
        target_day as i64 * SECONDS_PER_DAY + rule.time as i64 * SECONDS_PER_MINUTE;
    local_timestamp - (offset_minutes as i64) * SECONDS_PER_MINUTE
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        },
        _ => 30,
    }
}

// Include the generated timezone data
include!(concat!(env!("OUT_DIR"), "/tz_data.rs"));

/// Get the UTC offset in minutes for a timezone at a given epoch milliseconds.
pub fn get_offset(timezone_name: &str, epoch_ms: f64) -> Option<i16> {
    let tz = lookup_timezone(timezone_name)?;
    let timestamp_secs = (epoch_ms / 1000.0) as i64;
    Some(tz.offset_at(timestamp_secs))
}

/// Look up a timezone by name.
pub fn lookup_timezone(name: &str) -> Option<&'static Timezone> {
    // Binary search since TZ_VARIANTS is sorted
    TZ_VARIANTS
        .binary_search_by(|tz| tz.name.cmp(name))
        .ok()
        .map(|idx| &TZ_VARIANTS[idx])
}

/// List all available timezone names.
pub fn list_timezones() -> &'static [&'static str] {
    TZ_NAMES
}
