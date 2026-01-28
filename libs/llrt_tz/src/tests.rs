// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::compact::{get_offset, list_timezones, lookup_timezone};

#[test]
fn test_utc_offset() {
    // UTC should always be 0
    let offset = get_offset("UTC", 0.0).unwrap();
    assert_eq!(offset, 0);

    let offset = get_offset("UTC", 1700000000000.0).unwrap();
    assert_eq!(offset, 0);
}

#[test]
fn test_fixed_offset_timezone() {
    // Etc/GMT+5 is UTC-5 (note: Etc/GMT signs are inverted)
    // In January (no DST anywhere for fixed offsets)
    let jan_2024 = 1704067200000.0; // 2024-01-01 00:00:00 UTC
    let offset = get_offset("Etc/GMT+5", jan_2024).unwrap();
    assert_eq!(offset, -300); // -5 hours = -300 minutes
}

#[test]
fn test_dst_timezone_summer() {
    // America/New_York in summer (EDT = UTC-4)
    let jul_2024 = 1720000000000.0; // July 3, 2024
    let offset = get_offset("America/New_York", jul_2024).unwrap();
    assert_eq!(offset, -240); // -4 hours during DST
}

#[test]
fn test_dst_timezone_winter() {
    // America/New_York in winter (EST = UTC-5)
    let jan_2024 = 1704067200000.0; // January 1, 2024
    let offset = get_offset("America/New_York", jan_2024).unwrap();
    assert_eq!(offset, -300); // -5 hours during standard time
}

#[test]
fn test_lookup_timezone() {
    let tz = lookup_timezone("America/New_York").unwrap();
    assert_eq!(tz.name, "America/New_York");
    assert_eq!(tz.std_offset, -300); // EST = UTC-5

    let tz = lookup_timezone("Europe/London").unwrap();
    assert_eq!(tz.name, "Europe/London");
    assert_eq!(tz.std_offset, 0); // GMT

    let tz = lookup_timezone("Asia/Tokyo").unwrap();
    assert_eq!(tz.name, "Asia/Tokyo");
    assert_eq!(tz.std_offset, 540); // JST = UTC+9
    assert!(tz.dst_rule.is_none()); // Japan doesn't observe DST
}

#[test]
fn test_invalid_timezone() {
    let result = get_offset("Invalid/Timezone", 0.0);
    assert!(result.is_none());
}

#[test]
fn test_list_timezones() {
    let zones = list_timezones();
    assert!(!zones.is_empty());
    assert!(zones.contains(&"UTC"));
    assert!(zones.contains(&"America/New_York"));
    assert!(zones.contains(&"Europe/London"));
    assert!(zones.contains(&"Asia/Tokyo"));
}

#[test]
fn test_timezones_sorted() {
    let zones = list_timezones();
    let mut sorted = zones.to_vec();
    sorted.sort();
    assert_eq!(zones, sorted.as_slice());
}

// Comparison tests with chrono-tz
#[cfg(test)]
mod chrono_comparison {
    use super::*;
    use chrono::{Offset, TimeZone, Utc};
    use chrono_tz::Tz;

    fn chrono_offset(tz_name: &str, epoch_ms: f64) -> i16 {
        let tz: Tz = tz_name.parse().unwrap();
        let epoch_secs = (epoch_ms / 1000.0) as i64;
        let utc = Utc.timestamp_opt(epoch_secs, 0).unwrap();
        let local = utc.with_timezone(&tz);
        (local.offset().fix().local_minus_utc() / 60) as i16
    }

    #[test]
    fn test_matches_chrono_tz_recent_dates() {
        let test_cases = [
            ("UTC", 1700000000000.0),
            ("America/New_York", 1704067200000.0), // Jan 2024 (EST)
            ("America/New_York", 1720000000000.0), // Jul 2024 (EDT)
            ("America/Los_Angeles", 1704067200000.0), // Jan 2024 (PST)
            ("America/Los_Angeles", 1720000000000.0), // Jul 2024 (PDT)
            ("Europe/London", 1704067200000.0),    // Jan 2024 (GMT)
            ("Europe/London", 1720000000000.0),    // Jul 2024 (BST)
            ("Europe/Paris", 1704067200000.0),     // Jan 2024 (CET)
            ("Europe/Paris", 1720000000000.0),     // Jul 2024 (CEST)
            ("Asia/Tokyo", 1704067200000.0),       // Jan 2024
            ("Asia/Tokyo", 1720000000000.0),       // Jul 2024
            ("Australia/Sydney", 1704067200000.0), // Jan 2024 (AEDT)
            ("Australia/Sydney", 1720000000000.0), // Jul 2024 (AEST)
        ];

        for (tz_name, epoch_ms) in test_cases {
            let our_offset = get_offset(tz_name, epoch_ms).unwrap();
            let chrono_offset = chrono_offset(tz_name, epoch_ms);
            assert_eq!(
                our_offset, chrono_offset,
                "Mismatch for {} at {}: ours={}, chrono={}",
                tz_name, epoch_ms, our_offset, chrono_offset
            );
        }
    }
}
