// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! LLRT timezone module providing timezone offset calculations.

use chrono::{Offset, TimeZone, Utc};
use chrono_tz::Tz;
use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Array, Ctx, Exception, Object, Result,
};

use crate::libs::utils::module::{export_default, ModuleInfo};

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

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    globals.set("Timezone", timezone_object(ctx)?)?;
    Ok(())
}

pub struct LlrtTimezoneModule;

impl ModuleDef for LlrtTimezoneModule {
    fn declare(decl: &Declarations) -> Result<()> {
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

impl From<LlrtTimezoneModule> for ModuleInfo<LlrtTimezoneModule> {
    fn from(val: LlrtTimezoneModule) -> Self {
        ModuleInfo {
            name: "llrt:timezone",
            module: val,
        }
    }
}
