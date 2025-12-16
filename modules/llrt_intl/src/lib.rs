// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Minimal Intl module for LLRT.
//!
//! Provides a subset of the `Intl` API focused on timezone support,
//! enabling compatibility with libraries like dayjs.

mod date_time_format;

pub use date_time_format::{
    format_date_in_timezone, get_system_timezone, parse_to_locale_string_options, DateTimeFormat,
    DateTimeFormatOptions, ToLocaleStringOptions,
};

use chrono_tz::Tz;
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

/// Custom Date.prototype.toLocaleString implementation with timezone support
///
/// Note: The `locale` parameter is accepted but not used. It's required in the function
/// signature to match the JavaScript API `toLocaleString(locales, options)`, ensuring
/// the options object is correctly received as the second argument.
fn date_to_locale_string<'js>(
    ctx: Ctx<'js>,
    this: This<Value<'js>>,
    _locale: Opt<Value<'js>>,
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

    let (tz, opts) = parse_to_locale_string_options(&ctx, options.0)?;

    let timezone = tz.unwrap_or_else(|| {
        get_system_timezone()
            .parse::<Tz>()
            .unwrap_or(chrono_tz::UTC)
    });

    Ok(format_date_in_timezone(epoch_ms, &timezone, &opts))
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
