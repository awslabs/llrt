// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

mod date_time_format;

use chrono::{Offset, TimeZone, Utc};
use chrono_tz::Tz;
use llrt_utils::module::export_default;
use rquickjs::{
    atom::PredefinedAtom,
    function::This,
    module::Exports,
    prelude::{Func, Opt},
    Array, Class, Coerced, Ctx, Exception, Function, Object, Result, Value,
};

use crate::date_time_format::{
    format_date_in_timezone, parse_to_locale_string_options, DateTimeFormat,
};

/// Get the UTC offset in minutes for a timezone at a given epoch milliseconds.
/// Returns a positive value for timezones ahead of UTC (e.g., +60 for UTC+1)
/// and a negative value for timezones behind UTC (e.g., -420 for UTC-7).
fn get_offset(ctx: Ctx<'_>, timezone: String, epoch_ms: f64) -> Result<i32> {
    let tz: Tz = timezone
        .parse()
        .map_err(|_| Exception::throw_type(&ctx, &format!("Invalid timezone: {}", timezone)))?;

    let epoch_secs = (epoch_ms / 1000.0) as i64;
    let naive = Utc.timestamp_opt(epoch_secs, 0).single().ok_or_else(|| {
        Exception::throw_range(&ctx, &format!("Invalid epoch milliseconds: {}", epoch_ms))
    })?;

    let local = naive.with_timezone(&tz);
    let offset = local.offset().fix().local_minus_utc();

    // Return offset in minutes (positive = ahead of UTC, negative = behind)
    Ok(offset / 60)
}

/// List all available IANA timezone names.
fn list_timezones(ctx: Ctx<'_>) -> Result<Array<'_>> {
    let timezones = chrono_tz::TZ_VARIANTS;
    let array = Array::new(ctx.clone())?;

    for (i, tz) in timezones.iter().enumerate() {
        array.set(i, tz.name())?;
    }

    Ok(array)
}

fn timezone_object<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    let timezone = Object::new(ctx.clone())?;

    timezone.set("getOffset", Func::from(get_offset))?;
    timezone.set("list", Func::from(list_timezones))?;
    timezone.set(PredefinedAtom::SymbolToStringTag, "Timezone")?;

    Ok(timezone)
}

fn intl_object<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    let intl = Object::new(ctx.clone())?;

    // Add DateTimeFormat constructor
    Class::<DateTimeFormat>::define(&intl)?;

    intl.set(PredefinedAtom::SymbolToStringTag, "Intl")?;

    Ok(intl)
}

/// Custom toLocaleString implementation that supports timezone option
fn to_locale_string<'js>(
    ctx: Ctx<'js>,
    this: This<Value<'js>>,
    _locales: Opt<Value<'js>>,
    options: Opt<Object<'js>>,
) -> Result<String> {
    // Get the timestamp from the Date object
    let epoch_ms = this
        .0
        .get::<Coerced<f64>>()
        .map(|c| c.0)
        .map_err(|_| Exception::throw_type(&ctx, "this is not a Date object"))?;

    if epoch_ms.is_nan() {
        return Ok("Invalid Date".to_string());
    }

    // Parse options to get timezone
    let (tz, opts) = parse_to_locale_string_options(&ctx, options.into_inner())?;

    if let Some(timezone) = tz {
        // Format in the specified timezone
        Ok(format_date_in_timezone(epoch_ms, &timezone, &opts))
    } else {
        // No timezone specified - use the original method
        // We need to call the original toLocaleString
        let globals = ctx.globals();
        let date_proto: Object = globals.get::<_, Function>("Date")?.get("prototype")?;
        let original_fn: Function = date_proto.get("__originalToLocaleString")?;
        original_fn.call((This(this.0.clone()),))
    }
}

/// Patch Date.prototype.toLocaleString to support timezone option
fn patch_date_to_locale_string(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    let date_constructor: Function = globals.get("Date")?;
    let date_prototype: Object = date_constructor.get("prototype")?;

    // Store the original toLocaleString
    let original_to_locale_string: Function = date_prototype.get("toLocaleString")?;
    date_prototype.set("__originalToLocaleString", original_to_locale_string)?;

    // Replace with our timezone-aware version
    date_prototype.set("toLocaleString", Func::from(to_locale_string))?;

    Ok(())
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    // Add Timezone global
    globals.set("Timezone", timezone_object(ctx)?)?;

    // Add Intl global with DateTimeFormat
    globals.set("Intl", intl_object(ctx)?)?;

    // Patch Date.prototype.toLocaleString to support timezone option
    patch_date_to_locale_string(ctx)?;

    Ok(())
}

pub struct TimezoneModule;

impl rquickjs::module::ModuleDef for TimezoneModule {
    fn declare(decl: &rquickjs::module::Declarations) -> Result<()> {
        decl.declare("Timezone")?;
        decl.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let timezone = timezone_object(ctx)?;
        export_default(ctx, exports, |default| {
            default.set("Timezone", timezone.clone())?;
            Ok(())
        })?;
        exports.export("Timezone", timezone)?;
        Ok(())
    }
}

impl From<TimezoneModule> for llrt_utils::module::ModuleInfo<TimezoneModule> {
    fn from(val: TimezoneModule) -> Self {
        llrt_utils::module::ModuleInfo {
            name: "llrt:timezone",
            module: val,
        }
    }
}

// Re-export for use by other modules
pub use date_time_format::get_system_timezone;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timezone() {
        let tz: Tz = "America/Denver".parse().unwrap();
        assert_eq!(tz.name(), "America/Denver");
    }

    #[test]
    fn test_invalid_timezone() {
        let result: std::result::Result<Tz, _> = "Invalid/Timezone".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_system_timezone() {
        let tz = get_system_timezone();
        assert!(!tz.is_empty());
    }
}
