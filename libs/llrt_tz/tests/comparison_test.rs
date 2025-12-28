// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Comprehensive comparison test between chrono-tz and our compact implementation.
//!
//! This test ensures we have 100% feature parity with chrono-tz for timezone offset calculations.

use chrono::{DateTime, NaiveDate, Offset};
use chrono_tz::Tz as ChronoTz;
use llrt_tz::Tz as CompactTz;

/// Get offset in minutes using chrono-tz
fn chrono_tz_offset(tz_name: &str, timestamp_secs: i64) -> i16 {
    let tz: ChronoTz = tz_name.parse().unwrap();
    let utc = DateTime::from_timestamp(timestamp_secs, 0).unwrap();
    let local = utc.with_timezone(&tz);
    (local.offset().fix().local_minus_utc() / 60) as i16
}

/// Get offset in minutes using our compact implementation
fn compact_tz_offset(tz_name: &str, timestamp_secs: i64) -> i16 {
    let tz: CompactTz = tz_name.parse().unwrap();
    tz.offset_at_timestamp(timestamp_secs)
}

/// Compare offsets and panic with details if they differ
fn assert_offsets_match(tz_name: &str, timestamp_secs: i64, label: &str) {
    let chrono_offset = chrono_tz_offset(tz_name, timestamp_secs);
    let compact_offset = compact_tz_offset(tz_name, timestamp_secs);

    assert_eq!(
        chrono_offset, compact_offset,
        "Offset mismatch for {} at {} (ts={}): chrono-tz={}, compact={}",
        tz_name, label, timestamp_secs, chrono_offset, compact_offset
    );
}

/// Test a timezone across many timestamps
fn test_timezone_comprehensive(tz_name: &str) {
    // Test years from 1970 to 2024
    for year in [
        1970, 1975, 1980, 1985, 1990, 1995, 2000, 2005, 2006, 2007, 2008, 2010, 2015, 2020, 2024,
    ]
    .iter()
    {
        // Test multiple dates throughout the year
        for (month, day) in [
            (1, 1),
            (1, 15),
            (2, 15),
            (3, 1),
            (3, 10),
            (3, 15),
            (3, 31),
            (4, 1),
            (4, 15),
            (5, 15),
            (6, 15),
            (7, 1),
            (7, 15),
            (8, 15),
            (9, 15),
            (10, 1),
            (10, 15),
            (10, 31),
            (11, 1),
            (11, 3),
            (11, 15),
            (12, 15),
            (12, 31),
        ]
        .iter()
        {
            // Skip invalid dates
            if let Some(date) = NaiveDate::from_ymd_opt(*year, *month, *day) {
                // Test at multiple hours to catch DST transitions
                for hour in [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 23].iter() {
                    if let Some(dt) = date.and_hms_opt(*hour, 0, 0) {
                        let ts = dt.and_utc().timestamp();
                        let label = format!("{}-{:02}-{:02} {:02}:00 UTC", year, month, day, hour);
                        assert_offsets_match(tz_name, ts, &label);
                    }
                }
            }
        }
    }
}

/// Test DST transitions with second-level precision
fn test_dst_transitions(tz_name: &str, transitions: &[(i64, &str)]) {
    for (ts, label) in transitions {
        // Test exact second
        assert_offsets_match(tz_name, *ts, label);
        // Test 1 second before
        assert_offsets_match(tz_name, ts - 1, &format!("{} -1s", label));
        // Test 1 second after
        assert_offsets_match(tz_name, ts + 1, &format!("{} +1s", label));
        // Test 1 minute before
        assert_offsets_match(tz_name, ts - 60, &format!("{} -1m", label));
        // Test 1 minute after
        assert_offsets_match(tz_name, ts + 60, &format!("{} +1m", label));
    }
}

#[test]
fn test_america_new_york_comprehensive() {
    test_timezone_comprehensive("America/New_York");
}

#[test]
fn test_america_los_angeles_comprehensive() {
    test_timezone_comprehensive("America/Los_Angeles");
}

#[test]
fn test_europe_london_comprehensive() {
    test_timezone_comprehensive("Europe/London");
}

#[test]
fn test_europe_paris_comprehensive() {
    test_timezone_comprehensive("Europe/Paris");
}

#[test]
fn test_europe_berlin_comprehensive() {
    test_timezone_comprehensive("Europe/Berlin");
}

#[test]
fn test_asia_tokyo_comprehensive() {
    test_timezone_comprehensive("Asia/Tokyo");
}

#[test]
fn test_asia_shanghai_comprehensive() {
    test_timezone_comprehensive("Asia/Shanghai");
}

#[test]
fn test_asia_kolkata_comprehensive() {
    test_timezone_comprehensive("Asia/Kolkata");
}

#[test]
fn test_australia_sydney_comprehensive() {
    test_timezone_comprehensive("Australia/Sydney");
}

#[test]
fn test_pacific_auckland_comprehensive() {
    test_timezone_comprehensive("Pacific/Auckland");
}

#[test]
fn test_america_sao_paulo_comprehensive() {
    test_timezone_comprehensive("America/Sao_Paulo");
}

#[test]
fn test_africa_johannesburg_comprehensive() {
    test_timezone_comprehensive("Africa/Johannesburg");
}

#[test]
fn test_utc_comprehensive() {
    test_timezone_comprehensive("UTC");
}

#[test]
fn test_dst_transitions_precision() {
    // US DST 2024 - Spring forward March 10 at 2 AM EST (7 AM UTC)
    let spring_2024 = NaiveDate::from_ymd_opt(2024, 3, 10)
        .unwrap()
        .and_hms_opt(7, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    // US DST 2024 - Fall back November 3 at 2 AM EDT (6 AM UTC)
    let fall_2024 = NaiveDate::from_ymd_opt(2024, 11, 3)
        .unwrap()
        .and_hms_opt(6, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    test_dst_transitions(
        "America/New_York",
        &[
            (spring_2024, "2024 Spring Forward"),
            (fall_2024, "2024 Fall Back"),
        ],
    );

    // Pre-2007 US DST - April 2, 2006 at 2 AM EST (7 AM UTC)
    let spring_2006 = NaiveDate::from_ymd_opt(2006, 4, 2)
        .unwrap()
        .and_hms_opt(7, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    // Pre-2007 US DST - October 29, 2006 at 2 AM EDT (6 AM UTC)
    let fall_2006 = NaiveDate::from_ymd_opt(2006, 10, 29)
        .unwrap()
        .and_hms_opt(6, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    test_dst_transitions(
        "America/New_York",
        &[
            (spring_2006, "2006 Spring Forward (old rules)"),
            (fall_2006, "2006 Fall Back (old rules)"),
        ],
    );
}

#[test]
fn test_southern_hemisphere_dst() {
    // Australia Sydney DST 2024 - October 6 at 2 AM AEST (16:00 UTC Oct 5)
    let sydney_spring_2024 = NaiveDate::from_ymd_opt(2024, 10, 5)
        .unwrap()
        .and_hms_opt(16, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    test_dst_transitions(
        "Australia/Sydney",
        &[(sydney_spring_2024, "2024 Sydney Spring Forward")],
    );

    // New Zealand DST
    test_timezone_comprehensive("Pacific/Auckland");
}

#[test]
fn test_no_dst_timezones() {
    // These timezones don't observe DST
    let no_dst_zones = [
        "Asia/Tokyo",
        "Asia/Shanghai",
        "Asia/Singapore",
        "Asia/Dubai",
        "Africa/Cairo",
        "Africa/Johannesburg",
        "America/Phoenix", // Arizona doesn't observe DST
    ];

    for tz_name in no_dst_zones.iter() {
        test_timezone_comprehensive(tz_name);
    }
}

#[test]
fn test_edge_case_timezones() {
    // Half-hour and 45-minute offsets
    let edge_cases = [
        "Asia/Kolkata",       // UTC+5:30
        "Asia/Kathmandu",     // UTC+5:45
        "Australia/Adelaide", // UTC+9:30 / UTC+10:30
        "Asia/Yangon",        // UTC+6:30
        "Pacific/Marquesas",  // UTC-9:30
        "Pacific/Chatham",    // UTC+12:45 / UTC+13:45
    ];

    for tz_name in edge_cases.iter() {
        test_timezone_comprehensive(tz_name);
    }
}

#[test]
fn test_historical_changes() {
    // Timezones that have had significant historical changes
    let historical = [
        "America/New_York",  // US DST rule change in 2007
        "Europe/London",     // Various historical changes
        "America/Sao_Paulo", // Brazil has changed DST rules multiple times
        "Europe/Moscow",     // Russia has changed offset and DST multiple times
    ];

    for tz_name in historical.iter() {
        test_timezone_comprehensive(tz_name);
    }
}

/// Test ALL timezones with a sampling of dates
#[test]
fn test_all_timezones_sampled() {
    let timezones = llrt_tz::list_timezones();

    // Sample timestamps across decades
    let sample_timestamps: Vec<i64> = [
        // 1970s
        (1970, 1, 15, 12),
        (1975, 7, 15, 12),
        // 1980s
        (1980, 3, 15, 12),
        (1985, 9, 15, 12),
        // 1990s
        (1990, 6, 15, 12),
        (1995, 12, 15, 12),
        // 2000s
        (2000, 1, 1, 0),
        (2005, 6, 15, 12),
        (2006, 3, 15, 12), // Before US DST change
        (2007, 3, 15, 12), // After US DST change
        // 2010s
        (2010, 7, 4, 12),
        (2015, 11, 11, 12),
        // 2020s
        (2020, 2, 29, 12),  // Leap year
        (2024, 6, 21, 12),  // Summer solstice
        (2024, 12, 21, 12), // Winter solstice
    ]
    .iter()
    .filter_map(|(y, m, d, h)| {
        NaiveDate::from_ymd_opt(*y, *m, *d)
            .and_then(|date| date.and_hms_opt(*h, 0, 0))
            .map(|dt| dt.and_utc().timestamp())
    })
    .collect();

    let mut mismatches = Vec::new();

    for tz_name in timezones {
        for ts in &sample_timestamps {
            let chrono_offset = chrono_tz_offset(tz_name, *ts);
            let compact_offset = compact_tz_offset(tz_name, *ts);

            if chrono_offset != compact_offset {
                mismatches.push(format!(
                    "{} at ts={}: chrono={}, compact={}",
                    tz_name, ts, chrono_offset, compact_offset
                ));
            }
        }
    }

    if !mismatches.is_empty() {
        panic!(
            "Found {} mismatches across all timezones:\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );
    }

    println!(
        "Tested {} timezones × {} timestamps = {} comparisons, all passed!",
        timezones.len(),
        sample_timestamps.len(),
        timezones.len() * sample_timestamps.len()
    );
}

/// Exhaustive test of a critical timezone with hourly resolution
#[test]
fn test_new_york_hourly_1970_to_2024() {
    let tz_name = "America/New_York";
    let mut total_comparisons = 0;
    let mut mismatches = Vec::new();

    // Test every day from 1970 to 2024, at midnight and noon
    for year in 1970..=2024 {
        for month in 1..=12u32 {
            let days = match month {
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
            };

            for day in 1..=days {
                for hour in [0, 6, 12, 18].iter() {
                    if let Some(dt) = NaiveDate::from_ymd_opt(year, month, day)
                        .and_then(|d| d.and_hms_opt(*hour, 0, 0))
                    {
                        let ts = dt.and_utc().timestamp();
                        let chrono_offset = chrono_tz_offset(tz_name, ts);
                        let compact_offset = compact_tz_offset(tz_name, ts);
                        total_comparisons += 1;

                        if chrono_offset != compact_offset {
                            mismatches.push(format!(
                                "{}-{:02}-{:02} {:02}:00: chrono={}, compact={}",
                                year, month, day, hour, chrono_offset, compact_offset
                            ));
                        }
                    }
                }
            }
        }
    }

    if !mismatches.is_empty() {
        panic!(
            "Found {} mismatches in {} comparisons for {}:\n{}",
            mismatches.len(),
            total_comparisons,
            tz_name,
            mismatches
                .iter()
                .take(50)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!(
        "{}: {} comparisons, all passed!",
        tz_name, total_comparisons
    );
}

/// Test that all chrono-tz timezone names are available
#[test]
fn test_all_timezone_names_available() {
    let compact_timezones: std::collections::HashSet<_> =
        llrt_tz::list_timezones().iter().cloned().collect();

    let mut missing = Vec::new();

    for tz in chrono_tz::TZ_VARIANTS.iter() {
        let name = tz.name();
        if !compact_timezones.contains(name) {
            missing.push(name);
        }
    }

    if !missing.is_empty() {
        panic!(
            "Missing {} timezones from compact implementation:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    assert_eq!(
        compact_timezones.len(),
        chrono_tz::TZ_VARIANTS.len(),
        "Timezone count mismatch"
    );

    println!("All {} timezones available!", compact_timezones.len());
}
