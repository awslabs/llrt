// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Minimal Intl module for LLRT.
//!
//! Provides a subset of the `Intl` API focused on timezone support,
//! enabling compatibility with libraries like dayjs.

mod cldr_data;
mod date_time_format;
mod pattern_formatter;

pub use date_time_format::{
    format_date_in_timezone, get_system_timezone, parse_to_locale_string_options, DateTimeFormat,
    DateTimeFormatOptions, ToLocaleStringOptions,
};

use chrono::{TimeZone, Utc};
use cldr_data::get_locale_data;
use llrt_tz::Tz;
use pattern_formatter::{combine_datetime, format_with_pattern};
use rquickjs::{
    function::{Constructor, Opt, This},
    prelude::Func,
    Class, Coerced, Ctx, Exception, Object, Result, Value,
};

/// Initialize the Intl global object with DateTimeFormat
pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    // Create Intl object
    let intl = Object::new(ctx.clone())?;

    // Add DateTimeFormat constructor
    Class::<DateTimeFormat>::define(&intl)?;

    // Set Intl global
    globals.set("Intl", intl)?;

    // Patch Date.prototype.toLocaleString to support timeZone option
    patch_date_to_locale_string(ctx)?;

    Ok(())
}

/// Patch Date.prototype.toLocaleString to support the timeZone option
fn patch_date_to_locale_string(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    let date_ctor: Constructor = globals.get("Date")?;
    let date_proto: Object = date_ctor.get("prototype")?;

    // Replace toLocaleString with our implementation
    date_proto.set("toLocaleString", Func::from(date_to_locale_string))?;

    Ok(())
}

/// Custom Date.prototype.toLocaleString implementation with timezone and locale support
fn date_to_locale_string<'js>(
    ctx: Ctx<'js>,
    this: This<Value<'js>>,
    locale: Opt<Value<'js>>,
    options: Opt<Object<'js>>,
) -> Result<String> {
    // Coerce Date to number (uses valueOf internally, same as getTime)
    let epoch_ms = this
        .0
        .get::<Coerced<f64>>()
        .map(|c| c.0)
        .map_err(|_| Exception::throw_type(&ctx, "this is not a Date object"))?;

    // Check for NaN (Invalid Date)
    if epoch_ms.is_nan() {
        return Ok("Invalid Date".to_string());
    }

    // Parse locale
    let locale_str = parse_locale_arg(locale)?;
    let locale_data = get_locale_data(&locale_str);

    // Parse options
    let (tz, opts) = parse_to_locale_string_options(&ctx, options.0)?;

    let timezone =
        tz.unwrap_or_else(|| get_system_timezone().parse::<Tz>().unwrap_or(llrt_tz::UTC));

    // Convert epoch to DateTime
    let epoch_secs = (epoch_ms / 1000.0) as i64;
    let epoch_nanos = ((epoch_ms % 1000.0) * 1_000_000.0) as u32;

    let utc_dt = match Utc.timestamp_opt(epoch_secs, epoch_nanos).single() {
        Some(dt) => dt,
        None => return Ok(String::new()),
    };

    let local_dt = utc_dt.with_timezone(&timezone);

    // Determine hour12 setting - use locale default if not explicitly set
    let hour12 = if opts.hour12_set {
        Some(opts.hour12)
    } else {
        Some(locale_data.hour12_default)
    };

    // Format using CLDR patterns based on dateStyle/timeStyle
    let (date_style, time_style) = (opts.date_style.as_deref(), opts.time_style.as_deref());

    match (date_style, time_style) {
        (Some(ds), Some(ts)) => {
            // Both date and time
            let date_pattern = get_date_pattern(ds, locale_data);
            let time_pattern = get_time_pattern(ts, locale_data);
            let date_str = format_with_pattern(&local_dt, date_pattern, locale_data, hour12);
            let time_str = format_with_pattern(&local_dt, time_pattern, locale_data, hour12);
            Ok(combine_datetime(
                &date_str,
                &time_str,
                locale_data.datetime_pattern,
            ))
        },
        (Some(ds), None) => {
            // Date only
            let date_pattern = get_date_pattern(ds, locale_data);
            Ok(format_with_pattern(
                &local_dt,
                date_pattern,
                locale_data,
                hour12,
            ))
        },
        (None, Some(ts)) => {
            // Time only
            let time_pattern = get_time_pattern(ts, locale_data);
            Ok(format_with_pattern(
                &local_dt,
                time_pattern,
                locale_data,
                hour12,
            ))
        },
        (None, None) => {
            // Default: short date and medium time (matching browser behavior)
            let date_str = format_with_pattern(
                &local_dt,
                locale_data.date_formats.short,
                locale_data,
                hour12,
            );
            let time_str = format_with_pattern(
                &local_dt,
                locale_data.time_formats.medium,
                locale_data,
                hour12,
            );
            Ok(combine_datetime(
                &date_str,
                &time_str,
                locale_data.datetime_pattern,
            ))
        },
    }
}

/// Parse locale argument from JavaScript
fn parse_locale_arg(locale: Opt<Value<'_>>) -> Result<String> {
    if let Some(val) = locale.into_inner() {
        if val.is_undefined() || val.is_null() {
            return Ok("en-US".to_string());
        }
        if let Some(s) = val.as_string() {
            return s.to_string();
        }
        if let Some(arr) = val.as_array() {
            if let Ok(first) = arr.get::<Value>(0) {
                if let Some(s) = first.as_string() {
                    return s.to_string();
                }
            }
        }
    }
    Ok("en-US".to_string())
}

/// Get date pattern for a given style
fn get_date_pattern<'a>(style: &str, locale_data: &'a cldr_data::LocaleData) -> &'a str {
    match style {
        "full" => locale_data.date_formats.full,
        "long" => locale_data.date_formats.long,
        "medium" => locale_data.date_formats.medium,
        "short" => locale_data.date_formats.short,
        _ => locale_data.date_formats.medium,
    }
}

/// Get time pattern for a given style
fn get_time_pattern<'a>(style: &str, locale_data: &'a cldr_data::LocaleData) -> &'a str {
    match style {
        "full" => locale_data.time_formats.full,
        "long" => locale_data.time_formats.long,
        "medium" => locale_data.time_formats.medium,
        "short" => locale_data.time_formats.short,
        _ => locale_data.time_formats.medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_timezone() {
        let tz = get_system_timezone();
        assert!(!tz.is_empty());
        // Should be a valid IANA timezone
        assert!(tz.parse::<Tz>().is_ok());
    }

    #[test]
    fn test_system_timezone_parseable() {
        let tz_str = get_system_timezone();
        let tz: Tz = tz_str.parse().expect("System timezone should be valid");
        assert!(!tz.name().is_empty());
    }
}
