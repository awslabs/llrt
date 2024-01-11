use std::collections::HashSet;

use rquickjs::{
    atom::PredefinedAtom, function::This, Ctx, Exception, Function, Object, Result, Type, Value,
};

use crate::json::escape::escape_json_string;

const CIRCULAR_REF_DETECTION_DEPTH: usize = 20;

struct IterationContext<'a, 'js> {
    ctx: &'a Ctx<'js>,
    result: &'a mut String,
    value: &'a Value<'js>,
    depth: usize,
    indentation: Option<&'a str>,
    key: Option<&'a str>,
    index: Option<usize>,
    parent: Option<&'a Object<'js>>,
    ancestors: &'a mut Vec<(usize, String)>,
    replacer_fn: Option<&'a Function<'js>>,
    include_keys_replacer: Option<&'a HashSet<String>>,
}

#[allow(dead_code)]
pub fn json_stringify<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Option<String>> {
    json_stringify_replacer_space(ctx, value, None, None)
}

#[allow(dead_code)]
pub fn json_stringify_replacer<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    replacer: Option<Value<'js>>,
) -> Result<Option<String>> {
    json_stringify_replacer_space(ctx, value, replacer, None)
}

pub fn json_stringify_replacer_space<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    replacer: Option<Value<'js>>,
    indentation: Option<String>,
) -> Result<Option<String>> {
    let mut result = String::with_capacity(128);
    let mut replacer_fn = None;
    let mut include_keys_replacer = None;

    let tmp_function;

    if let Some(replacer) = replacer {
        if let Some(function) = replacer.as_function() {
            tmp_function = function.clone();
            replacer_fn = Some(&tmp_function);
        } else if let Some(array) = replacer.as_array() {
            let mut filter = HashSet::with_capacity(array.len());
            for value in array.clone().into_iter() {
                let value = value?;
                if let Some(string) = value.as_string() {
                    filter.insert(string.to_string()?);
                } else if let Some(number) = value.as_int() {
                    let mut buffer = itoa::Buffer::new();
                    filter.insert(buffer.format(number).to_string());
                } else if let Some(number) = value.as_float() {
                    let mut buffer = ryu::Buffer::new();
                    filter.insert(buffer.format(number).to_string());
                }
            }
            include_keys_replacer = Some(filter);
        }
    }

    let indentation = indentation.as_deref();
    let include_keys_replacer = include_keys_replacer.as_ref();

    let mut ancestors = Vec::with_capacity(10);

    let mut context = IterationContext {
        ctx,
        result: &mut result,
        value: &value,
        depth: 0,
        indentation: None,
        key: None,
        index: None,
        parent: None,
        ancestors: &mut ancestors,
        replacer_fn,
        include_keys_replacer,
    };

    if write_primitive(&mut context, false)? {
        return Ok(Some(result));
    }

    context.depth += 1;
    context.indentation = indentation;
    iterate(&mut context)?;
    Ok(Some(result))
}

#[inline(always)]
#[cold]
fn write_indentation(result: &mut String, indentation: Option<&str>, depth: usize) {
    if let Some(indentation) = indentation {
        result.push('\n');
        result.push_str(&indentation.repeat(depth - 1));
    }
}

#[inline(always)]
#[cold]
fn run_to_json<'js>(
    context: &mut IterationContext<'_, 'js>,
    js_object: &Object<'js>,
) -> Result<()> {
    let to_json = js_object.get::<_, Function>(PredefinedAtom::ToJSON)?;
    let val = to_json.call((This(js_object.clone()),))?;
    append_value(
        &mut IterationContext {
            ctx: context.ctx,
            result: context.result,
            value: &val,
            depth: context.depth,
            indentation: context.indentation,
            key: context.key,
            index: None,
            parent: Some(js_object),
            ancestors: context.ancestors,
            replacer_fn: context.replacer_fn,
            include_keys_replacer: context.include_keys_replacer,
        },
        false,
    )?;
    Ok(())
}

#[inline(always)]
#[cold]
fn run_replacer<'js>(
    context: &mut IterationContext<'_, 'js>,
    replacer_fn: &Function<'js>,
    add_comma: bool,
) -> Result<bool> {
    let parent = context.parent;
    let ctx = context.ctx;
    let value = context.value;
    let key = context.key;
    let index = context.index;
    let parent = if let Some(parent) = parent {
        parent.clone()
    } else {
        let parent = Object::new(ctx.clone())?;
        parent.set("", value.clone())?;
        parent
    };
    let new_value = replacer_fn.call((This(parent), get_key_or_index(key, index), value))?;
    write_primitive(
        &mut IterationContext {
            ctx,
            result: context.result,
            value: &new_value,
            replacer_fn: None,
            key,
            index: None,
            indentation: context.indentation,
            parent: None,
            include_keys_replacer: None,
            depth: context.depth,
            ancestors: context.ancestors,
        },
        add_comma,
    )
}

#[inline(always)]
fn write_primitive(context: &mut IterationContext, add_comma: bool) -> Result<bool> {
    if let Some(replacer_fn) = context.replacer_fn {
        return run_replacer(context, replacer_fn, add_comma);
    }

    let include_keys_replacer = context.include_keys_replacer;
    let value = context.value;
    let key = context.key;
    let index = context.index;
    let indentation = context.indentation;
    let depth = context.depth;

    let type_of = value.type_of();

    if matches!(type_of, Type::Symbol | Type::Undefined) {
        return Ok(true);
    }

    if let Some(include_keys_replacer) = include_keys_replacer {
        let key = get_key_or_index(key, index);
        if !include_keys_replacer.contains(&key) {
            return Ok(true);
        }
    };

    if let Some(indentation) = indentation {
        write_indented_separator(context.result, key, add_comma, indentation, depth);
    } else {
        write_sep(context.result, add_comma, false);
        if let Some(key) = key {
            write_key(context.result, key, false);
        }
    }

    match type_of {
        Type::Null => context.result.push_str("null"),
        Type::Bool => context.result.push_str(match value.as_bool().unwrap() {
            true => "true",
            false => "false",
        }),
        Type::Int => {
            let mut buffer = itoa::Buffer::new();
            context
                .result
                .push_str(buffer.format(value.as_int().unwrap()))
        }
        Type::Float => {
            let float_value = value.as_float().unwrap();
            const EXP_MASK: u64 = 0x7ff0000000000000;
            let bits = float_value.to_bits();
            if bits & EXP_MASK == EXP_MASK {
                context.result.push_str("null");
            } else {
                let mut buffer = ryu::Buffer::new();
                let str = buffer.format_finite(value.as_float().unwrap());

                let bytes = str.as_bytes();
                let len = bytes.len();

                context.result.push_str(str);

                if &bytes[len - 2..] == b".0" {
                    let len = context.result.len();
                    unsafe { context.result.as_mut_vec().set_len(len - 2) }
                }
            }
        }
        Type::String => write_string(context.result, &value.as_string().unwrap().to_string()?),
        _ => return Ok(false),
    }
    Ok(true)
}

#[inline(always)]
#[cold]
fn write_indented_separator(
    result: &mut String,
    key: Option<&str>,
    add_comma: bool,
    indentation: &str,
    depth: usize,
) {
    write_sep(result, add_comma, true);
    result.push_str(&indentation.repeat(depth));
    if let Some(key) = key {
        write_key(result, key, true);
    }
}

#[cold]
#[inline(always)]
fn detect_circular_reference(
    ctx: &Ctx<'_>,
    value: &Object<'_>,
    key: Option<&str>,
    index: Option<usize>,
    parent: Option<&Object<'_>>,
    ancestors: &mut Vec<(usize, String)>,
) -> Result<()> {
    let parent_ptr = unsafe { parent.unwrap().as_raw().u.ptr as usize };
    let current_ptr = unsafe { value.as_raw().u.ptr as usize };

    while !ancestors.is_empty()
        && match ancestors.last() {
            Some((ptr, _)) => ptr != &parent_ptr,
            _ => false,
        }
    {
        ancestors.pop();
    }

    if ancestors.iter().any(|(ptr, _)| ptr == &current_ptr) {
        let mut iter = ancestors.iter_mut();

        let first = &iter.next().unwrap().1;

        let mut path = iter
            .rev()
            .take(4)
            .rev()
            .fold(String::new(), |mut acc, (_, key)| {
                if !key.starts_with('[') {
                    acc.push('.');
                }
                acc.push_str(key);
                acc
            });

        if !first.starts_with('[') {
            path.push('.');
        }

        path.push_str(first);

        return Err(Exception::throw_type(
            ctx,
            &format!("Circular reference detected at: \"..{}\"", path),
        ));
    }
    ancestors.push((
        current_ptr,
        key.map(|k| k.to_string())
            .unwrap_or_else(|| format!("[{}]", index.unwrap_or_default())),
    ));

    Ok(())
}

#[inline(always)]
fn append_value(context: &mut IterationContext<'_, '_>, add_comma: bool) -> Result<()> {
    if !write_primitive(context, add_comma)? {
        context.depth += 1;
        iterate(context)?;
    }

    Ok(())
}

#[inline(always)]
fn write_key(string: &mut String, key: &str, indent: bool) {
    string.push('"');
    escape_json_string(string, key.as_bytes());
    if indent {
        string.push_str("\": ");
    } else {
        string.push_str("\":");
    }
}

#[inline(always)]
fn write_sep(result: &mut String, add_comma: bool, has_indentation: bool) {
    if !add_comma && !has_indentation {
        return;
    }

    const SEPARATOR_TABLE: [&str; 4] = [
        "",    // add_comma = false, has_indentation = false
        ",",   // add_comma = false, has_indentation = true
        "\n",  // add_comma = true, has_indentation = false
        ",\n", // add_comma = true, has_indentation = true
    ];

    let separator = SEPARATOR_TABLE[(add_comma as usize) | ((has_indentation as usize) << 1)];
    result.push_str(separator);
}

#[inline(always)]
fn write_string(string: &mut String, value: &str) {
    string.push('"');
    escape_json_string(string, value.as_bytes());
    string.push('"');
}

#[inline(always)]
fn get_key_or_index(key: Option<&str>, index: Option<usize>) -> String {
    key.map(|k| k.to_string()).unwrap_or_else(|| {
        let mut buffer = itoa::Buffer::new();
        buffer.format(index.unwrap_or_default()).to_string()
    })
}

#[inline(always)]
fn iterate(context: &mut IterationContext<'_, '_>) -> Result<()> {
    let mut add_comma;
    let elem = context.value;
    let depth = context.depth;
    let ctx = context.ctx;
    let indentation = context.indentation;
    match elem.type_of() {
        Type::Object => {
            let js_object = elem.as_object().unwrap();
            if js_object.contains_key(PredefinedAtom::ToJSON)? {
                return run_to_json(context, js_object);
            }

            //only start detect circular reference at this level
            if depth > CIRCULAR_REF_DETECTION_DEPTH {
                detect_circular_reference(
                    ctx,
                    js_object,
                    context.key,
                    context.index,
                    context.parent,
                    context.ancestors,
                )?;
            }

            context.result.push('{');

            add_comma = false;
            for key in js_object.keys::<String>() {
                let key = key?;
                let val = js_object.get(&key)?;

                append_value(
                    &mut IterationContext {
                        ctx,
                        result: context.result,
                        value: &val,
                        depth,
                        key: Some(&key),
                        indentation,
                        index: None,
                        parent: Some(js_object),
                        ancestors: context.ancestors,
                        replacer_fn: context.replacer_fn,
                        include_keys_replacer: context.include_keys_replacer,
                    },
                    add_comma,
                )?;
                add_comma = true;
            }
            if add_comma {
                write_indentation(context.result, indentation, depth);
            }
            context.result.push('}');
        }
        Type::Array => {
            context.result.push('[');
            add_comma = false;
            let js_array = elem.as_array().unwrap();
            //only start detect circular reference at this level
            if depth > CIRCULAR_REF_DETECTION_DEPTH {
                detect_circular_reference(
                    ctx,
                    js_array.as_object(),
                    context.key,
                    context.index,
                    context.parent,
                    context.ancestors,
                )?;
            }
            for (i, val) in js_array.iter::<Value>().enumerate() {
                let val = val?;
                append_value(
                    &mut IterationContext {
                        ctx,
                        result: context.result,
                        value: &val,
                        depth,
                        key: None,
                        indentation,
                        index: Some(i),
                        parent: Some(js_array),
                        ancestors: context.ancestors,
                        replacer_fn: context.replacer_fn,
                        include_keys_replacer: context.include_keys_replacer,
                    },
                    add_comma,
                )?;
                add_comma = true;
            }
            if add_comma {
                write_indentation(context.result, indentation, depth);
            }
            context.result.push(']');
        }
        _ => {}
    }
    Ok(())
}
