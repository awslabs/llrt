// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Build script that generates compact timezone data from chrono-tz.
//!
//! This extracts:
//! 1. Current DST rules for each timezone (compact, ~15KB)
//! 2. Historical transition tables (compressed, ~150KB)

use chrono::{Datelike, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Historical transition data for a timezone.
struct HistoricalTzData {
    /// List of (timestamp, offset_minutes) transitions
    transitions: Vec<(i64, i16)>,
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    generate_compact_data(&out_dir);
    generate_historical_data(&out_dir);

    println!("cargo:rerun-if-changed=build.rs");
}

/// Generate the compact DST rules data.
fn generate_compact_data(out_dir: &str) {
    let path = Path::new(out_dir).join("tz_data.rs");
    let mut file = File::create(path).unwrap();

    let mut timezones: Vec<TimezoneInfo> = Vec::new();

    for tz in chrono_tz::TZ_VARIANTS.iter() {
        let info = analyze_timezone(*tz);
        timezones.push(info);
    }

    // Sort by name for binary search
    timezones.sort_by(|a, b| a.name.cmp(b.name));

    // Generate the timezone array
    writeln!(file, "/// All available timezones, sorted by name.").unwrap();
    writeln!(file, "pub static TZ_VARIANTS: &[Timezone] = &[").unwrap();

    for tz in &timezones {
        write!(
            file,
            "    Timezone {{ name: {:?}, std_offset: {}, ",
            tz.name, tz.std_offset
        )
        .unwrap();

        if let Some(ref rule) = tz.dst_rule {
            writeln!(file, "dst_rule: Some(DstRule {{").unwrap();
            writeln!(
                file,
                "        start: TransitionRule {{ month: {}, week: {}, weekday: {}, time: {} }},",
                rule.start_month, rule.start_week, rule.start_weekday, rule.start_time
            )
            .unwrap();
            writeln!(
                file,
                "        end: TransitionRule {{ month: {}, week: {}, weekday: {}, time: {} }},",
                rule.end_month, rule.end_week, rule.end_weekday, rule.end_time
            )
            .unwrap();
            writeln!(file, "        dst_offset_delta: {},", rule.offset_delta).unwrap();
            write!(file, "    }}), ").unwrap();
        } else {
            write!(file, "dst_rule: None, ").unwrap();
        }

        writeln!(file, "rules_valid_from: {} }},", tz.rules_valid_from).unwrap();
    }

    writeln!(file, "];").unwrap();
    writeln!(file).unwrap();

    // Generate timezone names array
    writeln!(file, "/// All timezone names, sorted alphabetically.").unwrap();
    writeln!(file, "pub static TZ_NAMES: &[&str] = &[").unwrap();
    for tz in &timezones {
        writeln!(file, "    {:?},", tz.name).unwrap();
    }
    writeln!(file, "];").unwrap();
}

/// Generate the compressed historical transition data.
fn generate_historical_data(out_dir: &str) {
    let path = Path::new(out_dir).join("tz_historical.bin");
    let mut file = File::create(path).unwrap();

    // Collect historical transitions for each timezone, keeping the index from TZ_VARIANTS
    let mut tz_data: Vec<HistoricalTzData> = Vec::new();

    let mut tz_variants: Vec<_> = chrono_tz::TZ_VARIANTS.iter().collect();
    tz_variants.sort_by(|a, b| a.name().cmp(b.name()));

    for tz in tz_variants.iter() {
        let transitions = collect_historical_transitions(**tz);
        // Include all timezones, even with empty transitions (for correct indexing)
        tz_data.push(HistoricalTzData { transitions });
    }

    // Serialize all timezone data first
    let mut all_raw_data: Vec<Vec<u8>> = Vec::new();
    for tz in tz_data.iter() {
        let mut raw_data = Vec::new();
        for (ts, offset) in &tz.transitions {
            raw_data.extend_from_slice(&ts.to_le_bytes());
            raw_data.extend_from_slice(&offset.to_le_bytes());
        }
        all_raw_data.push(raw_data);
    }

    // Train a dictionary on all timezone data samples
    // Only use non-empty samples for training
    let samples: Vec<&[u8]> = all_raw_data
        .iter()
        .filter(|d| !d.is_empty())
        .map(|d| d.as_slice())
        .collect();

    // Train dictionary with 32KB size - good balance between size and compression
    let dict = zstd::dict::from_continuous(
        &samples.concat(),
        &samples.iter().map(|s| s.len()).collect::<Vec<_>>(),
        32 * 1024,
    )
    .expect("Failed to train zstd dictionary");

    // Build the binary format with dictionary
    // Format: [magic(4)][tz_count(2)][dict_len(4)][dictionary][index][compressed_data...]
    let mut header = Vec::new();

    // Magic number "LLTZ"
    header.extend_from_slice(&0x5A544C4Cu32.to_le_bytes());
    // Timezone count
    header.extend_from_slice(&(tz_data.len() as u16).to_le_bytes());
    // Dictionary length
    header.extend_from_slice(&(dict.len() as u32).to_le_bytes());

    // Create compressor with dictionary at level 19
    let mut compressor =
        zstd::bulk::Compressor::with_dictionary(19, &dict).expect("Failed to create compressor");

    let mut index = Vec::new();
    let mut data_sections = Vec::new();

    // Calculate offsets: header(10) + dict + index + data
    let index_start = 10 + dict.len();
    let index_size = tz_data.len() * 8; // 2 + 4 + 2 per entry
    let mut data_offset = index_start + index_size;

    for (i, raw_data) in all_raw_data.iter().enumerate() {
        // Compress with dictionary
        let compressed = if raw_data.is_empty() {
            Vec::new()
        } else {
            compressor
                .compress(raw_data)
                .unwrap_or_else(|_| raw_data.clone())
        };

        // Index entry: tz_id (2) + data_offset (4) + data_len (2)
        index.extend_from_slice(&(i as u16).to_le_bytes());
        index.extend_from_slice(&(data_offset as u32).to_le_bytes());
        index.extend_from_slice(&(compressed.len() as u16).to_le_bytes());

        data_offset += compressed.len();
        data_sections.push(compressed);
    }

    // Write everything
    file.write_all(&header).unwrap();
    file.write_all(&dict).unwrap();
    file.write_all(&index).unwrap();
    for section in data_sections {
        file.write_all(&section).unwrap();
    }
}

struct TimezoneInfo {
    name: &'static str,
    std_offset: i16,
    dst_rule: Option<DstRuleInfo>,
    rules_valid_from: i64,
}

struct DstRuleInfo {
    start_month: u8,
    start_week: u8,
    start_weekday: u8,
    start_time: u16,
    end_month: u8,
    end_week: u8,
    end_weekday: u8,
    end_time: u16,
    offset_delta: i16,
}

/// Analyze a timezone to extract its current DST rules.
fn analyze_timezone(tz: Tz) -> TimezoneInfo {
    let name = tz.name();

    // Use 2025 as the reference year for current rules
    let year = 2025;

    // Find offsets for current year
    let jan_dt = NaiveDate::from_ymd_opt(year, 1, 15)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let jan_offset = get_offset_at(tz, jan_dt);

    let jul_dt = NaiveDate::from_ymd_opt(year, 7, 15)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let jul_offset = get_offset_at(tz, jul_dt);

    // Check if current year has DST (Jan and Jul differ)
    let has_dst_current = jan_offset != jul_offset;

    let (std_offset, dst_rule) = if !has_dst_current {
        // No DST in current year - use January offset as standard
        (jan_offset, None)
    } else {
        // Has true DST - determine which is standard and find transition points
        let (std_off, dst_off) = if jan_offset < jul_offset {
            // Northern hemisphere: standard in winter
            (jan_offset, jul_offset)
        } else {
            // Southern hemisphere: standard in summer (southern winter)
            (jul_offset, jan_offset)
        };

        let dst_delta = dst_off - std_off;

        // Find transition dates
        let (start_rule, end_rule) = find_dst_transitions(tz, year, std_off, dst_off);

        (
            std_off,
            Some(DstRuleInfo {
                start_month: start_rule.0,
                start_week: start_rule.1,
                start_weekday: start_rule.2,
                start_time: start_rule.3,
                end_month: end_rule.0,
                end_week: end_rule.1,
                end_weekday: end_rule.2,
                end_time: end_rule.3,
                offset_delta: dst_delta,
            }),
        )
    };

    // Determine when current rules became valid
    // Be conservative: use historical data for all dates before 2024
    // This ensures correctness even for timezones with complex rule histories
    let mut rules_valid_from = find_rules_valid_from(tz, std_offset, &dst_rule);

    // Ensure rules_valid_from is at least 2025-01-01
    // This guarantees historical data is used for all pre-2025 dates
    // (2024 has many mid-year timezone changes that need historical lookup)
    let min_valid_from = NaiveDate::from_ymd_opt(2025, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    if rules_valid_from < min_valid_from {
        rules_valid_from = min_valid_from;
    }

    // Morocco and Western Sahara suspend DST during Ramadan, which follows the
    // Islamic lunar calendar. Since Ramadan shifts ~11 days earlier each year,
    // this cannot be encoded in fixed DST rules. Force historical data lookup
    // for all dates in these timezones.
    if name == "Africa/Casablanca" || name == "Africa/El_Aaiun" {
        rules_valid_from = i64::MAX;
    }

    TimezoneInfo {
        name,
        std_offset,
        dst_rule,
        rules_valid_from,
    }
}

/// Get the offset in minutes at a given datetime.
fn get_offset_at(tz: Tz, dt: NaiveDateTime) -> i16 {
    let utc = Utc.from_utc_datetime(&dt);
    let local = utc.with_timezone(&tz);
    (local.offset().fix().local_minus_utc() / 60) as i16
}

/// DST transition rule: (week_of_month, day_of_week, month, local_time_minutes)
type DstRule = (u8, u8, u8, u16);

/// Find DST transition rules by analyzing when offsets change.
/// Returns rules with LOCAL time (not UTC) for the transition hour.
fn find_dst_transitions(tz: Tz, year: i32, std_offset: i16, dst_offset: i16) -> (DstRule, DstRule) {
    let mut start_transition = None;
    let mut end_transition = None;

    // Scan through the year hour by hour to find transitions
    let mut prev_offset = get_offset_at(
        tz,
        NaiveDate::from_ymd_opt(year, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
    );

    for month in 1..=12u32 {
        let days_in_month = days_in_month(year, month);
        for day in 1..=days_in_month {
            for hour in 0..24u32 {
                let dt = NaiveDate::from_ymd_opt(year, month, day as u32)
                    .unwrap()
                    .and_hms_opt(hour, 0, 0)
                    .unwrap();
                let curr_offset = get_offset_at(tz, dt);

                if curr_offset != prev_offset {
                    // Found a transition
                    // Convert UTC datetime to local datetime to get correct weekday/date
                    let prev_offset_for_local = if curr_offset == dst_offset {
                        // Transitioning TO DST: use standard offset
                        std_offset
                    } else {
                        // Transitioning FROM DST: use DST offset
                        dst_offset
                    };

                    // Calculate local datetime from UTC
                    let utc_timestamp = dt.and_utc().timestamp();
                    let local_timestamp = utc_timestamp + (prev_offset_for_local as i64) * 60;
                    let local_days = (local_timestamp / 86400) as i32;
                    let local_secs = (local_timestamp % 86400) as u32;
                    let local_hour = (local_secs / 3600) as u16;

                    // Convert days since epoch to date
                    let (local_year, local_month, local_day) = days_since_epoch_to_ymd(local_days);
                    let local_date =
                        NaiveDate::from_ymd_opt(local_year, local_month as u32, local_day as u32)
                            .unwrap();
                    let weekday = local_date.weekday().num_days_from_sunday() as u8;
                    let week = week_of_month(local_day);

                    if curr_offset == dst_offset {
                        // Transition to DST
                        start_transition = Some((local_month, week, weekday, local_hour * 60));
                    } else {
                        // Transition from DST
                        end_transition = Some((local_month, week, weekday, local_hour * 60));
                    }
                }
                prev_offset = curr_offset;
            }
        }
    }

    // Default to common patterns if not found
    let start = start_transition.unwrap_or((3, 2, 0, 120)); // 2nd Sunday of March at 2 AM
    let end = end_transition.unwrap_or((11, 1, 0, 120)); // 1st Sunday of November at 2 AM

    (start, end)
}

/// Calculate the week of month (1-4 or 5 for last).
fn week_of_month(day: u8) -> u8 {
    match day {
        1..=7 => 1,
        8..=14 => 2,
        15..=21 => 3,
        22..=28 => 4,
        _ => 5, // Treat as "last"
    }
}

/// Convert days since Unix epoch to year/month/day.
fn days_since_epoch_to_ymd(days: i32) -> (i32, u8, u8) {
    // Shift to March 1, year 0 based counting
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32; // day of era
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year
    let mp = (5 * doy + 2) / 153; // month index (0 = Mar, 11 = Feb)
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Get days in month.
fn days_in_month(year: i32, month: u32) -> u8 {
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

/// Find when the current DST rules became valid by checking backwards.
fn find_rules_valid_from(tz: Tz, std_offset: i16, dst_rule: &Option<DstRuleInfo>) -> i64 {
    // Check years backwards until rules differ
    let current_year = 2025;

    for year in (1970..current_year).rev() {
        // Sample multiple months to catch DST patterns that don't align with Jan/Jul
        // (e.g., Costa Rica had DST in Feb-May)
        let mut offsets = Vec::new();
        for month in [1, 3, 5, 7, 9, 11] {
            let dt = NaiveDate::from_ymd_opt(year, month, 15)
                .unwrap()
                .and_hms_opt(12, 0, 0)
                .unwrap();
            offsets.push(get_offset_at(tz, dt));
        }

        // Use Jan for standard offset comparison
        let jan_offset = offsets[0];
        // Check if any month differs (indicates DST was active)
        let has_dst = offsets.iter().any(|&o| o != jan_offset);
        let rule_has_dst = dst_rule.is_some();

        if has_dst != rule_has_dst {
            // DST status changed
            return NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp();
        }

        if has_dst {
            // Check if the standard offset changed
            // Standard offset is the minimum (for northern) or maximum (for southern) of all offsets
            let year_std = *offsets.iter().min().unwrap();
            let year_dst = *offsets.iter().max().unwrap();
            if year_std != std_offset {
                return NaiveDate::from_ymd_opt(year + 1, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp();
            }

            // Check if the DST transition dates changed
            if let Some(ref rule) = dst_rule {
                let (year_start, year_end) = find_dst_transitions(tz, year, year_std, year_dst);

                // Compare start transition (month, week, weekday)
                if year_start.0 != rule.start_month
                    || year_start.1 != rule.start_week
                    || year_start.2 != rule.start_weekday
                {
                    return NaiveDate::from_ymd_opt(year + 1, 1, 1)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp();
                }

                // Compare end transition (month, week, weekday)
                if year_end.0 != rule.end_month
                    || year_end.1 != rule.end_week
                    || year_end.2 != rule.end_weekday
                {
                    return NaiveDate::from_ymd_opt(year + 1, 1, 1)
                        .unwrap()
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp();
                }
            }
        } else {
            // No DST - check if the standard offset changed
            if jan_offset != std_offset {
                return NaiveDate::from_ymd_opt(year + 1, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp();
            }
        }
    }

    // Rules have been stable since 1970
    0
}

/// Collect historical transitions for a timezone.
fn collect_historical_transitions(tz: Tz) -> Vec<(i64, i16)> {
    let mut transitions = Vec::new();

    // Scan from 1970 to 2025 and record offset changes at hourly granularity
    let mut prev_offset: Option<i16> = None;

    for year in 1970..=2025 {
        for month in 1..=12u32 {
            let days = days_in_month(year, month);
            for day in 1..=days {
                for hour in 0..24u32 {
                    let dt = NaiveDate::from_ymd_opt(year, month, day as u32)
                        .unwrap()
                        .and_hms_opt(hour, 0, 0)
                        .unwrap();
                    let offset = get_offset_at(tz, dt);

                    if prev_offset != Some(offset) {
                        let ts = dt.and_utc().timestamp();
                        transitions.push((ts, offset));
                        prev_offset = Some(offset);
                    }
                }
            }
        }
    }

    // Remove the first entry if it's just the initial state
    if transitions.len() > 1 {
        transitions
    } else {
        Vec::new()
    }
}
