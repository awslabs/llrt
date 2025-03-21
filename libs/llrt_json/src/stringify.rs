// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::HashSet, rc::Rc};

use rquickjs::{
    atom::PredefinedAtom, function::This, Ctx, Exception, Function, Object, Result, Type, Value,
};

use crate::escape::escape_json_string;

const CIRCULAR_REF_DETECTION_DEPTH: usize = 20;

struct StringifyContext<'a, 'js> {
    ctx: &'a Ctx<'js>,
    result: &'a mut String,
    value: &'a Value<'js>,
    depth: usize,
    indentation: Option<&'a str>,
    key: Option<&'a str>,
    index: Option<usize>,
    parent: Option<&'a Object<'js>>,
    ancestors: &'a mut Vec<(usize, Rc<str>)>,
    replacer_fn: Option<&'a Function<'js>>,
    include_keys_replacer: Option<&'a HashSet<String>>,
    itoa_buffer: &'a mut itoa::Buffer,
    ryu_buffer: &'a mut ryu::Buffer,
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

    let mut itoa_buffer = itoa::Buffer::new();
    let mut ryu_buffer = ryu::Buffer::new();

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
                    filter.insert(itoa_buffer.format(number).to_string());
                } else if let Some(number) = value.as_float() {
                    filter.insert(ryu_buffer.format(number).to_string());
                }
            }
            include_keys_replacer = Some(filter);
        }
    }

    let indentation = indentation.as_deref();
    let include_keys_replacer = include_keys_replacer.as_ref();

    let mut ancestors = Vec::with_capacity(10);

    let mut context = StringifyContext {
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
        itoa_buffer: &mut itoa_buffer,
        ryu_buffer: &mut ryu_buffer,
    };

    match write_primitive(&mut context, false)? {
        PrimitiveStatus::Written => {
            return Ok(Some(result));
        },
        PrimitiveStatus::Ignored => {
            return Ok(None);
        },
        _ => {},
    }

    context.depth += 1;
    context.indentation = indentation;
    iterate(&mut context, None)?;
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
    context: &mut StringifyContext<'_, 'js>,
    js_object: &Object<'js>,
) -> Result<()> {
    let to_json = js_object.get::<_, Function>(PredefinedAtom::ToJSON)?;
    let val: Value = to_json.call((This(js_object.clone()),))?;

    //only preserve indentation if we're returning nested data
    let indentation = context.indentation.and_then(|indentation| {
        matches!(val.type_of(), Type::Object | Type::Array | Type::Exception).then_some(indentation)
    });

    append_value(
        &mut StringifyContext {
            ctx: context.ctx,
            result: context.result,
            value: &val,
            depth: context.depth,
            indentation,
            key: None,
            index: None,
            parent: Some(js_object),
            ancestors: context.ancestors,
            replacer_fn: context.replacer_fn,
            include_keys_replacer: context.include_keys_replacer,
            itoa_buffer: context.itoa_buffer,
            ryu_buffer: context.ryu_buffer,
        },
        false,
    )?;
    Ok(())
}

#[derive(PartialEq)]
enum PrimitiveStatus<'js> {
    Written,
    Ignored,
    Iterate(Option<Value<'js>>),
}

#[inline(always)]
#[cold]
fn run_replacer<'js>(
    context: &mut StringifyContext<'_, 'js>,
    replacer_fn: &Function<'js>,
    add_comma: bool,
) -> Result<PrimitiveStatus<'js>> {
    let key = context.key;
    let index = context.index;
    let value = context.value;
    let parent = if let Some(parent) = context.parent {
        parent.clone()
    } else {
        let parent = Object::new(context.ctx.clone())?;
        parent.set("", value.clone())?;
        parent
    };
    let new_value: Value = replacer_fn.call((
        This(parent),
        get_key_or_index(context.itoa_buffer, key, index),
        value,
    ))?;

    write_primitive2(context, add_comma, Some(new_value))
}

fn write_primitive<'js>(
    context: &mut StringifyContext<'_, 'js>,
    add_comma: bool,
) -> Result<PrimitiveStatus<'js>> {
    if let Some(replacer_fn) = context.replacer_fn {
        return run_replacer(context, replacer_fn, add_comma);
    }

    write_primitive2(context, add_comma, None)
}

fn write_primitive2<'js>(
    context: &mut StringifyContext<'_, 'js>,
    add_comma: bool,
    new_value: Option<Value<'js>>,
) -> Result<PrimitiveStatus<'js>> {
    let key = context.key;
    let index = context.index;
    let include_keys_replacer = context.include_keys_replacer;
    let indentation = context.indentation;
    let depth = context.depth;

    let value = new_value.as_ref().unwrap_or(context.value);

    let type_of = value.type_of();

    if context.index.is_none()
        && matches!(
            type_of,
            Type::Symbol | Type::Undefined | Type::Function | Type::Constructor
        )
    {
        return Ok(PrimitiveStatus::Ignored);
    }

    if matches!(type_of, Type::BigInt) {
        return Err(Exception::throw_type(
            context.ctx,
            "Do not know how to serialize a BigInt",
        ));
    }

    if let Some(include_keys_replacer) = include_keys_replacer {
        let key = get_key_or_index(context.itoa_buffer, key, index);
        if !include_keys_replacer.contains(key) {
            return Ok(PrimitiveStatus::Ignored);
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
        Type::Null | Type::Undefined => context.result.push_str("null"),
        Type::Bool => {
            let bool_str = if unsafe { value.as_bool().unwrap_unchecked() } {
                "true"
            } else {
                "false"
            };
            context.result.push_str(bool_str);
        },
        Type::Int => context.result.push_str(
            context
                .itoa_buffer
                .format(unsafe { value.as_int().unwrap_unchecked() }),
        ),
        Type::Float => {
            let float_value = unsafe { value.as_float().unwrap_unchecked() };
            const EXP_MASK: u64 = 0x7ff0000000000000;
            let bits = float_value.to_bits();
            if bits & EXP_MASK == EXP_MASK {
                context.result.push_str("null");
            } else {
                let str = context.ryu_buffer.format_finite(float_value);

                let bytes = str.as_bytes();
                let len = bytes.len();

                context.result.push_str(str);

                if &bytes[len - 2..] == b".0" {
                    let len = context.result.len();
                    unsafe { context.result.as_mut_vec().set_len(len - 2) }
                }
            }
        },
        Type::String => write_string(
            context.result,
            &unsafe { value.as_string().unwrap_unchecked() }.to_string()?,
        ),
        _ => return Ok(PrimitiveStatus::Iterate(new_value)),
    }
    Ok(PrimitiveStatus::Written)
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
fn detect_circular_reference(
    ctx: &Ctx<'_>,
    value: &Object<'_>,
    key: Option<&str>,
    index: Option<usize>,
    parent: Option<&Object<'_>>,
    ancestors: &mut Vec<(usize, Rc<str>)>,
    itoa_buffer: &mut itoa::Buffer,
) -> Result<()> {
    let parent_ptr = unsafe { parent.unwrap_unchecked().as_raw().u.ptr as usize };
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

        let first = &unsafe { iter.next().unwrap_unchecked() }.1;

        let mut message = iter.rev().take(4).rev().fold(
            String::from("Circular reference detected at: \".."),
            |mut acc, (_, key)| {
                if !key.starts_with('[') {
                    acc.push('.');
                }
                acc.push_str(key);
                acc
            },
        );

        if !first.starts_with('[') {
            message.push('.');
        }

        message.push_str(first);
        message.push('"');

        return Err(Exception::throw_type(ctx, &message));
    }
    ancestors.push((
        current_ptr,
        key.map(|k| k.into()).unwrap_or_else(|| {
            ["[", itoa_buffer.format(index.unwrap_or_default()), "]"]
                .concat()
                .into()
        }),
    ));

    Ok(())
}

#[inline(always)]
fn append_value(context: &mut StringifyContext<'_, '_>, add_comma: bool) -> Result<bool> {
    match write_primitive(context, add_comma)? {
        PrimitiveStatus::Written => Ok(true),
        PrimitiveStatus::Ignored => Ok(false),
        PrimitiveStatus::Iterate(new_value) => {
            context.depth += 1;
            iterate(context, new_value)?;
            Ok(true)
        },
    }
}

#[inline(always)]
fn write_key(string: &mut String, key: &str, indent: bool) {
    string.push('"');
    escape_json_string(string, key.as_bytes());
    string.push_str("\":");
    if indent {
        string.push(' ');
    }
}

#[inline(always)]
fn write_sep(result: &mut String, add_comma: bool, has_indentation: bool) {
    if add_comma {
        result.push(',');
    }
    if has_indentation {
        result.push('\n');
    }
}

#[inline(always)]
fn write_string(string: &mut String, value: &str) {
    string.push('"');
    escape_json_string(string, value.as_bytes());
    string.push('"');
}

#[inline(always)]
fn get_key_or_index<'a>(
    itoa_buffer: &'a mut itoa::Buffer,
    key: Option<&'a str>,
    index: Option<usize>,
) -> &'a str {
    key.unwrap_or_else(|| itoa_buffer.format(index.unwrap_or_default()))
}

fn iterate<'js>(
    context: &mut StringifyContext<'_, 'js>,
    new_value: Option<Value<'js>>,
) -> Result<()> {
    let mut add_comma;
    let mut value_written;
    let elem = new_value.as_ref().unwrap_or(context.value);
    let depth = context.depth;
    let ctx = context.ctx;
    let indentation = context.indentation;
    match elem.type_of() {
        Type::Object | Type::Exception => {
            let js_object = unsafe { elem.as_object().unwrap_unchecked() };
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
                    context.itoa_buffer,
                )?;
            }

            context.result.push('{');

            value_written = false;

            for key in js_object.keys::<String>() {
                let key = key?;
                let val = js_object.get(&key)?;

                add_comma = append_value(
                    &mut StringifyContext {
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
                        itoa_buffer: context.itoa_buffer,
                        ryu_buffer: context.ryu_buffer,
                    },
                    value_written,
                )?;
                value_written = value_written || add_comma;
            }

            if value_written {
                write_indentation(context.result, indentation, depth);
            }
            context.result.push('}');
        },
        Type::Array => {
            context.result.push('[');
            add_comma = false;
            value_written = false;
            let js_array = unsafe { elem.as_array().unwrap_unchecked() };
            //only start detect circular reference at this level
            if depth > CIRCULAR_REF_DETECTION_DEPTH {
                detect_circular_reference(
                    ctx,
                    js_array.as_object(),
                    context.key,
                    context.index,
                    context.parent,
                    context.ancestors,
                    context.itoa_buffer,
                )?;
            }
            for (i, val) in js_array.iter::<Value>().enumerate() {
                let val = val?;
                add_comma = append_value(
                    &mut StringifyContext {
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
                        itoa_buffer: context.itoa_buffer,
                        ryu_buffer: context.ryu_buffer,
                    },
                    add_comma,
                )?;
                value_written = value_written || add_comma;
            }
            if value_written {
                write_indentation(context.result, indentation, depth);
            }
            context.result.push(']');
        },
        _ => {},
    }
    Ok(())
}
