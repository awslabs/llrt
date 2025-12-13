// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod text_decoder;
pub mod text_encoder;

use llrt_logging::{format_plain, inspect_value, InspectOptions, SortMode};
use llrt_utils::{
    module::{export_default, ModuleInfo},
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    function::Func,
    module::{Declarations, Exports, ModuleDef},
    prelude::Opt,
    Class, Ctx, Function, Object, Result, Value,
};
use text_decoder::TextDecoder;
use text_encoder::TextEncoder;

fn inherits<'js>(ctor: Function<'js>, super_ctor: Function<'js>) -> Result<()> {
    let super_proto: Object<'js> = super_ctor.get("prototype")?;
    let proto: Object<'js> = ctor.get("prototype")?;
    proto.set_prototype(Some(&super_proto))?;
    ctor.set("super_", super_ctor)?;
    Ok(())
}

/// Parse inspect options from a JavaScript value (object or boolean for legacy API)
/// Returns the options and an optional sort comparator function
fn parse_inspect_options<'js>(
    opts: Option<&Value<'js>>,
) -> (InspectOptions, Option<Function<'js>>) {
    // Start with util.inspect defaults (depth 2, break heuristics enabled)
    let mut options = InspectOptions::for_inspect();
    let mut sort_comparator: Option<Function<'js>> = None;

    if let Some(opts_val) = opts {
        if let Some(opts_obj) = opts_val.as_object() {
            // New API: util.inspect(obj, options)
            if let Ok(val) = opts_obj.get::<_, bool>("showHidden") {
                options.show_hidden = val;
            }
            if let Ok(val) = opts_obj.get::<_, Value>("depth") {
                if val.is_null() {
                    options.depth = usize::MAX;
                } else if let Some(d) = val.as_int() {
                    options.depth = d.max(0) as usize;
                }
            }
            if let Ok(val) = opts_obj.get::<_, bool>("colors") {
                options.colors = val;
            }
            if let Ok(val) = opts_obj.get::<_, bool>("customInspect") {
                options.custom_inspect = val;
            }
            if let Ok(val) = opts_obj.get::<_, i32>("maxArrayLength") {
                options.max_array_length = val.max(0) as usize;
            }
            if let Ok(val) = opts_obj.get::<_, i32>("maxStringLength") {
                options.max_string_length = val.max(0) as usize;
            }
            if let Ok(val) = opts_obj.get::<_, i32>("breakLength") {
                options.break_length = val.max(0) as usize;
            }
            // sorted can be a boolean or a comparator function
            if let Ok(val) = opts_obj.get::<_, Value>("sorted") {
                if let Some(b) = val.as_bool() {
                    options.sorted = if b {
                        SortMode::Alphabetical
                    } else {
                        SortMode::None
                    };
                } else if val.is_function() {
                    options.sorted = SortMode::Custom;
                    sort_comparator = val.into_function();
                }
            }
            if let Ok(val) = opts_obj.get::<_, Value>("compact") {
                // compact can be a number or boolean false (which means 0)
                if let Some(b) = val.as_bool() {
                    options.compact = if b { 3 } else { 0 };
                } else if let Some(n) = val.as_int() {
                    options.compact = n.max(0) as usize;
                }
            }
        } else if let Some(sh) = opts_val.as_bool() {
            // Legacy API: util.inspect(obj, showHidden) - just first bool arg
            options.show_hidden = sh;
        }
    }

    (options, sort_comparator)
}

/// util.inspect(object[, options])
fn inspect<'js>(ctx: Ctx<'js>, value: Value<'js>, opts: Opt<Value<'js>>) -> Result<String> {
    let (options, sort_comparator) = parse_inspect_options(opts.0.as_ref());
    inspect_value(&ctx, value, options, sort_comparator)
}

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare(stringify!(format))?;
        declare.declare(stringify!(inherits))?;
        declare.declare(stringify!(inspect))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;
            default.set(
                "format",
                Func::from(|ctx, args| format_plain(ctx, true, args)),
            )?;
            default.set("inherits", Func::from(inherits))?;

            // Set inspect function
            default.set("inspect", Func::from(inspect))?;

            // Get the function back to add properties
            let inspect_fn: Function = default.get("inspect")?;

            // Add inspect.custom symbol
            let primordials = BasePrimordials::get(ctx)?;
            let custom_symbol = primordials.symbol_custom_inspect.clone();
            inspect_fn.set("custom", custom_symbol)?;

            // Add inspect.defaultOptions
            let default_options = Object::new(ctx.clone())?;
            default_options.set("showHidden", false)?;
            default_options.set("depth", 2)?;
            default_options.set("colors", false)?;
            default_options.set("customInspect", true)?;
            default_options.set("maxArrayLength", 100)?;
            default_options.set("maxStringLength", 10000)?;
            default_options.set("breakLength", 80)?;
            default_options.set("compact", 3)?;
            default_options.set("sorted", false)?;
            inspect_fn.set("defaultOptions", default_options)?;

            Ok(())
        })
    }
}

impl From<UtilModule> for ModuleInfo<UtilModule> {
    fn from(val: UtilModule) -> Self {
        ModuleInfo {
            name: "util",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<TextEncoder>::define(&globals)?;
    Class::<TextDecoder>::define(&globals)?;

    Ok(())
}
