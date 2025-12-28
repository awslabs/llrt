// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! CLDR pattern parser and formatter.
//!
//! Parses Unicode CLDR date/time patterns and formats dates accordingly.
//! Pattern syntax follows Unicode Technical Standard #35:
//! https://unicode.org/reports/tr35/tr35-dates.html#Date_Field_Symbol_Table

use crate::cldr_data::LocaleData;
use chrono::{DateTime, Datelike, Offset, Timelike};
use llrt_tz::Tz;

/// Format a DateTime using a CLDR pattern string
pub fn format_with_pattern(
    dt: &DateTime<Tz>,
    pattern: &str,
    locale_data: &LocaleData,
    hour12_override: Option<bool>,
) -> String {
    let mut result = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Quoted literal text
            '\'' => {
                // Check for escaped quote ('')
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    result.push('\'');
                } else {
                    // Collect until closing quote
                    for c in chars.by_ref() {
                        if c == '\'' {
                            break;
                        }
                        result.push(c);
                    }
                }
            },
            // Pattern letters
            'y' | 'Y' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_year(&mut result, dt.year(), count);
            },
            'M' | 'L' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_month(&mut result, dt.month() as usize, count, locale_data);
            },
            'd' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_day(&mut result, dt.day(), count);
            },
            'E' | 'e' | 'c' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_weekday(
                    &mut result,
                    dt.weekday().num_days_from_sunday() as usize,
                    count,
                    locale_data,
                );
            },
            'a' => {
                consume_same(&mut chars, ch);
                // Only show AM/PM if we're using 12-hour format
                let use_12h = hour12_override.unwrap_or(true);
                if use_12h {
                    let hour = dt.hour();
                    if hour < 12 {
                        result.push_str(locale_data.am);
                    } else {
                        result.push_str(locale_data.pm);
                    }
                }
            },
            'h' => {
                let count = 1 + consume_same(&mut chars, ch);
                let hour = dt.hour();
                // If hour12 is explicitly false, use 24-hour format instead
                let use_12h = hour12_override.unwrap_or(true);
                if use_12h {
                    // 12-hour format (1-12)
                    let hour12 = match hour {
                        0 => 12,
                        1..=12 => hour,
                        _ => hour - 12,
                    };
                    format_number(&mut result, hour12, count);
                } else {
                    // 24-hour format (0-23)
                    format_number(&mut result, hour, count);
                }
            },
            'H' => {
                let count = 1 + consume_same(&mut chars, ch);
                let hour = dt.hour();
                // If hour12 is explicitly true, use 12-hour format instead
                let use_12h = hour12_override.unwrap_or(false);
                if use_12h {
                    let hour12 = match hour {
                        0 => 12,
                        1..=12 => hour,
                        _ => hour - 12,
                    };
                    format_number(&mut result, hour12, count);
                } else {
                    // 24-hour format (0-23)
                    format_number(&mut result, hour, count);
                }
            },
            'm' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_number(&mut result, dt.minute(), count);
            },
            's' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_number(&mut result, dt.second(), count);
            },
            'z' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_timezone(&mut result, dt, count);
            },
            'Z' | 'O' | 'v' | 'V' | 'X' | 'x' => {
                let count = 1 + consume_same(&mut chars, ch);
                format_timezone(&mut result, dt, count);
            },
            // Skip these pattern letters (not commonly needed)
            'G' | 'q' | 'Q' | 'w' | 'W' | 'D' | 'F' | 'g' | 'A' | 'S' => {
                consume_same(&mut chars, ch);
            },
            // Pass through literal characters
            _ => {
                result.push(ch);
            },
        }
    }

    // If hour12=true and pattern originally used 24h format (H) without AM/PM marker,
    // we need to append AM/PM since we converted to 12h format
    if let Some(true) = hour12_override {
        if !pattern.contains('a') && pattern.contains('H') {
            result.push(' ');
            if dt.hour() < 12 {
                result.push_str(locale_data.am);
            } else {
                result.push_str(locale_data.pm);
            }
        }
    }

    result
}

/// Consume consecutive identical characters, returning count of additional chars
fn consume_same(chars: &mut std::iter::Peekable<std::str::Chars>, ch: char) -> usize {
    let mut count = 0;
    while chars.peek() == Some(&ch) {
        chars.next();
        count += 1;
    }
    count
}

/// Format year based on pattern width
fn format_year(result: &mut String, year: i32, width: usize) {
    let mut buf = itoa::Buffer::new();
    if width == 2 {
        // 2-digit year
        let short_year = (year % 100).unsigned_abs();
        if short_year < 10 {
            result.push('0');
        }
        result.push_str(buf.format(short_year));
    } else {
        result.push_str(buf.format(year));
    }
}

/// Format month based on pattern width
fn format_month(result: &mut String, month: usize, width: usize, locale_data: &LocaleData) {
    match width {
        1 => {
            // Numeric, no padding
            let mut buf = itoa::Buffer::new();
            result.push_str(buf.format(month));
        },
        2 => {
            // Numeric, zero-padded
            let mut buf = itoa::Buffer::new();
            if month < 10 {
                result.push('0');
            }
            result.push_str(buf.format(month));
        },
        3 => {
            // Abbreviated
            if (1..=12).contains(&month) {
                result.push_str(locale_data.months_abbr[month - 1]);
            }
        },
        _ => {
            // Wide (4+)
            if (1..=12).contains(&month) {
                result.push_str(locale_data.months_wide[month - 1]);
            }
        },
    }
}

/// Format day based on pattern width
fn format_day(result: &mut String, day: u32, width: usize) {
    format_number(result, day, width);
}

/// Format weekday based on pattern width
fn format_weekday(result: &mut String, weekday: usize, width: usize, locale_data: &LocaleData) {
    match width {
        1..=3 => {
            // Abbreviated
            result.push_str(locale_data.days_abbr[weekday]);
        },
        _ => {
            // Wide (4+)
            result.push_str(locale_data.days_wide[weekday]);
        },
    }
}

/// Format a number with optional zero-padding
fn format_number(result: &mut String, value: u32, min_width: usize) {
    let mut buf = itoa::Buffer::new();
    let s = buf.format(value);
    if min_width >= 2 && s.len() < min_width {
        for _ in 0..(min_width - s.len()) {
            result.push('0');
        }
    }
    result.push_str(s);
}

/// Format timezone
fn format_timezone(result: &mut String, dt: &DateTime<Tz>, width: usize) {
    let offset = dt.offset().fix();
    let total_secs = offset.local_minus_utc();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600).abs() / 60;

    if width >= 4 {
        // Long form: timezone name
        result.push_str(dt.timezone().name());
    } else {
        // Short form: GMT offset
        result.push_str("GMT");
        if hours >= 0 {
            result.push('+');
        }
        let mut buf = itoa::Buffer::new();
        result.push_str(buf.format(hours));
        if mins != 0 {
            result.push(':');
            if mins < 10 {
                result.push('0');
            }
            result.push_str(buf.format(mins));
        }
    }
}

/// Combine date and time strings using a datetime pattern
pub fn combine_datetime(date: &str, time: &str, pattern: &str) -> String {
    pattern.replace("{1}", date).replace("{0}", time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cldr_data::get_locale_data;
    use chrono::{TimeZone, Utc};

    fn make_dt(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Tz> {
        let tz: Tz = "UTC".parse().unwrap();
        Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
            .unwrap()
            .with_timezone(&tz)
    }

    #[test]
    fn test_format_year() {
        let dt = make_dt(2024, 3, 15, 10, 30, 45);
        let locale = get_locale_data("en-US");

        assert!(format_with_pattern(&dt, "y", locale, None).contains("2024"));
        assert!(format_with_pattern(&dt, "yy", locale, None).contains("24"));
        assert!(format_with_pattern(&dt, "yyyy", locale, None).contains("2024"));
    }

    #[test]
    fn test_format_month() {
        let dt = make_dt(2024, 3, 15, 10, 30, 45);
        let locale = get_locale_data("en-US");

        assert_eq!(format_with_pattern(&dt, "M", locale, None), "3");
        assert_eq!(format_with_pattern(&dt, "MM", locale, None), "03");
        assert_eq!(format_with_pattern(&dt, "MMM", locale, None), "Mar");
        assert_eq!(format_with_pattern(&dt, "MMMM", locale, None), "March");
    }

    #[test]
    fn test_format_day() {
        let dt = make_dt(2024, 3, 5, 10, 30, 45);
        let locale = get_locale_data("en-US");

        assert_eq!(format_with_pattern(&dt, "d", locale, None), "5");
        assert_eq!(format_with_pattern(&dt, "dd", locale, None), "05");
    }

    #[test]
    fn test_format_weekday() {
        // March 15, 2024 is a Friday
        let dt = make_dt(2024, 3, 15, 10, 30, 45);
        let locale = get_locale_data("en-US");

        assert_eq!(format_with_pattern(&dt, "E", locale, None), "Fri");
        assert_eq!(format_with_pattern(&dt, "EEEE", locale, None), "Friday");
    }

    #[test]
    fn test_format_hour_12() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("en-US");

        assert_eq!(format_with_pattern(&dt, "h", locale, None), "2");
        assert_eq!(format_with_pattern(&dt, "hh", locale, None), "02");
    }

    #[test]
    fn test_format_hour_24() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("en-US");

        assert_eq!(format_with_pattern(&dt, "H", locale, None), "14");
        assert_eq!(format_with_pattern(&dt, "HH", locale, None), "14");
    }

    #[test]
    fn test_format_minute_second() {
        let dt = make_dt(2024, 3, 15, 14, 5, 9);
        let locale = get_locale_data("en-US");

        assert_eq!(format_with_pattern(&dt, "m", locale, None), "5");
        assert_eq!(format_with_pattern(&dt, "mm", locale, None), "05");
        assert_eq!(format_with_pattern(&dt, "s", locale, None), "9");
        assert_eq!(format_with_pattern(&dt, "ss", locale, None), "09");
    }

    #[test]
    fn test_format_am_pm() {
        let locale = get_locale_data("en-US");

        let dt_am = make_dt(2024, 3, 15, 10, 30, 45);
        assert_eq!(format_with_pattern(&dt_am, "a", locale, None), "AM");

        let dt_pm = make_dt(2024, 3, 15, 14, 30, 45);
        assert_eq!(format_with_pattern(&dt_pm, "a", locale, None), "PM");
    }

    #[test]
    fn test_format_quoted_literal() {
        let dt = make_dt(2024, 3, 15, 10, 30, 45);
        let locale = get_locale_data("en-US");

        assert_eq!(
            format_with_pattern(&dt, "d 'de' MMMM", locale, None),
            "15 de March"
        );
    }

    #[test]
    fn test_format_full_date_en_us() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("en-US");

        let result = format_with_pattern(&dt, locale.date_formats.full, locale, None);
        assert_eq!(result, "Friday, March 15, 2024");
    }

    #[test]
    fn test_format_full_date_de() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("de-DE");

        let result = format_with_pattern(&dt, locale.date_formats.full, locale, None);
        assert_eq!(result, "Freitag, 15. März 2024");
    }

    #[test]
    fn test_format_short_date_en_us() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("en-US");

        let result = format_with_pattern(&dt, locale.date_formats.short, locale, None);
        assert_eq!(result, "3/15/24");
    }

    #[test]
    fn test_format_short_date_de() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("de-DE");

        let result = format_with_pattern(&dt, locale.date_formats.short, locale, None);
        assert_eq!(result, "15.03.24");
    }

    #[test]
    fn test_format_short_time_en_us() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("en-US");

        let result = format_with_pattern(&dt, locale.time_formats.short, locale, None);
        assert_eq!(result, "2:30 PM");
    }

    #[test]
    fn test_format_short_time_de() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("de-DE");

        let result = format_with_pattern(&dt, locale.time_formats.short, locale, None);
        assert_eq!(result, "14:30");
    }

    #[test]
    fn test_combine_datetime() {
        assert_eq!(
            combine_datetime("3/15/24", "2:30 PM", "{1}, {0}"),
            "3/15/24, 2:30 PM"
        );
        assert_eq!(
            combine_datetime("15.03.24", "14:30", "{1} {0}"),
            "15.03.24 14:30"
        );
    }

    #[test]
    fn test_midnight_12h() {
        let dt = make_dt(2024, 3, 15, 0, 0, 0);
        let locale = get_locale_data("en-US");

        let result = format_with_pattern(&dt, "h:mm a", locale, None);
        assert_eq!(result, "12:00 AM");
    }

    #[test]
    fn test_noon_12h() {
        let dt = make_dt(2024, 3, 15, 12, 0, 0);
        let locale = get_locale_data("en-US");

        let result = format_with_pattern(&dt, "h:mm a", locale, None);
        assert_eq!(result, "12:00 PM");
    }

    #[test]
    fn test_japanese_locale() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("ja-JP");

        let result = format_with_pattern(&dt, locale.date_formats.long, locale, None);
        assert_eq!(result, "2024年3月15日");
    }

    #[test]
    fn test_korean_locale() {
        let dt = make_dt(2024, 3, 15, 14, 30, 45);
        let locale = get_locale_data("ko-KR");

        let result = format_with_pattern(&dt, locale.date_formats.medium, locale, None);
        assert_eq!(result, "2024. 3. 15.");
    }
}
