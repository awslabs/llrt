// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashSet,
    io::{stdout, IsTerminal},
    ops::Deref,
};

use llrt_json::stringify::json_stringify;
use llrt_numbers::float_to_string;
use llrt_utils::{
    class::get_class_name,
    error::ErrorExtensions,
    hash,
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    atom::PredefinedAtom, function::This, object::Filter, prelude::Rest, promise::PromiseState,
    Coerced, Ctx, Error, Function, Object, Result, Symbol, Type, Value,
};

pub const NEWLINE: char = '\n';
pub const CARRIAGE_RETURN: char = '\r';
const SPACING: char = ' ';
const CIRCULAR: &str = "[Circular]";
pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

const MAX_INDENTATION_LEVEL: usize = 4;
const MAX_EXPANSION_DEPTH: usize = 4;
const INDENTATION_LOOKUP: [&str; MAX_INDENTATION_LEVEL + 1] =
    ["", "  ", "    ", "        ", "                "];

macro_rules! ascii_colors {
    ( $( $name:ident => $value:expr ),* ) => {
        #[derive(Debug, Clone, Copy)]
        pub enum Color {
            $(
                $name,
            )*
        }

        impl AsRef<str> for Color {
            fn as_ref(&self) -> &str {
                match self {
                    $(
                        Color::$name => concat!("\x1b[", stringify!($value), "m"),
                    )*
                }
            }
        }
    }
}

impl Color {
    #[inline(always)]
    fn push(self, value: &mut String) {
        value.push_str(self.as_ref())
    }

    #[inline(always)]
    fn reset(value: &mut String) {
        value.push_str(Color::RESET.as_ref())
    }
}

// Define the colors
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
    #[allow(clippy::inherent_to_string)]
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
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

pub struct FormatOptions<'js> {
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
    pub fn new(ctx: &Ctx<'js>, color: bool, newline: bool) -> Result<Self> {
        let primordials = BasePrimordials::get(ctx)?;

        let get_own_property_desc_fn = primordials.function_get_own_property_descriptor.clone();
        let object_prototype = primordials.prototype_object.clone();

        let parse_float = primordials.function_parse_float.clone();
        let parse_int = primordials.function_parse_int.clone();

        let object_filter = Filter::new().private().string().symbol();

        let custom_inspect_symbol = primordials.symbol_custom_inspect.clone();
        let number_function = primordials.constructor_number.deref().clone();

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

pub fn format_plain<'js>(ctx: Ctx<'js>, newline: bool, args: Rest<Value<'js>>) -> Result<String> {
    format_values(&ctx, args, false, newline)
}

pub fn format<'js>(ctx: &Ctx<'js>, newline: bool, args: Rest<Value<'js>>) -> Result<String> {
    format_values(ctx, args, stdout().is_terminal(), newline)
}

pub fn format_values<'js>(
    ctx: &Ctx<'js>,
    args: Rest<Value<'js>>,
    tty: bool,
    newline: bool,
) -> Result<String> {
    let mut result = String::with_capacity(64);
    let mut options = FormatOptions::new(ctx, tty, newline)?;
    build_formatted_string(&mut result, ctx, args, &mut options)?;
    Ok(result)
}

pub fn build_formatted_string<'js>(
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

#[inline(always)]
fn format_raw<'js>(
    result: &mut String,
    value: Value<'js>,
    options: &FormatOptions<'js>,
) -> Result<()> {
    format_raw_inner(result, value, options, &mut HashSet::default(), 0)?;
    Ok(())
}

fn format_raw_inner<'js>(
    result: &mut String,
    value: Value<'js>,
    options: &FormatOptions<'js>,
    visited: &mut HashSet<usize>,
    depth: usize,
) -> Result<()> {
    let value_type = value.type_of();

    let color_enabled = options.color;
    let is_root = depth == 0;

    match value_type {
        Type::Uninitialized | Type::Null => {
            if color_enabled {
                Color::BOLD.push(result);
            }
            result.push_str("null")
        },
        Type::Undefined => {
            if color_enabled {
                Color::BLACK.push(result);
            }
            result.push_str("undefined")
        },
        Type::Bool => {
            if color_enabled {
                Color::YELLOW.push(result);
            }
            const BOOL_STRINGS: [&str; 2] = ["false", "true"];
            result.push_str(BOOL_STRINGS[unsafe { value.as_bool().unwrap_unchecked() } as usize]);
        },
        Type::BigInt => {
            if color_enabled {
                Color::YELLOW.push(result);
            }
            let mut buffer = itoa::Buffer::new();
            let big_int = unsafe { value.as_big_int().unwrap_unchecked() };
            result.push_str(buffer.format(big_int.clone().to_i64().unwrap()));
            result.push('n');
        },
        Type::Int => {
            if color_enabled {
                Color::YELLOW.push(result);
            }
            let mut buffer = itoa::Buffer::new();
            result.push_str(buffer.format(unsafe { value.as_int().unwrap_unchecked() }));
        },
        Type::Float => {
            if color_enabled {
                Color::YELLOW.push(result);
            }
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
                !is_root,
                color_enabled,
            );
        },
        Type::Symbol => {
            if color_enabled {
                Color::YELLOW.push(result);
            }
            let description = unsafe { value.as_symbol().unwrap_unchecked() }.description()?;
            result.push_str("Symbol(");
            result.push_str(&unsafe { description.get::<String>().unwrap_unchecked() });
            result.push(')');
        },
        Type::Function | Type::Constructor => {
            if color_enabled {
                Color::CYAN.push(result);
            }
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

            result.push_str(if is_class { "[class: " } else { "[function: " });
            result.push_str(&name);
            result.push(']');
        },
        Type::Promise => {
            let promise = unsafe { value.as_promise().unwrap_unchecked() };
            let state = promise.state();
            result.push_str("Promise {");
            let is_pending = matches!(state, PromiseState::Pending);
            let apply_indentation = depth < 2 && !is_pending;
            write_sep(result, false, apply_indentation, options.newline);
            if apply_indentation {
                push_indentation(result, depth + 1);
            }

            match state {
                PromiseState::Pending => {
                    if color_enabled {
                        Color::CYAN.push(result);
                    }
                    result.push_str("<pending>");
                    if color_enabled {
                        Color::reset(result);
                    }
                },
                PromiseState::Resolved => {
                    let value: Value = unsafe { promise.result().unwrap_unchecked() }?;
                    format_raw_inner(result, value, options, visited, depth + 1)?;
                },
                PromiseState::Rejected => {
                    let value: Error =
                        unsafe { promise.result::<Value>().unwrap_unchecked() }.unwrap_err();
                    let value = value.into_value(promise.ctx())?;
                    if color_enabled {
                        Color::RED.push(result);
                    }
                    result.push_str("<rejected> ");
                    if color_enabled {
                        Color::reset(result);
                    }
                    format_raw_inner(result, value, options, visited, depth + 1)?;
                },
            }
            write_sep(result, false, apply_indentation, options.newline);
            if apply_indentation {
                push_indentation(result, depth);
            }

            result.push('}');
            return Ok(());
        },
        Type::Array | Type::Object | Type::Exception => {
            let hash = hash::default_hash(&value);
            if visited.contains(&hash) {
                if color_enabled {
                    Color::CYAN.push(result);
                }
                result.push_str(CIRCULAR);
                if color_enabled {
                    Color::reset(result);
                }
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
                if color_enabled {
                    Color::BLACK.push(result);
                }

                if let Ok(stack) = stack {
                    for line in stack.trim().split('\n') {
                        result.push(if options.newline {
                            NEWLINE
                        } else {
                            CARRIAGE_RETURN
                        });
                        push_indentation(result, depth + 1);
                        result.push_str(line);
                    }
                }
                if color_enabled {
                    Color::reset(result);
                }
                return Ok(());
            }

            let mut class_name: Option<String> = None;
            let mut is_object = false;
            if value_type == Type::Object {
                is_object = true;
                class_name = get_class_name(&value)?;
                match class_name.as_deref() {
                    Some("Date") => {
                        if color_enabled {
                            Color::MAGENTA.push(result);
                        }
                        let iso_fn: Function = obj.get("toISOString").unwrap();
                        let str: String = iso_fn.call((This(value),))?;
                        result.push_str(&str);
                        if color_enabled {
                            Color::reset(result);
                        }
                        return Ok(());
                    },
                    Some("RegExp") => {
                        if color_enabled {
                            Color::RED.push(result);
                        }
                        let source: String = obj.get("source")?;
                        let flags: String = obj.get("flags")?;
                        result.push('/');
                        result.push_str(&source);
                        result.push('/');
                        result.push_str(&flags);
                        if color_enabled {
                            Color::reset(result);
                        }
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

                    //TODO fix when quickjs-ng exposes these types
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
                        color_enabled,
                        is_array,
                    );
                }

                write_object(
                    result,
                    obj,
                    options,
                    visited,
                    depth,
                    color_enabled,
                    is_array,
                )?;
            } else {
                if color_enabled {
                    Color::CYAN.push(result);
                }
                result.push_str(if is_object { "[Object]" } else { "[Array]" });
            }
        },
        _ => {},
    }

    if color_enabled {
        Color::reset(result);
    }

    Ok(())
}

fn format_raw_string(result: &mut String, value: String, options: &FormatOptions<'_>) {
    format_raw_string_inner(result, value, false, options.color);
}

fn format_raw_string_inner(result: &mut String, value: String, quoted: bool, color_enabled: bool) {
    if quoted {
        if color_enabled {
            Color::GREEN.push(result);
        }
        result.push('\'');
    }
    result.push_str(&value);
    if quoted {
        result.push('\'');
    }
}

fn write_object<'js>(
    result: &mut String,
    obj: &Object<'js>,
    options: &FormatOptions<'js>,
    visited: &mut HashSet<usize>,
    depth: usize,
    color_enabled: bool,
    is_array: bool,
) -> Result<()> {
    result.push(if is_array { '[' } else { '{' });

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
    let apply_indentation = !is_array && depth < 2;

    let mut first = false;
    let mut numeric_key;
    let length = keys.len();
    for (i, key) in keys.flatten().enumerate() {
        let value: Value = obj.get::<&String, _>(&key)?;
        if !(value.is_function() && filter_functions) {
            numeric_key = key.parse::<f64>().is_ok();
            write_sep(result, first, apply_indentation, options.newline);

            if apply_indentation {
                push_indentation(result, depth + 1);
            }
            if depth > MAX_INDENTATION_LEVEL - 1 {
                result.push(SPACING);
            }
            if !is_array {
                format_raw_string_inner(result, key, numeric_key, numeric_key & color_enabled);
                if numeric_key && color_enabled {
                    Color::reset(result);
                }

                result.push(':');
                result.push(SPACING);
            }

            format_raw_inner(result, value, options, visited, depth + 1)?;
            first = true;
            if i > 99 {
                result.push_str("... ");
                let mut buffer = itoa::Buffer::new();
                result.push_str(buffer.format(length - i));
                result.push_str(" more items");
                break;
            }
        }
    }
    if first {
        if apply_indentation {
            result.push(if options.newline {
                NEWLINE
            } else {
                CARRIAGE_RETURN
            });
            push_indentation(result, depth);
        } else {
            result.push(SPACING);
        }
    }

    result.push(if is_array { ']' } else { '}' });

    Ok(())
}

#[inline(always)]
fn write_sep(result: &mut String, add_comma: bool, has_indentation: bool, newline: bool) {
    if add_comma {
        result.push(',');
    }

    if has_indentation {
        if newline {
            result.push('\n');
        } else {
            result.push('\r')
        }
    } else {
        result.push(' ');
    }
}

#[inline(always)]
fn push_indentation(result: &mut String, depth: usize) {
    result.push_str(INDENTATION_LOOKUP[depth]);
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
