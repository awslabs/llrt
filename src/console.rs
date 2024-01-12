use std::fmt::Write as FormatWrite;
use std::{
    io::{stderr, stdout, IsTerminal, Write},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
};

use chrono::{DateTime, Utc};
use fxhash::FxHashSet;
use once_cell::sync::Lazy;
use rquickjs::{
    atom::PredefinedAtom,
    object::Filter,
    prelude::{Func, Rest, This},
    Ctx, Function, Object, Result, Type, Value,
};

use crate::{
    json::escape::escape_json,
    number::float_to_string,
    util::{get_class_name, ResultExt},
};

pub static AWS_LAMBDA_MODE: AtomicBool = AtomicBool::new(false);
pub static AWS_LAMBDA_JSON_LOG_FORMAT: AtomicBool = AtomicBool::new(false);
pub static AWS_LAMBDA_JSON_LOG_LEVEL: AtomicUsize = AtomicUsize::new(LogLevel::Info as usize);

//TODO The same principle can be added to JSON.stringify if indentation is space or tab
const SPACE_INDENTATION: &str = "                                                                                                                                                                                                                                                                ";
const SPACE_INDENTATION_LENGTH: usize = SPACE_INDENTATION.len();

#[inline(always)]
fn push_indentation(result: &mut String, depth: usize) {
    let width = depth * 2;
    if width <= SPACE_INDENTATION_LENGTH {
        result.push_str(&SPACE_INDENTATION[..width]);
        return;
    }
    let indentation = SPACE_INDENTATION.repeat(width / SPACE_INDENTATION_LENGTH);

    result.push_str(&indentation[..width]);
}

#[derive(Clone)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 4,
    Error = 8,
    Fatal = 16,
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

pub static LAMBDA_REQUEST_ID: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;

    console.set("log", Func::from(log))?;
    console.set("debug", Func::from(log_debug))?;
    console.set("info", Func::from(log))?;
    console.set("trace", Func::from(log_trace))?;
    console.set("error", Func::from(log_error))?;
    console.set("warn", Func::from(log_warn))?;
    console.set("assert", Func::from(log_assert))?;
    console.set("__format", Func::from(js_format))?;
    console.set("__formatPlain", Func::from(format_plain))?;

    globals.set("console", console)?;

    Ok(())
}

const NEWLINE_LOOKUP: [char; 2] = [NEWLINE, CARRIAGE_RETURN];
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_BLACK: &str = "\x1b[30m";
const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_YELLOW: &str = "\x1b[33m";
const COLOR_PURPLE: &str = "\x1b[35m";
const COLOR_CYAN: &str = "\x1b[36m";

const NEWLINE: char = '\n';
const CARRIAGE_RETURN: char = '\r';
const SPACING: char = ' ';
const SINGLE_QUOTE: char = '\'';
const SEPARATOR: char = ',';
const CIRCULAR: &str = "[Circular]";
const OBJECT: &str = "[Object]";

fn stringify_primitive<'js>(
    result: &mut String,
    _ctx: &Ctx<'js>,
    value: &Value<'js>,
    value_type: Type,
    tty: bool,
) -> Result<()> {
    let mut has_color = false;
    if tty {
        has_color = true;
        match value_type {
            Type::Undefined => result.push_str(COLOR_BLACK),
            Type::Int | Type::Float | Type::Bool => result.push_str(COLOR_YELLOW),
            Type::Symbol => result.push_str(COLOR_GREEN),
            _ => has_color = false,
        }
    }

    match value_type {
        Type::Uninitialized | Type::Null => result.push_str("null"),
        Type::Undefined => result.push_str("undefined"),
        Type::Bool => result.push_str(if value.as_bool().unwrap() {
            "true"
        } else {
            "false"
        }),
        Type::Int => {
            let mut buffer = itoa::Buffer::new();
            result.push_str(buffer.format(value.as_int().unwrap()))
        }
        Type::Float => {
            let mut buffer = ryu::Buffer::new();
            result.push_str(
                match float_to_string(&mut buffer, value.as_float().unwrap()) {
                    Ok(v) => v,
                    Err(v) => v,
                },
            )
        }
        Type::String => result.push_str(&value.as_string().unwrap().to_string()?),
        Type::Symbol => {
            let description = value.as_symbol().unwrap().description()?;
            let description = description.to_string()?;
            result.push_str("Symbol(");
            if description != "undefined" {
                result.push_str(&description);
            }
            result.push(')');
        }
        _ => {}
    }
    if has_color {
        result.push_str(COLOR_RESET);
    }
    Ok(())
}

struct StringifyItem<'js> {
    value: Option<Value<'js>>,
    depth: usize,
    key: Option<String>,
    end: Option<char>,
    expand: bool,
}

#[inline(always)]
fn is_primitive_like_or_void(typeof_value: Type) -> bool {
    matches!(
        typeof_value,
        Type::Uninitialized
            | Type::Undefined
            | Type::Null
            | Type::Bool
            | Type::Int
            | Type::Float
            | Type::String
            | Type::Symbol
            | Type::Unknown
    )
}

fn stringify_value<'js>(
    result: &mut String,
    ctx: &Ctx<'js>,
    obj: Value<'js>,
    tty: bool,
    newline_char: char,
) -> Result<()> {
    let obj_type = obj.type_of();

    if is_primitive_like_or_void(obj_type) {
        stringify_primitive(result, ctx, &obj, obj_type, tty)?;
        return Ok(());
    }

    //let obj = obj.to_owned();
    let default_obj = Object::new(ctx.clone())?;
    let object_ctor: Object = default_obj.get(PredefinedAtom::Constructor)?;
    let object_prototype = default_obj
        .get_prototype()
        .ok_or("Can't get prototype")
        .or_throw(ctx)?;
    let get_own_property_desc_fn: Function =
        object_ctor.get(PredefinedAtom::GetOwnPropertyDescriptor)?;

    let mut stack = Vec::<StringifyItem>::with_capacity(32);

    let mut visited = FxHashSet::default();

    stack.push(StringifyItem {
        value: Some(obj),
        depth: 0,
        key: None,
        end: None,
        expand: false,
    });

    while let Some(StringifyItem {
        value,
        depth,
        key,
        end,
        expand,
    }) = stack.pop()
    {
        if let Some(end) = end {
            if expand {
                result.push(newline_char);
                if !stack.is_empty() {
                    push_indentation(result, depth);
                }
            }
            result.push(end);
        } else {
            if expand {
                result.push(newline_char);
                push_indentation(result, depth);
            }
            if let Some(key) = key {
                result.push_str(&key);
                result.push(':');
                result.push(SPACING);
            }

            if let Some(value) = value {
                let typeof_value = value.type_of();

                if is_primitive_like_or_void(typeof_value) {
                    if typeof_value == Type::String {
                        if tty {
                            result.push_str(COLOR_GREEN)
                        }
                        result.push(SINGLE_QUOTE);
                        result.push_str(&value.as_string().unwrap().to_string().unwrap());
                        result.push(SINGLE_QUOTE);
                        if tty {
                            result.push_str(COLOR_RESET)
                        }
                    } else {
                        stringify_primitive(result, ctx, &value, typeof_value, tty)?;
                    }
                } else if typeof_value == Type::Function || typeof_value == Type::Constructor {
                    if tty {
                        result.push_str(COLOR_CYAN);
                    }

                    let obj = value.as_object().unwrap();
                    let mut name: String =
                        obj.get(PredefinedAtom::Name).unwrap_or(String::from(""));
                    if name.is_empty() {
                        name.push_str("(anonymous)")
                    }

                    let mut is_class = false;
                    if obj.contains_key(PredefinedAtom::Prototype)? {
                        let desc: Object = get_own_property_desc_fn.call((value, "prototype"))?;
                        let writable: bool = desc.get(PredefinedAtom::Writable)?;
                        is_class = !writable;
                    }

                    result.push_str(if is_class { "[class: " } else { "[function: " });
                    result.push_str(&name);
                    result.push(']');

                    if tty {
                        result.push_str(COLOR_RESET);
                    }
                } else if typeof_value == Type::Array
                    || typeof_value == Type::Object
                    || typeof_value == Type::Exception
                {
                    //let ptr = unsafe { value.as_raw().u.ptr } as usize;
                    if visited.contains(&fxhash::hash(&value)) {
                        if tty {
                            result.push_str(COLOR_CYAN);
                        }
                        result.push_str(CIRCULAR);
                        if tty {
                            result.push_str(COLOR_RESET);
                        }
                    } else {
                        visited.insert(fxhash::hash(&value));
                        let mut class_name = None;
                        let mut is_object_like = false;
                        if value.is_error() {
                            let obj = value.as_object().unwrap();
                            let name: String = obj.get(PredefinedAtom::Name).unwrap();
                            let message: String = obj.get(PredefinedAtom::Message).unwrap();
                            let stack: Result<String> = obj.get(PredefinedAtom::Stack);
                            result.push_str(&name);
                            result.push_str(": ");
                            result.push_str(&message);
                            result.push(newline_char);
                            if tty {
                                result.push_str(COLOR_BLACK);
                            }
                            if let Ok(stack) = stack {
                                stack.trim().split('\n').for_each(|line| {
                                    push_indentation(result, depth + 1);
                                    result.push_str(line);
                                });
                            }
                            if tty {
                                result.push_str(COLOR_RESET);
                            }
                        } else if typeof_value == Type::Object {
                            let cl = get_class_name(&value)?;
                            match cl.as_deref() {
                                Some("Date") => {
                                    if tty {
                                        result.push_str(COLOR_PURPLE);
                                    }
                                    let this = value.as_object().unwrap().to_owned();
                                    let iso_fn: Function =
                                        value.as_object().unwrap().get("toISOString").unwrap();

                                    let str: String = iso_fn.call((This(this),)).unwrap();
                                    result.push_str(&str);
                                    if tty {
                                        result.push_str(COLOR_RESET);
                                    }
                                }
                                Some("Promise") => {
                                    result.push_str("Promise {}");
                                }
                                None | Some("") | Some("Object") => {
                                    is_object_like = true;
                                }
                                _ => {
                                    class_name = cl;
                                    is_object_like = true;
                                }
                            }
                        } else {
                            is_object_like = true;
                        }

                        if is_object_like {
                            if depth < 4 {
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
                                    )
                                }

                                let obj = value.as_object().unwrap();

                                let is_array = is_typed_array || obj.is_array();

                                result.push(if is_array { '[' } else { '{' });

                                let mut keys = obj.keys().rev();
                                let mut filter_functions = false;

                                if !is_array && keys.len() == 0 {
                                    if let Some(proto) = obj.get_prototype() {
                                        if proto != object_prototype {
                                            keys = proto
                                                .own_keys(Filter::new().private().string().symbol())
                                                .rev();
                                            filter_functions = true;
                                        }
                                    }
                                }

                                stack.push(StringifyItem {
                                    value: None,
                                    depth,
                                    key: None,
                                    end: Some(if is_array { ']' } else { '}' }),
                                    expand: false,
                                });

                                let mut i = 0;
                                let stack_len = stack.len();
                                let mut expand_stack = false;
                                let mut has_value = false;
                                keys.for_each(|key: Result<String>| {
                                    if let Ok(key) = key {
                                        let value: Value = obj.get(&key).unwrap();
                                        if !(value.is_function() && filter_functions) {
                                            has_value = true;
                                            let is_error = value.is_error();
                                            let is_obj = value.is_object() && depth < 2;
                                            if !expand_stack && (is_error || is_obj) {
                                                expand_stack = true;
                                            }
                                            stack.push(StringifyItem {
                                                value: Some(value),
                                                depth: depth + 1,
                                                expand: false,
                                                key: if is_array { None } else { Some(key) },
                                                end: None,
                                            });
                                            i += 1;
                                        }
                                    }
                                });

                                if expand_stack {
                                    for item in
                                        stack.iter_mut().take(stack_len + i).skip(stack_len - 1)
                                    {
                                        item.expand = true
                                    }
                                }

                                if has_value && !expand_stack {
                                    result.push(SPACING);
                                }
                            } else {
                                if tty {
                                    result.push_str(COLOR_CYAN);
                                }
                                result.push_str(OBJECT);
                                if tty {
                                    result.push_str(COLOR_RESET);
                                }
                                if stack.last().and_then(|n| n.end).is_none() {
                                    result.push(SEPARATOR);
                                }
                                result.push(SPACING);
                            }
                            continue;
                        };
                    }
                }
            }
        }

        if !stack.is_empty() {
            let next = stack.last().unwrap();
            let next_is_end = next.end.is_some();
            let next_expand = next.expand;
            if !next_is_end {
                result.push(SEPARATOR);
            }

            if !next_expand {
                result.push(SPACING);
            }
        }
    }

    Ok(())
}

fn log_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_err(ctx, args, LogLevel::Error)
}

fn log_warn<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_err(ctx, args, LogLevel::Warn)
}

fn log_debug<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_out(ctx, args, LogLevel::Debug)
}

fn log_trace<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_out(ctx, args, LogLevel::Trace)
}

fn log_assert<'js>(ctx: Ctx<'js>, expression: bool, args: Rest<Value<'js>>) -> Result<()> {
    if !expression {
        log_error(ctx, args)?;
    }

    Ok(())
}

fn log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    log_std_out(ctx, args, LogLevel::Info)
}

fn js_format<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<String> {
    format(&ctx, args)
}

pub fn format<'js>(ctx: &Ctx<'js>, args: Rest<Value<'js>>) -> Result<String> {
    format_values(ctx, args, stdout().is_terminal())
}

fn format_plain<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<String> {
    format_values(&ctx, args, false)
}

fn format_values_internal<'js>(
    result: &mut String,
    ctx: &Ctx<'js>,
    args: Rest<Value<'js>>,
    tty: bool,
    newline_char: char,
) -> Result<()> {
    for arg in args.0.into_iter() {
        stringify_value(result, ctx, arg, tty, newline_char)?;
    }
    Ok(())
}

#[inline]
fn format_values<'js>(ctx: &Ctx<'js>, args: Rest<Value<'js>>, tty: bool) -> Result<String> {
    let mut result = String::with_capacity(64);
    let newline_char = NEWLINE_LOOKUP[AWS_LAMBDA_MODE.load(Ordering::Relaxed) as usize];
    format_values_internal(&mut result, ctx, args, tty, newline_char)?;
    Ok(result)
}

fn log_std_out<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>, level: LogLevel) -> Result<()> {
    write_log(stdout(), ctx, args, level)
}

#[inline(always)]
#[allow(clippy::unused_io_amount)]
fn write_log<'js, T>(
    mut output: T,
    ctx: Ctx<'js>,
    args: Rest<Value<'js>>,
    level: LogLevel,
) -> Result<()>
where
    T: Write + IsTerminal,
{
    let mut is_tty = output.is_terminal();
    let mut result;
    let is_json_log_format;
    let mut newline_char = NEWLINE;
    if AWS_LAMBDA_MODE.load(Ordering::Relaxed) {
        is_json_log_format = AWS_LAMBDA_JSON_LOG_FORMAT.load(Ordering::Relaxed);

        if is_json_log_format
            && AWS_LAMBDA_JSON_LOG_LEVEL.load(Ordering::Relaxed) < level.clone() as usize
        {
            //do not log if we don't meet the log level
            return Ok(());
        }
        if !is_tty {
            newline_char = CARRIAGE_RETURN;
        }

        result = String::with_capacity(64);

        let current_time: DateTime<Utc> = Utc::now();
        let formatted_time = current_time.format("%Y-%m-%dT%H:%M:%S%.3fZ");
        let request_id = LAMBDA_REQUEST_ID.lock().unwrap().clone();

        if is_json_log_format {
            is_tty = false;
            result.push('{');
            result.push(newline_char);
            result.push_str("  \"time\": \"");
            write!(&mut result, "{}", formatted_time).unwrap();
            result.push_str("\",");
            result.push(newline_char);

            if let Some(id) = request_id {
                result.push_str("  \"requestId\": \"");
                result.push_str(&id);
                result.push_str("\",");
                result.push(newline_char);
            }

            result.push_str("  \"level\": \"");
            result.push_str(&level.to_string());
            result.push_str("\",");
            result.push(newline_char);
            result.push_str("  \"message\": \"");
        } else {
            write!(&mut result, "{}", formatted_time).unwrap();
            result.push('\t');

            match request_id {
                Some(id) => result.push_str(&id),
                None => result.push_str("n/a"),
            }

            result.push('\t');
            result.push_str(&level.to_string());
            result.push('\t');
        }
    } else {
        result = String::with_capacity(64);
        is_json_log_format = false;
    }

    if is_json_log_format {
        let mut values_string = String::with_capacity(64);
        format_values_internal(&mut values_string, &ctx, args, is_tty, newline_char)?;
        result.push_str(&escape_json(values_string.as_bytes()));
        result.push('\"');
        result.push(newline_char);
        result.push('}');
    } else {
        format_values_internal(&mut result, &ctx, args, is_tty, newline_char)?;
    }

    result.push(NEWLINE);

    //we don't care if output is interrupted
    let _ = output.write_all(result.as_bytes());

    Ok(())
}

fn log_std_err<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>, level: LogLevel) -> Result<()> {
    write_log(stderr(), ctx, args, level)
}
