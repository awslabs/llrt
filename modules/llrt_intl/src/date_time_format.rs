// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Minimal Intl.DateTimeFormat implementation for timezone support.
//! This provides just enough functionality to support dayjs and similar libraries.

use chrono::{Datelike, Offset, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use rquickjs::{
    atom::PredefinedAtom, prelude::Opt, Array, Coerced, Ctx, Exception, Object, Result, Value,
};

/// Stores the resolved options for a DateTimeFormat instance
#[derive(Clone, Debug)]
pub struct DateTimeFormatOptions {
    pub locale: String,
    pub timezone: Tz,
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
            timezone: chrono_tz::UTC,
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
                opts.timezone = tz_val.parse().map_err(|_| {
                    Exception::throw_range(&ctx, &format!("Invalid time zone: {}", tz_val))
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
        let parts = self.format_to_parts(ctx, date)?;
        let mut result = String::new();

        for i in 0..parts.len() {
            if let Ok(part) = parts.get::<Object>(i) {
                if let Ok(value) = part.get::<_, String>("value") {
                    result.push_str(&value);
                }
            }
        }

        Ok(result)
    }

    /// Format a date to parts
    #[qjs(rename = "formatToParts")]
    pub fn format_to_parts<'js>(&self, ctx: Ctx<'js>, date: Opt<Value<'js>>) -> Result<Array<'js>> {
        // Get the timestamp from the date
        let epoch_ms = if let Some(date_val) = date.into_inner() {
            if date_val.is_undefined() {
                // Use current time
                Utc::now().timestamp_millis() as f64
            } else if let Some(num) = date_val.as_number() {
                num
            } else {
                // Try to coerce to number (works for Date objects via valueOf)
                date_val
                    .get::<Coerced<f64>>()
                    .map(|c| c.0)
                    .map_err(|_| Exception::throw_type(&ctx, "Invalid date"))?
            }
        } else {
            // No date provided, use current time
            Utc::now().timestamp_millis() as f64
        };

        let epoch_secs = (epoch_ms / 1000.0) as i64;
        let epoch_nanos = ((epoch_ms % 1000.0) * 1_000_000.0) as u32;

        let utc_dt = Utc
            .timestamp_opt(epoch_secs, epoch_nanos)
            .single()
            .ok_or_else(|| Exception::throw_range(&ctx, "Invalid timestamp"))?;

        let local_dt = utc_dt.with_timezone(&self.options.timezone);

        let parts = Array::new(ctx.clone())?;
        let mut idx = 0;

        // Helper to add a part
        let mut add_part = |part_type: &str, value: &str| -> Result<()> {
            let part = Object::new(ctx.clone())?;
            part.set("type", part_type)?;
            part.set("value", value)?;
            parts.set(idx, part)?;
            idx += 1;
            Ok(())
        };

        // Build parts based on options (following en-US format order)
        // Month
        if let Some(ref month_opt) = self.options.month {
            let month_val = match month_opt.as_str() {
                "2-digit" => format!("{:02}", local_dt.month()),
                "numeric" => format!("{}", local_dt.month()),
                _ => format!("{:02}", local_dt.month()),
            };
            add_part("month", &month_val)?;
            add_part("literal", "/")?;
        }

        // Day
        if let Some(ref day_opt) = self.options.day {
            let day_val = match day_opt.as_str() {
                "2-digit" => format!("{:02}", local_dt.day()),
                "numeric" => format!("{}", local_dt.day()),
                _ => format!("{:02}", local_dt.day()),
            };
            add_part("day", &day_val)?;
            add_part("literal", "/")?;
        }

        // Year
        if let Some(ref year_opt) = self.options.year {
            let year_val = match year_opt.as_str() {
                "2-digit" => format!("{:02}", local_dt.year() % 100),
                "numeric" => format!("{}", local_dt.year()),
                _ => format!("{}", local_dt.year()),
            };
            add_part("year", &year_val)?;
        }

        // Add separator between date and time if we have both
        let has_date = self.options.year.is_some()
            || self.options.month.is_some()
            || self.options.day.is_some();
        let has_time = self.options.hour.is_some()
            || self.options.minute.is_some()
            || self.options.second.is_some();

        if has_date && has_time {
            add_part("literal", ", ")?;
        }

        // Hour
        if self.options.hour.is_some() {
            let hour = local_dt.hour();
            let hour_val = if self.options.hour12 {
                if hour == 0 {
                    12
                } else if hour > 12 {
                    hour - 12
                } else {
                    hour
                }
            } else {
                hour
            };

            let hour_str = match self.options.hour.as_deref() {
                Some("2-digit") => format!("{:02}", hour_val),
                _ => format!("{}", hour_val),
            };
            add_part("hour", &hour_str)?;

            if self.options.minute.is_some() || self.options.second.is_some() {
                add_part("literal", ":")?;
            }
        }

        // Minute
        if self.options.minute.is_some() {
            let minute_str = match self.options.minute.as_deref() {
                Some("2-digit") => format!("{:02}", local_dt.minute()),
                _ => format!("{}", local_dt.minute()),
            };
            add_part("minute", &minute_str)?;

            if self.options.second.is_some() {
                add_part("literal", ":")?;
            }
        }

        // Second
        if self.options.second.is_some() {
            let second_str = match self.options.second.as_deref() {
                Some("2-digit") => format!("{:02}", local_dt.second()),
                _ => format!("{}", local_dt.second()),
            };
            add_part("second", &second_str)?;
        }

        // dayPeriod for 12-hour format
        if self.options.hour12 && self.options.hour.is_some() {
            let hour = local_dt.hour();
            add_part("literal", " ")?;
            add_part("dayPeriod", if hour >= 12 { "PM" } else { "AM" })?;
        }

        // Timezone name
        if let Some(ref tz_name_opt) = self.options.timezone_name {
            add_part("literal", " ")?;
            let tz_str = match tz_name_opt.as_str() {
                "short" => {
                    // Format as offset like "GMT-7" or abbreviated name
                    let offset = local_dt.offset().fix();
                    let hours = offset.local_minus_utc() / 3600;
                    if hours >= 0 {
                        format!("GMT+{}", hours)
                    } else {
                        format!("GMT{}", hours)
                    }
                },
                "long" => self.options.timezone.name().to_string(),
                "shortOffset" => {
                    let offset = local_dt.offset().fix();
                    let hours = offset.local_minus_utc() / 3600;
                    let mins = (offset.local_minus_utc() % 3600) / 60;
                    if mins == 0 {
                        if hours >= 0 {
                            format!("GMT+{}", hours)
                        } else {
                            format!("GMT{}", hours)
                        }
                    } else if hours >= 0 {
                        format!("GMT+{}:{:02}", hours, mins.abs())
                    } else {
                        format!("GMT{}:{:02}", hours, mins.abs())
                    }
                },
                _ => self.options.timezone.name().to_string(),
            };
            add_part("timeZoneName", &tz_str)?;
        }

        Ok(parts)
    }

    /// Return resolved options
    #[qjs(rename = "resolvedOptions")]
    pub fn resolved_options<'js>(&self, ctx: Ctx<'js>) -> Result<Object<'js>> {
        let obj = Object::new(ctx)?;

        obj.set("locale", self.options.locale.as_str())?;
        obj.set("calendar", "gregory")?;
        obj.set("numberingSystem", "latn")?;
        obj.set("timeZone", self.options.timezone.name())?;

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

/// Get the system's default timezone
pub fn get_system_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_string())
}

/// Format a date in the specified timezone using locale options.
/// This is used to implement Date.prototype.toLocaleString with timezone support.
pub fn format_date_in_timezone(
    epoch_ms: f64,
    timezone: &Tz,
    options: &ToLocaleStringOptions,
) -> String {
    let epoch_secs = (epoch_ms / 1000.0) as i64;
    let epoch_nanos = ((epoch_ms % 1000.0) * 1_000_000.0) as u32;

    let utc_dt = match Utc.timestamp_opt(epoch_secs, epoch_nanos).single() {
        Some(dt) => dt,
        None => return String::new(),
    };

    let local_dt = utc_dt.with_timezone(timezone);

    // Format as MM/DD/YYYY, HH:MM:SS AM/PM (en-US style)
    let month = local_dt.month();
    let day = local_dt.day();
    let year = local_dt.year();
    let hour = local_dt.hour();
    let minute = local_dt.minute();
    let second = local_dt.second();

    if options.hour12 {
        let (hour12, period) = if hour == 0 {
            (12, "AM")
        } else if hour < 12 {
            (hour, "AM")
        } else if hour == 12 {
            (12, "PM")
        } else {
            (hour - 12, "PM")
        };
        format!(
            "{:02}/{:02}/{}, {}:{:02}:{:02} {}",
            month, day, year, hour12, minute, second, period
        )
    } else {
        format!(
            "{:02}/{:02}/{}, {:02}:{:02}:{:02}",
            month, day, year, hour, minute, second
        )
    }
}

/// Options for toLocaleString
#[derive(Default)]
pub struct ToLocaleStringOptions {
    pub hour12: bool,
}

/// Parse toLocaleString options from a JavaScript object
pub fn parse_to_locale_string_options<'js>(
    ctx: &Ctx<'js>,
    options: Option<Object<'js>>,
) -> Result<(Option<Tz>, ToLocaleStringOptions)> {
    let mut tz: Option<Tz> = None;
    let mut opts = ToLocaleStringOptions::default();

    if let Some(options_obj) = options {
        // Parse timeZone
        if let Ok(tz_val) = options_obj.get::<_, String>("timeZone") {
            tz = Some(tz_val.parse().map_err(|_| {
                Exception::throw_range(ctx, &format!("Invalid time zone: {}", tz_val))
            })?);
        }

        // Parse hour12 (defaults to true for en-US)
        if let Ok(h12) = options_obj.get::<_, bool>("hour12") {
            opts.hour12 = h12;
        } else {
            // Default to 12-hour for en-US locale
            opts.hour12 = true;
        }
    } else {
        // Default to 12-hour for en-US locale
        opts.hour12 = true;
    }

    Ok((tz, opts))
}
