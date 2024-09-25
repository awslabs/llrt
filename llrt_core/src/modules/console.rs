// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    fmt::Write as FormatWrite,
    io::{stderr, stdout, IsTerminal, Write},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use chrono::{DateTime, Utc};
use fxhash::FxHashSet;
use llrt_utils::{class::get_class_name, object::CreateSymbol};
use rquickjs::{
    atom::PredefinedAtom,
    function::This,
    module::{Declarations, Exports, ModuleDef},
    object::Filter,
    prelude::{Func, Rest},
    Array, Class, Coerced, Ctx, Function, Object, Result, Symbol, Type, Value,
};

use crate::json::stringify::json_stringify;
use crate::module_builder::ModuleInfo;
use crate::modules::module::export_default;
use crate::number::float_to_string;
use crate::{json::escape::escape_json, runtime_client, utils::result::ResultExt};

pub static AWS_LAMBDA_MODE: AtomicBool = AtomicBool::new(false);
pub static AWS_LAMBDA_JSON_LOG_FORMAT: AtomicBool = AtomicBool::new(false);
pub static AWS_LAMBDA_JSON_LOG_LEVEL: AtomicUsize = AtomicUsize::new(LogLevel::Info as usize);

use llrt_utils::class::CUSTOM_INSPECT_SYMBOL_DESCRIPTION;

const NEWLINE: char = '\n';
const SPACING: char = ' ';
const CIRCULAR: &str = "[Circular]";
const OBJECT_ARRAY_LOOKUP: [&str; 2] = ["[Array]", "[Object]"];
const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

const MAX_INDENTATION_LEVEL: usize = 4;
const MAX_EXPANSION_DEPTH: usize = 4;
const OBJECT_ARRAY_START: [char; 2] = ['[', '{'];
const OBJECT_ARRAY_END: [char; 2] = [']', '}'];
const LINE_BREAK_LOOKUP: [&str; 3] = ["", "\r", "\n"];
const SPACING_LOOKUP: [&str; 2] = ["", " "];
const SINGLE_QUOTE_LOOKUP: [&str; 2] = ["", "\'"];
const CLASS_FUNCTION_LOOKUP: [&str; 2] = ["[function: ", "[class: "];
const INDENTATION_LOOKUP: [&str; MAX_INDENTATION_LEVEL + 1] =
    ["", "  ", "    ", "        ", "                "];

macro_rules! ascii_colors {
    ( $( $name:ident => $value:expr ),* ) => {
        #[derive(Debug, Clone, Copy)]
        pub enum Color {
            $(
                $name = $value+1,
            )*
        }

        pub const COLOR_LOOKUP: [&str; 39] = {
            let mut array = [""; 39];
            $(
                //shift 1 position so if disabled we return ""
                array[Color::$name as usize] = concat!("\x1b[", stringify!($value), "m");
            )*
            array
        };
    }
}

ascii_colors!(
    RESET => 0,
    BOLD => 1,
    BLACK => 30,
    RED => 31,
    GREEN => 32,
    YELLOW => 33,
    BLUE => 34,
    MAGENTA => 35,
    CYAN => 36,
    WHITE => 37
);

#[derive(Clone)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 4,
    Error = 8,
    Fatal = 16,
}

trait PushByte {
    fn push_byte(&mut self, byte: u8);
}

impl PushByte for String {
    fn push_byte(&mut self, byte: u8) {
        unsafe { self.as_mut_vec() }.push(byte);
    }
}

impl LogLevel {
    fn to_string(&self) -> String {
        match self {
            LogLevel::Trace => String::from("TRACE"),
            LogLevel::Debug => String::from("DEBUG"),
            LogLevel::Info => String::from("INFO"),
            LogLevel::Warn => String::from("WARN"),
            LogLevel::Error => String::from("ERROR"),
            LogLevel::Fatal => String::from("FATAL"),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            "FATAL" => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }
}

pub struct ConsoleModule;

impl ModuleDef for ConsoleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(Console))?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        Class::<Console>::register(ctx)?;

        export_default(ctx, exports, |default| {
            Class::<Console>::define(default)?;

            Ok(())
        })
    }
}

impl From<ConsoleModule> for ModuleInfo<ConsoleModule> {
    fn from(val: ConsoleModule) -> Self {
        ModuleInfo {
            name: "console",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;

    console.set("log", Func::from(log))?;
    console.set("clear", Func::from(clear))?;
    console.set("debug", Func::from(log_debug))?;
    console.set("info", Func::from(log))?;
    console.set("trace", Func::from(log_trace))?;
    console.set("error", Func::from(log_error))?;
    console.set("warn", Func::from(log_warn))?;
    console.set("assert", Func::from(log_assert))?;
    console.set("__format", Func::from(|ctx, args| format(&ctx, args)))?;

    globals.set("console", console)?;

    Ok(())
}

#[inline(always)]
fn write_sep(result: &mut String, add_comma: bool, has_indentation: bool, newline: bool) {
    const SEPARATOR_TABLE: [&str; 8] = ["", ",", "\r", ",\r", " ", ", ", "\n", ",\n"];
    let index = (add_comma as usize) | (has_indentation as usize) << 1 | (newline as usize) << 2;
    result.push_str(SEPARATOR_TABLE[index]);
}

#[inline(always)]
fn push_indentation(result: &mut String, depth: usize) {
    result.push_str(INDENTATION_LOOKUP[depth]);
}

impl Color {
    #[inline(always)]
    fn push(self, value: &mut String, enabled: usize) {
        value.push_str(COLOR_LOOKUP[self as usize & enabled])
    }

    #[inline(always)]
    fn reset(value: &mut String, enabled: usize) {
        value.push_str(COLOR_LOOKUP[Color::RESET as usize & enabled])
    }
}

fn log_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_err(&ctx, args, LogLevel::Error)
}

fn log_warn<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_err(&ctx, args, LogLevel::Warn)
}

fn log_debug<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_out(&ctx, args, LogLevel::Debug)
}

fn log_trace<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_out(&ctx, args, LogLevel::Trace)
}

fn log_assert<'js>(ctx: Ctx<'js>, expression: bool, args: Rest<Value<'js>>) -> Result<()> {
    if !expression {
        log_error(ctx, args)?;
    }

    Ok(())
}

fn log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_out(&ctx, args, LogLevel::Info)
}

fn clear() {
    let _ = stdout().write_all(b"\x1b[1;1H\x1b[0J");
}

pub fn format_plain<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<String> {
    format_values(&ctx, args, false)
}

pub fn format<'js>(ctx: &Ctx<'js>, args: Rest<Value<'js>>) -> Result<String> {
    format_values(ctx, args, stdout().is_terminal())
}

#[inline(always)]
fn format_raw<'js>(
    result: &mut String,
    value: Value<'js>,
    options: &FormatOptions<'js>,
) -> Result<()> {
    format_raw_inner(result, value, options, &mut FxHashSet::default(), 0)?;
    Ok(())
}

fn format_raw_string<'js>(result: &mut String, value: String, options: &FormatOptions<'js>) {
    let (color_enabled_mask, not_root_mask, not_root) = get_masks(options, 0);
    format_raw_string_inner(result, value, not_root_mask, color_enabled_mask, not_root);
}

fn format_raw_inner<'js>(
    result: &mut String,
    value: Value<'js>,
    options: &FormatOptions<'js>,
    visited: &mut FxHashSet<usize>,
    depth: usize,
) -> Result<()> {
    let value_type = value.type_of();

    let (color_enabled_mask, not_root_mask, not_root) = get_masks(options, depth);

    match value_type {
        Type::Uninitialized | Type::Null => {
            Color::BOLD.push(result, color_enabled_mask);
            result.push_str("null")
        },
        Type::Undefined => {
            Color::BLACK.push(result, color_enabled_mask);
            result.push_str("undefined")
        },
        Type::Bool => {
            Color::YELLOW.push(result, color_enabled_mask);
            const BOOL_STRINGS: [&str; 2] = ["false", "true"];
            result.push_str(BOOL_STRINGS[unsafe { value.as_bool().unwrap_unchecked() } as usize]);
        },
        Type::BigInt => {
            Color::YELLOW.push(result, color_enabled_mask);
            let mut buffer = itoa::Buffer::new();
            let big_int = unsafe { value.as_big_int().unwrap_unchecked() };
            result.push_str(buffer.format(big_int.clone().to_i64().unwrap()));
            result.push('n');
        },
        Type::Int => {
            Color::YELLOW.push(result, color_enabled_mask);
            let mut buffer = itoa::Buffer::new();
            result.push_str(buffer.format(unsafe { value.as_int().unwrap_unchecked() }));
        },
        Type::Float => {
            Color::YELLOW.push(result, color_enabled_mask);
            let mut buffer = ryu::Buffer::new();
            result.push_str(float_to_string(&mut buffer, unsafe {
                value.as_float().unwrap_unchecked()
            }));
        },
        Type::String => {
            format_raw_string_inner(
                result,
                unsafe {
                    value
                        .as_string()
                        .unwrap_unchecked()
                        .to_string()
                        .unwrap_unchecked()
                },
                not_root_mask,
                color_enabled_mask,
                not_root,
            );
        },
        Type::Symbol => {
            Color::YELLOW.push(result, color_enabled_mask);
            let description = unsafe { value.as_symbol().unwrap_unchecked() }.description()?;
            result.push_str("Symbol(");
            result.push_str(&unsafe { description.get::<String>().unwrap_unchecked() });
            result.push(')');
        },
        Type::Function | Type::Constructor => {
            Color::CYAN.push(result, color_enabled_mask);
            let obj = unsafe { value.as_object().unwrap_unchecked() };

            const ANONYMOUS: &str = "(anonymous)";
            let mut name: String = obj
                .get(PredefinedAtom::Name)
                .unwrap_or(String::with_capacity(ANONYMOUS.len()));
            if name.is_empty() {
                name.push_str(ANONYMOUS);
            }

            let mut is_class = false;
            if obj.contains_key(PredefinedAtom::Prototype)? {
                let desc: Object = options
                    .get_own_property_desc_fn
                    .call((value, "prototype"))?;
                let writable: bool = desc.get(PredefinedAtom::Writable)?;
                is_class = !writable;
            }
            result.push_str(CLASS_FUNCTION_LOOKUP[is_class as usize]);
            result.push_str(&name);
            result.push(']');
        },
        Type::Promise => {
            result.push_str("Promise {}");
            return Ok(());
        },
        Type::Array | Type::Object | Type::Exception => {
            let hash = fxhash::hash(&value);
            if visited.contains(&hash) {
                Color::CYAN.push(result, color_enabled_mask);
                result.push_str(CIRCULAR);
                Color::reset(result, color_enabled_mask);
                return Ok(());
            }
            visited.insert(hash);
            let obj = unsafe { value.as_object().unwrap_unchecked() };

            if value.is_error() {
                let name: String = obj.get(PredefinedAtom::Name)?;
                let message: String = obj.get(PredefinedAtom::Message)?;
                let stack: Result<String> = obj.get(PredefinedAtom::Stack);
                result.push_str(&name);
                result.push_str(": ");
                result.push_str(&message);
                Color::BLACK.push(result, color_enabled_mask);
                if let Ok(stack) = stack {
                    for line in stack.trim().split('\n') {
                        result.push_str(LINE_BREAK_LOOKUP[1 + (options.newline as usize)]);
                        push_indentation(result, depth + 1);
                        result.push_str(line);
                    }
                }
                Color::reset(result, color_enabled_mask);
                return Ok(());
            }

            let mut class_name: Option<String> = None;
            let mut is_object = false;
            if value_type == Type::Object {
                is_object = true;
                class_name = get_class_name(&value)?;
                match class_name.as_deref() {
                    Some("Date") => {
                        Color::MAGENTA.push(result, color_enabled_mask);
                        let iso_fn: Function = obj.get("toISOString").unwrap();
                        let str: String = iso_fn.call((This(value),))?;
                        result.push_str(&str);
                        Color::reset(result, color_enabled_mask);
                        return Ok(());
                    },
                    Some("RegExp") => {
                        Color::RED.push(result, color_enabled_mask);
                        let source: String = obj.get("source")?;
                        let flags: String = obj.get("flags")?;
                        result.push('/');
                        result.push_str(&source);
                        result.push('/');
                        result.push_str(&flags);
                        Color::reset(result, color_enabled_mask);
                        return Ok(());
                    },
                    None | Some("") | Some("Object") => {
                        class_name = None;
                    },
                    _ => {},
                }
            }

            if depth < MAX_EXPANSION_DEPTH {
                let mut is_typed_array = false;
                if let Some(class_name) = class_name {
                    result.push_str(&class_name);
                    result.push(SPACING);
                    is_typed_array = matches!(
                        class_name.as_str(),
                        "Int8Array"
                            | "Uint8Array"
                            | "Uint8ClampedArray"
                            | "Int16Array"
                            | "Uint16Array"
                            | "Int32Array"
                            | "Uint32Array"
                            | "Int64Array"
                            | "Uint64Array"
                            | "Float32Array"
                            | "Float64Array"
                            | "Buffer"
                    );
                }

                let is_array = is_typed_array || obj.is_array();

                if let Ok(obj) = &obj.get::<_, Object>(options.custom_inspect_symbol.as_atom()) {
                    return write_object(
                        result,
                        obj,
                        options,
                        visited,
                        depth,
                        color_enabled_mask,
                        is_array,
                    );
                }

                write_object(
                    result,
                    obj,
                    options,
                    visited,
                    depth,
                    color_enabled_mask,
                    is_array,
                )?;
            } else {
                Color::CYAN.push(result, color_enabled_mask);
                result.push_str(OBJECT_ARRAY_LOOKUP[is_object as usize]);
            }
        },
        _ => {},
    }

    Color::reset(result, color_enabled_mask);

    Ok(())
}

#[inline(always)]
fn get_masks(options: &FormatOptions<'_>, depth: usize) -> (usize, usize, usize) {
    let color_enabled_mask = bitmask(options.color);
    let not_root_mask = bitmask(depth != 0);
    let not_root = (depth != 0) as usize;
    (color_enabled_mask, not_root_mask, not_root)
}

fn format_raw_string_inner(
    result: &mut String,
    value: String,
    not_root_mask: usize,
    color_enabled_mask: usize,
    not_root: usize,
) {
    Color::GREEN.push(result, not_root_mask & color_enabled_mask);
    result.push_str(SINGLE_QUOTE_LOOKUP[not_root]);
    result.push_str(&value);
    result.push_str(SINGLE_QUOTE_LOOKUP[not_root]);
}

fn write_object<'js>(
    result: &mut String,
    obj: &Object<'js>,
    options: &FormatOptions<'js>,
    visited: &mut FxHashSet<usize>,
    depth: usize,
    color_enabled_mask: usize,
    is_array: bool,
) -> Result<()> {
    result.push(OBJECT_ARRAY_START[(!is_array) as usize]);

    let mut keys = obj.keys();
    let mut filter_functions = false;
    if !is_array && keys.len() == 0 {
        if let Some(proto) = obj.get_prototype() {
            if proto != options.object_prototype {
                keys = proto.own_keys(options.object_filter);

                filter_functions = true;
            }
        }
    }
    let apply_indentation = bitmask(!is_array && depth < 2);

    let mut first = 0;
    let mut numeric_key;
    let length = keys.len();
    for (i, key) in keys.flatten().enumerate() {
        let value: Value = obj.get::<&String, _>(&key)?;
        if !(value.is_function() && filter_functions) {
            numeric_key = if key.parse::<f64>().is_ok() { !0 } else { 0 };
            write_sep(result, first > 0, apply_indentation > 0, options.newline);
            push_indentation(result, apply_indentation & (depth + 1));
            if depth > MAX_INDENTATION_LEVEL - 1 {
                result.push(SPACING);
            }
            if !is_array {
                Color::GREEN.push(result, color_enabled_mask & numeric_key);
                result.push_str(SINGLE_QUOTE_LOOKUP[numeric_key & 1]);
                result.push_str(&key);
                result.push_str(SINGLE_QUOTE_LOOKUP[numeric_key & 1]);
                Color::reset(result, color_enabled_mask & numeric_key);
                result.push(':');
                result.push(SPACING);
            }

            format_raw_inner(result, value, options, visited, depth + 1)?;
            first = !0;
            if i > 99 {
                result.push_str("... ");
                let mut buffer = itoa::Buffer::new();
                result.push_str(buffer.format(length - i));
                result.push_str(" more items");
                break;
            }
        }
    }
    result
        .push_str(LINE_BREAK_LOOKUP[first & apply_indentation & (1 + (options.newline as usize))]);
    result.push_str(SPACING_LOOKUP[first & !apply_indentation & 1]);
    push_indentation(result, first & apply_indentation & depth);
    result.push(OBJECT_ARRAY_END[(!is_array) as usize]);
    Ok(())
}

#[inline(always)]
fn bitmask(condition: bool) -> usize {
    !(condition as usize).wrapping_sub(1)
}

fn format_values_internal<'js>(
    result: &mut String,
    ctx: &Ctx<'js>,
    args: Rest<Value<'js>>,
    options: &mut FormatOptions<'js>,
) -> Result<()> {
    let size = args.len();
    let mut iter = args.0.into_iter().enumerate().peekable();

    let current_filter = options.object_filter;
    let default_filter = Filter::default();

    while let Some((index, arg)) = iter.next() {
        if index == 0 && size > 1 {
            if let Some(str) = arg.as_string() {
                let str = str.to_string()?;

                //fast check for format any strings
                if str.find('%').is_none() {
                    format_raw_string(result, str, options);
                    continue;
                }
                let bytes = str.as_bytes();
                bytes.iter().position(|p| *p == b'%');
                let mut i = 0;
                let len = bytes.len();
                let mut next_byte;
                let mut byte;
                while i < len {
                    byte = bytes[i];
                    if byte == b'%' && i + 1 < len {
                        next_byte = bytes[i + 1];
                        i += 1;
                        if iter.peek().is_some() {
                            i += 1;

                            let mut next_value = || unsafe { iter.next().unwrap_unchecked() }.1;

                            let value = match next_byte {
                                b's' => {
                                    let str = next_value().get::<Coerced<String>>()?;
                                    result.push_str(str.as_str());
                                    continue;
                                },
                                b'd' => options.number_function.call((next_value(),))?,
                                b'i' => options.parse_int.call((next_value(),))?,
                                b'f' => options.parse_float.call((next_value(),))?,
                                b'j' => {
                                    result.push_str(
                                        &json_stringify(ctx, next_value())?
                                            .unwrap_or("undefined".into()),
                                    );
                                    continue;
                                },
                                b'O' => {
                                    options.object_filter = default_filter;
                                    next_value()
                                },
                                b'o' => next_value(),
                                b'c' => {
                                    // CSS is ignored
                                    continue;
                                },
                                b'%' => {
                                    result.push_byte(byte);
                                    result.push_byte(byte);
                                    continue;
                                },
                                _ => {
                                    result.push_byte(byte);
                                    result.push_byte(next_byte);
                                    continue;
                                },
                            };
                            options.color = false;

                            format_raw(result, value, options)?;
                            options.object_filter = current_filter;
                            continue;
                        }
                        result.push_byte(byte);
                        result.push_byte(next_byte);
                    } else {
                        result.push_byte(byte);
                    }

                    i += 1;
                }
                continue;
            }
        }
        if index != 0 {
            result.push(SPACING);
        }
        format_raw(result, arg, options)?;
    }

    Ok(())
}

struct FormatOptions<'js> {
    color: bool,
    newline: bool,
    get_own_property_desc_fn: Function<'js>,
    object_prototype: Object<'js>,
    number_function: Function<'js>,
    parse_float: Function<'js>,
    parse_int: Function<'js>,
    object_filter: Filter,
    custom_inspect_symbol: Symbol<'js>,
}
impl<'js> FormatOptions<'js> {
    fn new(ctx: &Ctx<'js>, color: bool, newline: bool) -> Result<Self> {
        let globals = ctx.globals();
        let default_obj = Object::new(ctx.clone())?;
        let object_ctor: Object = default_obj.get(PredefinedAtom::Constructor)?;
        let object_prototype = default_obj
            .get_prototype()
            .ok_or("Can't get prototype")
            .or_throw(ctx)?;
        let get_own_property_desc_fn: Function =
            object_ctor.get(PredefinedAtom::GetOwnPropertyDescriptor)?;

        let number_function = globals.get(PredefinedAtom::Number)?;
        let parse_float = globals.get("parseFloat")?;
        let parse_int = globals.get("parseInt")?;

        let object_filter = Filter::new().private().string().symbol();
        let custom_inspect_symbol =
            Symbol::for_description(&globals, CUSTOM_INSPECT_SYMBOL_DESCRIPTION)?;

        let options = FormatOptions {
            color,
            newline,
            object_filter,
            get_own_property_desc_fn,
            object_prototype,
            number_function,
            parse_float,
            parse_int,
            custom_inspect_symbol,
        };
        Ok(options)
    }
}

pub fn format_values<'js>(ctx: &Ctx<'js>, args: Rest<Value<'js>>, tty: bool) -> Result<String> {
    let mut result = String::with_capacity(64);
    let mut options = FormatOptions::new(ctx, tty, !AWS_LAMBDA_MODE.load(Ordering::Relaxed))?;
    format_values_internal(&mut result, ctx, args, &mut options)?;
    Ok(result)
}

fn log_std_out<'js>(ctx: &Ctx<'js>, args: Rest<Value<'js>>, level: LogLevel) -> Result<()> {
    write_log(stdout(), ctx, args, level)
}

pub(crate) fn log_std_err<'js>(
    ctx: &Ctx<'js>,
    args: Rest<Value<'js>>,
    level: LogLevel,
) -> Result<()> {
    write_log(stderr(), ctx, args, level)
}

#[allow(clippy::unused_io_amount)]
fn write_log<'js, T>(
    mut output: T,
    ctx: &Ctx<'js>,
    args: Rest<Value<'js>>,
    level: LogLevel,
) -> Result<()>
where
    T: Write + IsTerminal,
{
    let is_tty = output.is_terminal();
    let mut result = String::new();
    let mut is_lambda_mode = AWS_LAMBDA_MODE.load(Ordering::Relaxed);

    if is_lambda_mode && is_tty {
        is_lambda_mode = false;
    }

    if is_lambda_mode {
        let is_json_log_format = AWS_LAMBDA_JSON_LOG_FORMAT.load(Ordering::Relaxed);
        let max_log_level = AWS_LAMBDA_JSON_LOG_LEVEL.load(Ordering::Relaxed);
        if !write_lambda_log(
            ctx,
            &mut result,
            args,
            level,
            is_tty,
            is_json_log_format,
            max_log_level,
            TIME_FORMAT,
        )? {
            return Ok(());
        }
    } else {
        let mut options = FormatOptions::new(ctx, is_tty, true)?;
        format_values_internal(&mut result, ctx, args, &mut options)?;
    }

    result.push(NEWLINE);

    //we don't care if output is interrupted
    let _ = output.write_all(result.as_bytes());

    Ok(())
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn write_lambda_log<'js>(
    ctx: &Ctx<'js>,
    result: &mut String,
    args: Rest<Value<'js>>,
    level: LogLevel,
    is_tty: bool,
    is_json_log_format: bool,
    max_log_level: usize,
    time_format: &str,
) -> Result<bool> {
    let mut is_newline = true;

    if is_json_log_format && max_log_level < level.clone() as usize {
        //do not log if we don't meet the log level
        return Ok(false);
    }
    result.reserve(64);
    if !is_tty {
        is_newline = false;
    }

    let current_time: DateTime<Utc> = Utc::now();
    let formatted_time = current_time.format(time_format);
    let request_id = runtime_client::LAMBDA_REQUEST_ID.read().unwrap();

    if is_json_log_format {
        result.push('{');
        //time
        result.push_str("\"time\":\"");
        write!(result, "{}", formatted_time).unwrap();
        result.push_str("\",");

        //request id
        if let Some(id) = request_id.as_ref() {
            result.push_str("\"requestId\":\"");
            result.push_str(id);
            result.push_str("\",");
        }

        //level
        result.push_str("\"level\":\"");
        result.push_str(&level.to_string());
        result.push('\"');
    } else {
        write!(result, "{}", formatted_time).unwrap();
        result.push('\t');

        match request_id.as_ref() {
            Some(id) => result.push_str(id),
            None => result.push_str("n/a"),
        }

        result.push('\t');
        result.push_str(&level.to_string());
        result.push('\t');
    }

    if is_json_log_format {
        let mut values_string = String::with_capacity(64);

        if args.0.len() == 1 {
            let mut first_arg = unsafe { args.0.first().unwrap_unchecked() }.clone();

            if first_arg.is_error() || first_arg.is_exception() {
                if let Some(exception) = first_arg.as_exception() {
                    let obj = Object::new(ctx.clone())?;
                    obj.set("errorType", get_class_name(exception.as_value()))?;
                    if let Some(message) = exception.message() {
                        obj.set("errorMessage", message)?;
                    }
                    if let Some(stack) = exception.stack() {
                        let stack_object = Array::new(ctx.clone())?;

                        for (i, trace) in stack.split('\n').enumerate() {
                            stack_object.set(i, String::from(trace))?;
                        }
                        obj.set("stackTrace", stack_object)?;
                    }
                    first_arg = obj.into_value();
                }
            }
            if let Some(json_string) = json_stringify(ctx, first_arg)? {
                //message
                result.push(',');
                result.push_str("\"message\":");
                result.push_str(&json_string);
            }
        } else {
            //message
            result.push(',');
            result.push_str("\"message\":\"");

            let mut exception = None;

            let mut options = FormatOptions::new(ctx, is_tty, true)?;

            for arg in args.0.iter() {
                if arg.is_error() && exception.is_none() {
                    let exception_value = arg.clone();
                    exception = Some(exception_value.into_exception().unwrap());
                    break;
                }
            }

            format_values_internal(&mut values_string, ctx, args, &mut options)?;

            result.push_str(&escape_json(values_string.as_bytes()));
            result.push('\"');
            if let Some(exception) = exception {
                //error type
                result.push_str(",\"errorType\":\"");
                result
                    .push_str(&get_class_name(exception.as_value())?.unwrap_or("Exception".into()));
                result.push_str("\",");

                //error message
                if let Some(message) = exception.message() {
                    result.push_str("\"errorMessage\":\"");
                    result.push_str(&message);
                    result.push_str("\",");
                }

                //stack trace
                result.push_str("\"stackTrace\":[");
                let mut write_comma = false;
                if let Some(stack) = exception.stack() {
                    if !stack.is_empty() {
                        for trace in stack.split('\n') {
                            if write_comma {
                                result.push(',');
                            }
                            result.push('\"');
                            result.push_str(trace);
                            result.push('\"');
                            write_comma = true;
                        }
                    }
                }

                result.push(']');
            }
        }

        result.push('}');
    } else {
        let mut options = FormatOptions::new(ctx, is_tty && !is_json_log_format, is_newline)?;
        format_values_internal(result, ctx, args, &mut options)?;

        replace_newline_with_carriage_return(result);
    }

    Ok(true)
}

pub fn replace_newline_with_carriage_return(result: &mut str) {
    //OK since we just modify newlines
    let str_bytes = unsafe { result.as_bytes_mut() };

    //modify \n inside of strings, stacks etc
    let mut pos = 0;
    while let Some(index) = str_bytes[pos..].iter().position(|b| *b == b'\n') {
        str_bytes[pos + index] = b'\r';
        pos += index + 1; // Move the position after the found '\n'
    }
}

#[derive(rquickjs::class::Trace)]
#[rquickjs::class]
pub struct Console {}

#[rquickjs::methods(rename_all = "camelCase")]
impl Console {
    #[qjs(constructor)]
    pub fn new() -> Self {
        // We ignore the parameters for now since we don't support stream
        Self {}
    }

    pub fn log<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log(ctx, args)
    }
    pub fn clear(&self) {
        clear()
    }
    pub fn debug<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_debug(ctx, args)
    }
    pub fn info<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log(ctx, args)
    }
    pub fn trace<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_trace(ctx, args)
    }
    pub fn error<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_error(ctx, args)
    }
    pub fn warn<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_warn(ctx, args)
    }
    pub fn assert<'js>(
        &self,
        ctx: Ctx<'js>,
        expression: bool,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        log_assert(ctx, expression, args)
    }
}

#[cfg(test)]
mod tests {

    use rquickjs::{function::Rest, Error, IntoJs, Null, Object, Undefined, Value};

    use crate::{
        json::stringify::json_stringify_replacer_space,
        modules::console::{write_lambda_log, LogLevel},
        test_utils::utils::with_js_runtime,
    };

    #[tokio::test]
    async fn json_log_format() {
        with_js_runtime(|ctx| {
            let write_log = |args| {
                let mut result = String::new();

                write_lambda_log(
                    &ctx,
                    &mut result,
                    Rest(args),
                    LogLevel::Info,
                    false,
                    true,
                    LogLevel::Info as usize,
                    "",
                )?;


                //validate json
                ctx.json_parse(result.clone())?;

                Ok::<_, Error>(result)
            };

            assert_eq!(
                write_log(["Hello".into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":"Hello"}"#
            );

            assert_eq!(
                write_log([1.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":1}"#
            );

            assert_eq!(
                write_log([true.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":true}"#
            );

            assert_eq!(
                write_log([Undefined.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO"}"#
            );

            assert_eq!(
                write_log([Null.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":null}"#
            );

            let obj = Object::new(ctx.clone())?;
            obj.set("a", 1)?;
            obj.set("b", "Hello")?;

            assert_eq!(
                write_log([obj.clone().into_value()].into())?,
                r#"{"time":"","level":"INFO","message":{"a":1,"b":"Hello"}}"#
            );

            //validate second argument passed
            assert_eq!(
                write_log([obj.into_value(), true.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":"{\n  a: 1,\n  b: 'Hello'\n} true"}"#
            );

            //single error
            let e1:Value = ctx.eval(r#"new ReferenceError("some reference error")"#)?;
            assert_eq!(
                write_log([e1.clone()].into())?,
                r#"{"time":"","level":"INFO","message":{"errorType":"ReferenceError","errorMessage":"some reference error","stackTrace":["    at <eval> (eval_script:1:1)",""]}}"#
            );

             //validate many args with additional errors
            let e2:Value = ctx.eval(r#"new SyntaxError("some syntax error")"#)?;
            assert_eq!(
                write_log(["errors logged".into_js(&ctx)?, e1, e2].into())?,
                r#"{"time":"","level":"INFO","message":"errors logged ReferenceError: some reference error\n  at <eval> (eval_script:1:1) SyntaxError: some syntax error\n  at <eval> (eval_script:1:1)","errorType":"ReferenceError","errorMessage":"some reference error","stackTrace":["    at <eval> (eval_script:1:1)",""]}"#
            );

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn standard_log_format() {
        with_js_runtime(|ctx| {
            let write_log = |args| {
                let mut result = String::new();

                write_lambda_log(
                    &ctx,
                    &mut result,
                    Rest(args),
                    LogLevel::Info,
                    false,
                    false,
                    LogLevel::Info as usize,
                    "",
                )?;

                Ok::<_, Error>(result)
            };

            assert_eq!(
                write_log(["Hello".into_js(&ctx)?].into())?,
               "\tn/a\tINFO\tHello"
            );

            assert_eq!(
                write_log([1.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\t1"
            );

            assert_eq!(
                write_log([true.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\ttrue"
            );

            assert_eq!(
                write_log([Undefined.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\tundefined"
            );

            assert_eq!(
                write_log([Null.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\tnull"
            );

            let obj = Object::new(ctx.clone())?;
            obj.set("a", 1)?;
            obj.set("b", "Hello")?;

            assert_eq!(
                write_log([obj.clone().into_value()].into())?,
                 "\tn/a\tINFO\t{\r  a: 1,\r  b: 'Hello'\r}"
            );

            //validate second argument passed
            assert_eq!(
                write_log([obj.clone().into_value(), true.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\t{\r  a: 1,\r  b: 'Hello'\r} true"
            );

            //single error
            let e1:Value = ctx.eval(r#"new ReferenceError("some reference error")"#)?;
            assert_eq!(
                write_log([e1.clone()].into())?,
                "\tn/a\tINFO\tReferenceError: some reference error\r  at <eval> (eval_script:1:1)"
            );

             //validate many args with additional errors
            let e2:Value = ctx.eval(r#"new SyntaxError("some syntax error")"#)?;
            assert_eq!(
                write_log(["errors logged".into_js(&ctx)?, e1, e2].into())?,
                "\tn/a\tINFO\terrors logged ReferenceError: some reference error\r  at <eval> (eval_script:1:1) SyntaxError: some syntax error\r  at <eval> (eval_script:1:1)"
            );

            //newline replacement
            assert_eq!(
                write_log([
                    "event:".into_js(&ctx)?,
                    json_stringify_replacer_space(&ctx, obj.into_value(), None, Some("  ".into()))?.into_js(&ctx)?
                ].into())?,
               "\tn/a\tINFO\tevent: {\r  \"a\": 1,\r  \"b\": \"Hello\"\r}"
            );

            Ok(())
        })
        .await;
    }
}
