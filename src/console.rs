use std::{
    collections::HashSet,
    env,
    io::{self, stderr, stdout, IsTerminal, Write},
};

use rquickjs::{
    atom::PredefinedAtom,
    object::Filter,
    prelude::{Func, Rest, This},
    Ctx, Function, Object, Result, Type, Value,
};

use crate::util::{get_class_name, ResultExt};

pub const ENV_LLRT_CONSOLE_NEWLINE_RETURN: &str = "LLRT_CONSOLE_NEWLINE_RETURN";

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let console = Object::new(ctx.clone())?;

    console.set("log", Func::from(log))?;
    console.set("debug", Func::from(log))?;
    console.set("info", Func::from(log))?;
    console.set("info", Func::from(log))?;
    console.set("error", Func::from(log_error))?;
    console.set("warn", Func::from(log_error))?;
    console.set("assert", Func::from(log_assert))?;
    console.set("__format", Func::from(js_format))?;
    console.set("__formatPlain", Func::from(format_plain))?;

    globals.set("console", console)?;

    Ok(())
}

const RESET: &str = "\x1b[0m";
const NEWLINE: char = '\n';
const CARRIAGE_RETURN: char = '\r';
const SPACING: char = ' ';
const SEPARATOR: char = ',';

fn stringify_primitive<'js>(ctx: &Ctx<'js>, obj: &Value<'js>, obj_type: Type, tty: bool) -> String {
    let mut string = String::with_capacity(10);
    let mut has_color = false;
    if tty {
        if let Some(c) = get_color(obj_type) {
            string.push_str(c);
            has_color = true;
        }
    }
    string.push_str(&match obj_type {
        Type::Uninitialized | Type::Null => String::from("null"),
        Type::Undefined => String::from("undefined"),
        Type::Bool => String::from(if obj.as_bool().unwrap() {
            "true"
        } else {
            "false"
        }),
        Type::Int => obj.as_int().unwrap().to_string(),
        Type::Float => obj.as_float().unwrap().to_string(),
        Type::String => obj.as_string().unwrap().to_string().unwrap(),
        Type::Symbol => {
            let ctor: Function = ctx.globals().get(PredefinedAtom::String).unwrap();
            let string: String = ctor.call((obj,)).unwrap();
            string
        }
        _ => String::from(""),
    });
    if has_color {
        string.push_str("\x1b[0m");
    }
    string
}

fn get_color(obj_type: Type) -> Option<&'static str> {
    match obj_type {
        Type::Undefined => Some("\x1b[30m"),
        Type::Int | Type::Float | Type::Bool => Some("\x1b[33m"),
        Type::Symbol => Some("\x1b[32m"),
        _ => None,
    }
}

struct StringifyItem<'js> {
    value: Option<Value<'js>>,
    depth: usize,
    key: Option<String>,
    end: Option<char>,
    expand: bool,
}

fn is_primitive_like_or_void(typeof_value: Type) -> bool {
    (typeof_value != Type::Object
        && typeof_value != Type::Array
        && typeof_value != Type::Function
        && typeof_value != Type::Constructor
        && typeof_value != Type::Exception)
        || typeof_value.is_void()
}

#[allow(clippy::mutable_key_type)]
fn stringify_value<'js>(
    ctx: &Ctx<'js>,
    obj: &Value<'js>,
    tty: bool,
    use_carriage_return: bool,
) -> Result<String> {
    let obj_type = obj.type_of();

    let newline_deliminator = if use_carriage_return {
        CARRIAGE_RETURN
    } else {
        NEWLINE
    };

    if is_primitive_like_or_void(obj_type) {
        return Ok(stringify_primitive(ctx, obj, obj_type, tty));
    }

    let obj = obj.to_owned();
    let default_obj = Object::new(ctx.clone())?;
    let object_ctor: Object = default_obj.get(PredefinedAtom::Constructor)?;
    let object_prototype = default_obj
        .get_prototype()
        .ok_or("Can't get prototype")
        .or_throw(ctx)?;
    let get_own_property_desc_fn: Function =
        object_ctor.get(PredefinedAtom::GetOwnPropertyDescriptor)?;

    let mut stack = Vec::<StringifyItem>::with_capacity(32);

    let mut visited = HashSet::new();

    stack.push(StringifyItem {
        value: Some(obj),
        depth: 0,
        key: None,
        end: None,
        expand: false,
    });

    let mut result = String::with_capacity(64);

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
                result.push(newline_deliminator);
                if !stack.is_empty() {
                    result.push_str(&SPACING.to_string().repeat(2 * depth));
                }
            }
            result.push(end);
        } else {
            if expand {
                result.push(newline_deliminator);
                result.push_str(&SPACING.to_string().repeat(2 * depth));
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
                            result.push_str("\x1b[32m")
                        }
                        result.push('\'');
                        result.push_str(&value.as_string().unwrap().to_string().unwrap());
                        result.push('\'');
                        if tty {
                            result.push_str(RESET)
                        }
                    } else {
                        result.push_str(&stringify_primitive(ctx, &value, typeof_value, tty));
                    }
                } else if typeof_value == Type::Function || typeof_value == Type::Constructor {
                    if tty {
                        result.push_str("\x1b[36m");
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
                        result.push_str(RESET);
                    }
                } else if typeof_value == Type::Array
                    || typeof_value == Type::Object
                    || typeof_value == Type::Exception
                {
                    if visited.contains(&value) {
                        if tty {
                            result.push_str("\x1b[36m");
                        }
                        result.push_str("[Circular]");
                        if tty {
                            result.push_str(RESET);
                        }
                    } else {
                        visited.insert(value.clone());
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
                            result.push(newline_deliminator);
                            if tty {
                                result.push_str("\x1b[30m");
                            }
                            if let Ok(stack) = stack {
                                stack.trim().split('\n').for_each(|l| {
                                    result.push_str(&SPACING.to_string().repeat(2 * (depth + 1)));
                                    result.push_str(l);
                                });
                            }
                            if tty {
                                result.push_str(RESET);
                            }
                        } else if typeof_value == Type::Object {
                            let cl = get_class_name(&value)?;
                            match cl.as_deref() {
                                Some("Date") => {
                                    if tty {
                                        result.push_str("\x1b[35m");
                                    }
                                    let this = value.as_object().unwrap().to_owned();
                                    let iso_fn: Function =
                                        value.as_object().unwrap().get("toISOString").unwrap();

                                    let str: String = iso_fn.call((This(this),)).unwrap();
                                    result.push_str(&str);
                                    if tty {
                                        result.push_str(RESET);
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
                                    result.push(' ');
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
                                    result.push_str("\x1b[36m");
                                }
                                result.push_str("[Object]");
                                if tty {
                                    result.push_str(RESET);
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

    Ok(result)
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

fn format_values<'js>(ctx: &Ctx<'js>, args: Rest<Value<'js>>, tty: bool) -> Result<String> {
    let use_carriage_return = env::var(ENV_LLRT_CONSOLE_NEWLINE_RETURN).is_ok();
    args.iter()
        .map(|arg| stringify_value(ctx, arg, tty, use_carriage_return))
        .collect::<Result<Vec<String>>>()
        .map(|v| v.join(" "))
}

#[allow(clippy::unused_io_amount)]
fn log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    let str = format_values(&ctx, args, stdout().is_terminal())?;
    let mut stdout = io::stdout();
    stdout.write_all(str.as_bytes()).unwrap();
    stdout.write(b"\n").unwrap();
    Ok(())
}

#[allow(clippy::unused_io_amount)]
fn log_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    let str = format_values(&ctx, args, stderr().is_terminal())?;
    let mut stderr = io::stderr();
    stderr.write_all(str.as_bytes()).unwrap();
    stderr.write(b"\n").unwrap();
    Ok(())
}

fn log_assert<'js>(ctx: Ctx<'js>, expression: bool, args: Rest<Value<'js>>) -> Result<()> {
    if !expression {
        log_error(ctx, args)?;
    }

    Ok(())
}
