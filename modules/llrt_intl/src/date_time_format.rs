// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Minimal Intl.DateTimeFormat implementation for timezone support.
//! This provides just enough functionality to support dayjs and similar libraries.

use jiff::{tz::TimeZone, Timestamp, Zoned};
use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Coerced, Ctx, Exception, Object, Result, Value,
};

/// Stores the resolved options for a DateTimeFormat instance
#[derive(Clone, Debug)]
pub struct DateTimeFormatOptions {
    pub locale: String,
    pub timezone: TimeZone,
    pub hour12: bool,
    pub year: Option<String>,
    pub month: Option<String>,
    pub day: Option<String>,
    pub hour: Option<String>,
    pub minute: Option<String>,
    pub second: Option<String>,
    pub weekday: Option<String>,
    pub timezone_name: Option<String>,
    pub fractional_second_digits: Option<u8>,
}

impl Default for DateTimeFormatOptions {
    fn default() -> Self {
        Self {
            locale: "en-US".to_string(),
            timezone: TimeZone::UTC,
            hour12: false,
            year: None,
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            weekday: None,
            timezone_name: None,
            fractional_second_digits: None,
        }
    }
}

/// A formatted part with type and value
#[derive(Debug, Clone)]
pub struct FormatPart {
    pub part_type: &'static str,
    pub value: String,
}

impl FormatPart {
    #[inline]
    fn new(part_type: &'static str, value: String) -> Self {
        Self { part_type, value }
    }

    #[inline]
    fn literal(value: &'static str) -> Self {
        Self {
            part_type: "literal",
            value: value.to_string(),
        }
    }
}

/// Format a number with optional zero-padding
#[inline]
fn format_number(value: i16, two_digit: bool) -> String {
    let mut buf = itoa::Buffer::new();
    if two_digit && value < 10 {
        let mut result = String::with_capacity(2);
        result.push('0');
        result.push_str(buf.format(value));
        result
    } else {
        buf.format(value).to_string()
    }
}

/// Format a number component based on option style
#[inline]
fn format_component(value: i16, style: Option<&str>) -> String {
    let two_digit = matches!(style, Some("2-digit"));
    format_number(value, two_digit)
}

/// Build format parts from a Zoned datetime in pure Rust
fn build_format_parts(local_dt: &Zoned, options: &DateTimeFormatOptions) -> Vec<FormatPart> {
    let mut parts = Vec::with_capacity(16);

    // Month
    if let Some(ref month_opt) = options.month {
        parts.push(FormatPart::new(
            "month",
            format_component(local_dt.month().into(), Some(month_opt)),
        ));
        parts.push(FormatPart::literal("/"));
    }

    // Day
    if let Some(ref day_opt) = options.day {
        parts.push(FormatPart::new(
            "day",
            format_component(local_dt.day().into(), Some(day_opt)),
        ));
        parts.push(FormatPart::literal("/"));
    }

    // Year
    if let Some(ref year_opt) = options.year {
        let year_val = if year_opt == "2-digit" {
            format_number(local_dt.year() % 100, true)
        } else {
            let mut buf = itoa::Buffer::new();
            buf.format(local_dt.year()).to_string()
        };
        parts.push(FormatPart::new("year", year_val));
    }

    // Add separator between date and time if we have both
    let has_date = options.year.is_some() || options.month.is_some() || options.day.is_some();
    let has_time = options.hour.is_some() || options.minute.is_some() || options.second.is_some();

    if has_date && has_time {
        parts.push(FormatPart::literal(", "));
    }

    // Hour
    if options.hour.is_some() {
        let hour = local_dt.hour();
        let hour_val = if options.hour12 {
            match hour {
                0 => 12,
                13..=23 => hour - 12,
                _ => hour,
            }
        } else {
            hour
        };

        parts.push(FormatPart::new(
            "hour",
            format_component(hour_val.into(), options.hour.as_deref()),
        ));

        if options.minute.is_some() || options.second.is_some() {
            parts.push(FormatPart::literal(":"));
        }
    }

    // Minute
    if options.minute.is_some() {
        parts.push(FormatPart::new(
            "minute",
            format_component(local_dt.minute().into(), options.minute.as_deref()),
        ));

        if options.second.is_some() {
            parts.push(FormatPart::literal(":"));
        }
    }

    // Second
    if options.second.is_some() {
        parts.push(FormatPart::new(
            "second",
            format_component(local_dt.second().into(), options.second.as_deref()),
        ));
    }

    // dayPeriod for 12-hour format
    if options.hour12 && options.hour.is_some() {
        let hour = local_dt.hour();
        parts.push(FormatPart::literal(" "));
        parts.push(FormatPart::new(
            "dayPeriod",
            if hour >= 12 { "PM" } else { "AM" }.to_string(),
        ));
    }

    // Timezone name
    if let Some(ref tz_name_opt) = options.timezone_name {
        parts.push(FormatPart::literal(" "));
        let tz_str = format_timezone_name(local_dt, &options.timezone, tz_name_opt);
        parts.push(FormatPart::new("timeZoneName", tz_str));
    }

    parts
}

/// Format timezone name based on style option
fn format_timezone_name(local_dt: &Zoned, timezone: &TimeZone, style: &str) -> String {
    match style {
        "short" | "shortOffset" => {
            let offset = local_dt.offset();
            let total_secs = offset.seconds();
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600).abs() / 60;

            let mut result = String::with_capacity(10);
            result.push_str("GMT");

            if hours >= 0 {
                result.push('+');
            }

            let mut buf = itoa::Buffer::new();
            result.push_str(buf.format(hours));

            if mins != 0 && style == "shortOffset" {
                result.push(':');
                if mins < 10 {
                    result.push('0');
                }
                result.push_str(buf.format(mins));
            }

            result
        },
        "long" => timezone.iana_name().unwrap_or_default().into(),
        _ => timezone.iana_name().unwrap_or_default().into(),
    }
}

/// Convert format parts Vec to JS Array
fn parts_to_js_array<'js>(ctx: &Ctx<'js>, parts: Vec<FormatPart>) -> Result<Array<'js>> {
    let array = Array::new(ctx.clone())?;
    for (idx, part) in parts.into_iter().enumerate() {
        let obj = Object::new(ctx.clone())?;
        obj.set("type", part.part_type)?;
        obj.set("value", part.value)?;
        array.set(idx, obj)?;
    }
    Ok(array)
}

/// Join format parts into a single string
fn parts_to_string(parts: &[FormatPart]) -> String {
    let total_len: usize = parts.iter().map(|p| p.value.len()).sum();
    let mut result = String::with_capacity(total_len);
    for part in parts {
        result.push_str(&part.value);
    }
    result
}

/// Parse epoch milliseconds from a Date value
fn parse_epoch_ms<'js>(ctx: &Ctx<'js>, date: Opt<Value<'js>>) -> Result<f64> {
    if let Some(date_val) = date.into_inner() {
        if date_val.is_undefined() {
            Ok(Timestamp::now().as_millisecond() as f64)
        } else if let Some(num) = date_val.as_number() {
            Ok(num)
        } else {
            date_val
                .get::<Coerced<f64>>()
                .map(|c| c.0)
                .map_err(|_| Exception::throw_type(ctx, "Invalid date"))
        }
    } else {
        Ok(Timestamp::now().as_millisecond() as f64)
    }
}

/// Convert epoch milliseconds to Zoned datetime in the specified timezone
fn epoch_to_datetime(ctx: &Ctx<'_>, epoch_ms: f64, timezone: &TimeZone) -> Result<Zoned> {
    let utc_dt = Timestamp::from_millisecond(epoch_ms as i64)
        .map_err(|_| Exception::throw_range(ctx, "Invalid timestamp"))?;
    Ok(utc_dt.to_zoned(timezone.clone()))
}

/// Minimal Intl.DateTimeFormat implementation
#[derive(Clone, rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct DateTimeFormat {
    #[qjs(skip_trace)]
    options: DateTimeFormatOptions,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl DateTimeFormat {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, locales: Opt<Value<'_>>, options: Opt<Object<'_>>) -> Result<Self> {
        let mut opts = DateTimeFormatOptions::default();

        // Parse locale (we only care about extracting the language tag)
        if let Some(locale_val) = locales.into_inner() {
            if let Some(s) = locale_val.as_string() {
                opts.locale = s.to_string()?;
            } else if let Some(arr) = locale_val.as_array() {
                if let Ok(first) = arr.get::<Value>(0) {
                    if let Some(s) = first.as_string() {
                        opts.locale = s.to_string()?;
                    }
                }
            }
        }

        // Parse options
        if let Some(options_obj) = options.into_inner() {
            // timeZone
            if let Ok(tz_val) = options_obj.get::<_, String>("timeZone") {
                opts.timezone = TimeZone::get(&tz_val).map_err(|_| {
                    Exception::throw_range(&ctx, &["Invalid time zone: ", &tz_val].concat())
                })?;
            }

            // hour12
            if let Ok(h12) = options_obj.get::<_, bool>("hour12") {
                opts.hour12 = h12;
            }

            // Date/time components
            if let Ok(v) = options_obj.get::<_, String>("year") {
                opts.year = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, String>("month") {
                opts.month = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, String>("day") {
                opts.day = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, String>("hour") {
                opts.hour = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, String>("minute") {
                opts.minute = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, String>("second") {
                opts.second = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, String>("weekday") {
                opts.weekday = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, String>("timeZoneName") {
                opts.timezone_name = Some(v);
            }
            if let Ok(v) = options_obj.get::<_, u8>("fractionalSecondDigits") {
                opts.fractional_second_digits = Some(v);
            }
        }

        Ok(Self { options: opts })
    }

    /// Format a date according to the locale and options
    pub fn format<'js>(&self, ctx: Ctx<'js>, date: Opt<Value<'js>>) -> Result<String> {
        let epoch_ms = parse_epoch_ms(&ctx, date)?;
        let local_dt = epoch_to_datetime(&ctx, epoch_ms, &self.options.timezone)?;
        let parts = build_format_parts(&local_dt, &self.options);
        Ok(parts_to_string(&parts))
    }

    /// Format a date to parts
    #[qjs(rename = "formatToParts")]
    pub fn format_to_parts<'js>(&self, ctx: Ctx<'js>, date: Opt<Value<'js>>) -> Result<Array<'js>> {
        let epoch_ms = parse_epoch_ms(&ctx, date)?;
        let local_dt = epoch_to_datetime(&ctx, epoch_ms, &self.options.timezone)?;
        let parts = build_format_parts(&local_dt, &self.options);
        parts_to_js_array(&ctx, parts)
    }

    /// Return resolved options
    #[qjs(rename = "resolvedOptions")]
    pub fn resolved_options<'js>(&self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;

        obj.set("locale", self.options.locale.as_str())?;
        obj.set("calendar", "gregory")?;
        obj.set("numberingSystem", "latn")?;
        obj.set(
            "timeZone",
            self.options.timezone.iana_name().unwrap_or_default(),
        )?;

        if self.options.hour.is_some() {
            obj.set("hour12", self.options.hour12)?;
            obj.set("hourCycle", if self.options.hour12 { "h12" } else { "h23" })?;
        }

        if let Some(ref v) = self.options.year {
            obj.set("year", v.as_str())?;
        }
        if let Some(ref v) = self.options.month {
            obj.set("month", v.as_str())?;
        }
        if let Some(ref v) = self.options.day {
            obj.set("day", v.as_str())?;
        }
        if let Some(ref v) = self.options.hour {
            obj.set("hour", v.as_str())?;
        }
        if let Some(ref v) = self.options.minute {
            obj.set("minute", v.as_str())?;
        }
        if let Some(ref v) = self.options.second {
            obj.set("second", v.as_str())?;
        }
        if let Some(ref v) = self.options.weekday {
            obj.set("weekday", v.as_str())?;
        }
        if let Some(ref v) = self.options.timezone_name {
            obj.set("timeZoneName", v.as_str())?;
        }

        Ok(obj)
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        "Intl.DateTimeFormat"
    }
}

/// Format a date in the specified timezone using locale options.
/// This is used to implement Date.prototype.toLocaleString with timezone support.
pub fn format_date_in_timezone(
    epoch_ms: f64,
    timezone: &TimeZone,
    options: &ToLocaleStringOptions,
) -> String {
    let utc_dt = Timestamp::from_millisecond(epoch_ms as i64).unwrap();
    let local_dt = utc_dt.to_zoned(timezone.clone());

    // Format as MM/DD/YYYY, HH:MM:SS AM/PM (en-US style)
    let month = local_dt.month();
    let day = local_dt.day();
    let year = local_dt.year();
    let hour = local_dt.hour();
    let minute = local_dt.minute();
    let second = local_dt.second();

    let mut buf = itoa::Buffer::new();
    let mut result = String::with_capacity(24);

    // Month (zero-padded)
    if month < 10 {
        result.push('0');
    }
    result.push_str(buf.format(month));
    result.push('/');

    // Day (zero-padded)
    if day < 10 {
        result.push('0');
    }
    result.push_str(buf.format(day));
    result.push('/');

    // Year
    result.push_str(buf.format(year));
    result.push_str(", ");

    if options.hour12 {
        let (hour12, period) = match hour {
            0 => (12, "AM"),
            1..=11 => (hour, "AM"),
            12 => (12, "PM"),
            _ => (hour - 12, "PM"),
        };

        // Hour (no padding for 12-hour format)
        result.push_str(buf.format(hour12));
        result.push(':');

        // Minute (zero-padded)
        if minute < 10 {
            result.push('0');
        }
        result.push_str(buf.format(minute));
        result.push(':');

        // Second (zero-padded)
        if second < 10 {
            result.push('0');
        }
        result.push_str(buf.format(second));

        result.push(' ');
        result.push_str(period);
    } else {
        // Hour (zero-padded)
        if hour < 10 {
            result.push('0');
        }
        result.push_str(buf.format(hour));
        result.push(':');

        // Minute (zero-padded)
        if minute < 10 {
            result.push('0');
        }
        result.push_str(buf.format(minute));
        result.push(':');

        // Second (zero-padded)
        if second < 10 {
            result.push('0');
        }
        result.push_str(buf.format(second));
    }

    result
}

/// Options for toLocaleString
#[derive(Default)]
pub struct ToLocaleStringOptions {
    pub hour12: bool,
    pub hour12_set: bool,
    pub date_style: Option<String>,
    pub time_style: Option<String>,
}

/// Parse toLocaleString options from a JavaScript object
pub fn parse_to_locale_string_options<'js>(
    ctx: &Ctx<'js>,
    options: Option<Object<'js>>,
) -> Result<(Option<TimeZone>, ToLocaleStringOptions)> {
    let mut tz: Option<TimeZone> = None;
    let mut opts = ToLocaleStringOptions::default();

    if let Some(options_obj) = options {
        // Parse timeZone
        if let Ok(tz_val) = options_obj.get::<_, String>("timeZone") {
            tz = Some(TimeZone::get(&tz_val).map_err(|_| {
                Exception::throw_range(ctx, &["Invalid time zone: ", &tz_val].concat())
            })?);
        }

        // Parse hour12
        if let Ok(h12) = options_obj.get::<_, bool>("hour12") {
            opts.hour12 = h12;
            opts.hour12_set = true;
        }

        // Parse dateStyle
        if let Ok(ds) = options_obj.get::<_, String>("dateStyle") {
            opts.date_style = Some(ds);
        }

        // Parse timeStyle
        if let Ok(ts) = options_obj.get::<_, String>("timeStyle") {
            opts.time_style = Some(ts);
        }
    }

    Ok((tz, opts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::tz::TimeZone;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(5, false), "5");
        assert_eq!(format_number(5, true), "05");
        assert_eq!(format_number(12, true), "12");
        assert_eq!(format_number(0, true), "00");
    }

    #[test]
    fn test_format_component() {
        assert_eq!(format_component(5, Some("2-digit")), "05");
        assert_eq!(format_component(5, Some("numeric")), "5");
        assert_eq!(format_component(12, Some("2-digit")), "12");
        assert_eq!(format_component(12, None), "12");
    }

    #[test]
    fn test_build_format_parts_date_only() {
        let tz = TimeZone::get("UTC").unwrap();
        // 2024-03-15 10:30:45 UTC
        let epoch_ms = 1710499845000.0;
        let utc_dt = Timestamp::from_millisecond(epoch_ms as i64).unwrap();
        let local_dt = utc_dt.to_zoned(tz);

        let options = DateTimeFormatOptions {
            year: Some("numeric".to_string()),
            month: Some("2-digit".to_string()),
            day: Some("2-digit".to_string()),
            ..Default::default()
        };

        let parts = build_format_parts(&local_dt, &options);

        assert_eq!(parts.len(), 5); // month, /, day, /, year
        assert_eq!(parts[0].part_type, "month");
        assert_eq!(parts[0].value, "03");
        assert_eq!(parts[1].part_type, "literal");
        assert_eq!(parts[1].value, "/");
        assert_eq!(parts[2].part_type, "day");
        assert_eq!(parts[2].value, "15");
        assert_eq!(parts[4].part_type, "year");
        assert_eq!(parts[4].value, "2024");
    }

    #[test]
    fn test_build_format_parts_time_only() {
        let tz = TimeZone::get("UTC").unwrap();
        // 2024-03-15 10:30:45 UTC
        let epoch_ms = 1710498645000.0;
        let utc_dt = Timestamp::from_millisecond(epoch_ms as i64).unwrap();
        let local_dt = utc_dt.to_zoned(tz);

        let options = DateTimeFormatOptions {
            hour: Some("2-digit".to_string()),
            minute: Some("2-digit".to_string()),
            second: Some("2-digit".to_string()),
            ..Default::default()
        };

        let parts = build_format_parts(&local_dt, &options);

        assert_eq!(parts[0].part_type, "hour");
        assert_eq!(parts[0].value, "10");
        assert_eq!(parts[2].part_type, "minute");
        assert_eq!(parts[2].value, "30");
        assert_eq!(parts[4].part_type, "second");
        assert_eq!(parts[4].value, "45");
    }

    #[test]
    fn test_build_format_parts_12hour() {
        let tz = TimeZone::get("UTC").unwrap();
        // 2024-03-15 14:30:00 UTC (2 PM)
        let epoch_ms = 1710514200000.0;
        let utc_dt = Timestamp::from_millisecond(epoch_ms as i64).unwrap();
        let local_dt = utc_dt.to_zoned(tz);

        let options = DateTimeFormatOptions {
            hour: Some("numeric".to_string()),
            hour12: true,
            ..Default::default()
        };

        let parts = build_format_parts(&local_dt, &options);

        assert_eq!(parts[0].part_type, "hour");
        assert_eq!(parts[0].value, "2"); // 14:00 -> 2 PM
        assert_eq!(parts[2].part_type, "dayPeriod");
        assert_eq!(parts[2].value, "PM");
    }

    #[test]
    fn test_build_format_parts_midnight_12hour() {
        let tz = TimeZone::get("UTC").unwrap();
        // 2024-03-15 00:30:00 UTC (12:30 AM)
        let epoch_ms = 1710463800000.0;
        let utc_dt = Timestamp::from_millisecond(epoch_ms as i64).unwrap();
        let local_dt = utc_dt.to_zoned(tz);

        let options = DateTimeFormatOptions {
            hour: Some("numeric".to_string()),
            hour12: true,
            ..Default::default()
        };

        let parts = build_format_parts(&local_dt, &options);

        assert_eq!(parts[0].part_type, "hour");
        assert_eq!(parts[0].value, "12"); // 00:00 -> 12 AM
        assert_eq!(parts[2].part_type, "dayPeriod");
        assert_eq!(parts[2].value, "AM");
    }

    #[test]
    fn test_format_timezone_name_short() {
        let tz = TimeZone::get("America/New_York").unwrap();
        // Summer time (EDT = UTC-4)
        let epoch_ms = 1720000000000.0; // July 2024
        let utc_dt = Timestamp::from_millisecond(epoch_ms as i64).unwrap();
        let local_dt = utc_dt.to_zoned(tz.clone());

        let result = format_timezone_name(&local_dt, &tz, "short");
        assert_eq!(result, "GMT-4");
    }

    #[test]
    fn test_format_timezone_name_long() {
        let tz = TimeZone::get("America/New_York").unwrap();
        let epoch_ms = 1720000000000.0;
        let utc_dt = Timestamp::from_millisecond(epoch_ms as i64).unwrap();
        let local_dt = utc_dt.to_zoned(tz.clone());

        let result = format_timezone_name(&local_dt, &tz, "long");
        assert_eq!(result, "America/New_York");
    }

    #[test]
    fn test_parts_to_string() {
        let parts = vec![
            FormatPart::new("month", "03".to_string()),
            FormatPart::literal("/"),
            FormatPart::new("day", "15".to_string()),
            FormatPart::literal("/"),
            FormatPart::new("year", "2024".to_string()),
        ];

        assert_eq!(parts_to_string(&parts), "03/15/2024");
    }

    #[test]
    fn test_format_date_in_timezone() {
        // 2024-03-15 14:30:45 UTC
        let epoch_ms = 1710513045000.0;
        let tz = TimeZone::get("UTC").unwrap();

        let opts = ToLocaleStringOptions {
            hour12: true,
            ..Default::default()
        };
        let result = format_date_in_timezone(epoch_ms, &tz, &opts);
        assert_eq!(result, "03/15/2024, 2:30:45 PM");

        let opts = ToLocaleStringOptions {
            hour12: false,
            ..Default::default()
        };
        let result = format_date_in_timezone(epoch_ms, &tz, &opts);
        assert_eq!(result, "03/15/2024, 14:30:45");
    }

    #[test]
    fn test_format_date_in_timezone_with_tz() {
        // 2024-03-15 14:30:45 UTC -> 10:30:45 AM EDT (UTC-4)
        let epoch_ms = 1710513045000.0;
        let tz = TimeZone::get("America/New_York").unwrap();

        let opts = ToLocaleStringOptions {
            hour12: true,
            ..Default::default()
        };
        let result = format_date_in_timezone(epoch_ms, &tz, &opts);
        assert_eq!(result, "03/15/2024, 10:30:45 AM");
    }

    #[test]
    fn test_format_date_midnight() {
        // 2024-03-15 00:00:00 UTC
        let epoch_ms = 1710460800000.0;
        let tz = TimeZone::get("UTC").unwrap();

        let opts = ToLocaleStringOptions {
            hour12: true,
            ..Default::default()
        };
        let result = format_date_in_timezone(epoch_ms, &tz, &opts);
        assert_eq!(result, "03/15/2024, 12:00:00 AM");

        let opts = ToLocaleStringOptions {
            hour12: false,
            ..Default::default()
        };
        let result = format_date_in_timezone(epoch_ms, &tz, &opts);
        assert_eq!(result, "03/15/2024, 00:00:00");
    }

    #[test]
    fn test_format_date_noon() {
        // 2024-03-15 12:00:00 UTC
        let epoch_ms = 1710504000000.0;
        let tz = TimeZone::get("UTC").unwrap();

        let opts = ToLocaleStringOptions {
            hour12: true,
            ..Default::default()
        };
        let result = format_date_in_timezone(epoch_ms, &tz, &opts);
        assert_eq!(result, "03/15/2024, 12:00:00 PM");
    }
}
