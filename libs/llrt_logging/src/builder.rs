// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! IR Builder - converts JavaScript values to PrintIR.
//!
//! This module handles the first phase of the two-phase approach:
//! traversing JavaScript values and building an intermediate representation
//! that captures all the information needed for rendering.

use std::collections::HashSet;

use llrt_utils::{class::get_class_name, error::ErrorExtensions, hash};
use rquickjs::{
    atom::PredefinedAtom,
    function::This,
    object::Filter,
    promise::PromiseState,
    qjs, Ctx,
    Error::{self},
    Function, Object, Result, Symbol, Type, Value,
};

use crate::ir::{NumberIR, ObjectKey, PrintIR, PromiseStateIR, TruncatedKind};

use std::{mem, slice};

/// Options controlling how the IR is built
pub struct BuildOptions<'js> {
    /// Maximum recursion depth for objects
    pub max_depth: usize,
    /// Maximum number of array/object elements to include
    pub max_array_length: usize,
    /// Maximum string length before truncation
    pub max_string_length: usize,
    /// Whether to include non-enumerable properties
    pub show_hidden: bool,
    /// Whether to call custom inspect functions
    pub custom_inspect: bool,
    /// Filter for object properties
    pub object_filter: Filter,
    /// Object.prototype for comparison
    pub object_prototype: Object<'js>,
    /// Symbol for custom inspect
    pub custom_inspect_symbol: Symbol<'js>,
    /// Object.getOwnPropertyDescriptor function
    pub get_own_property_desc_fn: Function<'js>,
    /// Sort mode for object keys
    pub sorted: SortMode,
    /// Custom sort comparator function
    pub sort_comparator: Option<Function<'js>>,
}

/// Sort mode for object keys
#[derive(Clone, Default)]
pub enum SortMode {
    #[default]
    None,
    Alphabetical,
    Custom,
}

impl<'js> BuildOptions<'js> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ctx: &Ctx<'js>,
        max_depth: usize,
        max_array_length: usize,
        max_string_length: usize,
        show_hidden: bool,
        custom_inspect: bool,
        sorted: SortMode,
        sort_comparator: Option<Function<'js>>,
    ) -> Result<Self> {
        use llrt_utils::primordials::{BasePrimordials, Primordial};

        let primordials = BasePrimordials::get(ctx)?;
        let object_filter = if show_hidden {
            Filter::new().private().string().symbol()
        } else {
            Filter::new().private().string().enum_only()
        };

        Ok(Self {
            max_depth,
            max_array_length,
            max_string_length,
            show_hidden,
            custom_inspect,
            object_filter,
            object_prototype: primordials.prototype_object.clone(),
            custom_inspect_symbol: primordials.symbol_custom_inspect.clone(),
            get_own_property_desc_fn: primordials.function_get_own_property_descriptor.clone(),
            sorted,
            sort_comparator,
        })
    }
}

/// Build PrintIR from a JavaScript value
pub fn build_ir<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    options: &BuildOptions<'js>,
) -> Result<PrintIR> {
    let mut visited = HashSet::new();
    build_ir_inner(ctx, value, options, &mut visited, 0, false)
}

/// Build PrintIR from a JavaScript value (internal recursive function)
fn build_ir_inner<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    options: &BuildOptions<'js>,
    visited: &mut HashSet<usize>,
    depth: usize,
    quoted: bool,
) -> Result<PrintIR> {
    let value_type = value.type_of();
    let is_root = depth == 0;

    match value_type {
        Type::Uninitialized | Type::Null => Ok(PrintIR::Null),
        Type::Undefined => Ok(PrintIR::Undefined),
        Type::Bool => Ok(PrintIR::Bool(unsafe { value.as_bool().unwrap_unchecked() })),
        Type::BigInt => {
            let big_int = unsafe { value.as_big_int().unwrap_unchecked() };
            Ok(PrintIR::Number(NumberIR::BigInt(
                big_int.clone().to_i64().unwrap_or(0),
            )))
        },
        Type::Int => Ok(PrintIR::Number(NumberIR::Int(unsafe {
            value.as_int().unwrap_unchecked()
        }))),
        Type::Float => Ok(PrintIR::Number(NumberIR::Float(unsafe {
            value.as_float().unwrap_unchecked()
        }))),
        Type::String => {
            let lossy_string = get_lossy_string(value)?;
            let should_quote = quoted || !is_root;
            let char_count = lossy_string.chars().count();

            if options.max_string_length < char_count {
                let truncated: String = lossy_string
                    .chars()
                    .take(options.max_string_length)
                    .collect();
                Ok(PrintIR::Group(vec![
                    PrintIR::String {
                        value: truncated,
                        quoted: should_quote,
                    },
                    PrintIR::Truncated {
                        remaining: char_count - options.max_string_length,
                        kind: TruncatedKind::Characters,
                    },
                ]))
            } else {
                Ok(PrintIR::String {
                    value: lossy_string,
                    quoted: should_quote,
                })
            }
        },
        Type::Symbol => {
            let description = unsafe { value.as_symbol().unwrap_unchecked() }.description()?;
            let desc_str: String = description.get()?;
            Ok(PrintIR::Symbol(desc_str))
        },
        Type::Function | Type::Constructor => {
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

            Ok(PrintIR::Function { name, is_class })
        },
        Type::Promise => {
            let promise = unsafe { value.as_promise().unwrap_unchecked() };
            let state = promise.state();

            let state_ir = match state {
                PromiseState::Pending => PromiseStateIR::Pending,
                PromiseState::Resolved => {
                    let resolved_value: Value = unsafe { promise.result().unwrap_unchecked() }?;
                    let resolved_ir =
                        build_ir_inner(ctx, resolved_value, options, visited, depth + 1, false)?;
                    PromiseStateIR::Resolved(Box::new(resolved_ir))
                },
                PromiseState::Rejected => {
                    let rejected_error: Error =
                        unsafe { promise.result::<Value>().unwrap_unchecked() }.unwrap_err();
                    let rejected_value = rejected_error.into_value(promise.ctx())?;
                    let rejected_ir =
                        build_ir_inner(ctx, rejected_value, options, visited, depth + 1, false)?;
                    PromiseStateIR::Rejected(Box::new(rejected_ir))
                },
            };

            Ok(PrintIR::Promise(state_ir))
        },
        Type::Array | Type::Object | Type::Exception => {
            let hash = hash::default_hash(&value);
            if visited.contains(&hash) {
                return Ok(PrintIR::Circular);
            }
            visited.insert(hash);

            let obj = unsafe { value.as_object().unwrap_unchecked() };

            // Handle Error objects
            if value.is_error() {
                let name: String = obj.get(PredefinedAtom::Name)?;
                let message: String = obj.get(PredefinedAtom::Message)?;
                let stack: Result<String> = obj.get(PredefinedAtom::Stack);

                let stack_lines = stack
                    .ok()
                    .map(|s| s.trim().split('\n').map(|line| line.to_string()).collect());

                visited.remove(&hash);
                return Ok(PrintIR::Error {
                    name,
                    message,
                    stack: stack_lines,
                });
            }

            // Get class name for all object-like types (Object, Array, Exception)
            // This handles custom classes like Headers, URLSearchParams, etc.
            let mut class_name: Option<String> = get_class_name(&value)?;
            let is_object = value_type == Type::Object;

            match class_name.as_deref() {
                Some("Date") => {
                    let iso_fn: Function = obj.get("toISOString").unwrap();
                    let str: String = iso_fn.call((This(value),))?;
                    visited.remove(&hash);
                    return Ok(PrintIR::Date(str));
                },
                Some("RegExp") => {
                    let source: String = obj.get("source")?;
                    let flags: String = obj.get("flags")?;
                    visited.remove(&hash);
                    return Ok(PrintIR::RegExp { source, flags });
                },
                // Filter out generic class names
                None | Some("") | Some("Object") | Some("Array") => {
                    class_name = None;
                },
                _ => {},
            }

            // Check max depth
            if depth >= options.max_depth {
                visited.remove(&hash);
                return Ok(PrintIR::MaxDepth {
                    is_array: !is_object,
                });
            }

            // Check for typed arrays
            let mut is_typed_array = false;
            if let Some(ref cn) = class_name {
                is_typed_array = matches!(
                    cn.as_str(),
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

            // Check for custom inspect function if enabled
            if options.custom_inspect {
                if let Ok(custom_fn) =
                    obj.get::<_, Function>(options.custom_inspect_symbol.as_atom())
                {
                    let remaining_depth = options.max_depth.saturating_sub(depth);
                    let inspect_result: Value =
                        custom_fn.call((This(obj.clone()), remaining_depth))?;

                    visited.remove(&hash);

                    if let Some(s) = inspect_result.as_string() {
                        return Ok(PrintIR::Custom(s.to_string()?));
                    } else {
                        // Recursively build IR for the custom result
                        let inner_ir = build_ir_inner(
                            ctx,
                            inspect_result,
                            options,
                            visited,
                            depth + 1,
                            false,
                        )?;
                        // Wrap with class name if present (custom inspect replaces content, not class name)
                        if let Some(cn) = class_name {
                            return Ok(PrintIR::WithClass {
                                class_name: cn,
                                inner: Box::new(inner_ir),
                            });
                        }
                        return Ok(inner_ir);
                    }
                } else if let Ok(custom_value) =
                    obj.get::<_, Value>(options.custom_inspect_symbol.as_atom())
                {
                    if !custom_value.is_undefined() && !custom_value.is_null() {
                        visited.remove(&hash);

                        if let Some(s) = custom_value.as_string() {
                            return Ok(PrintIR::Custom(s.to_string()?));
                        } else {
                            // Recursively build IR for the custom value
                            let inner_ir =
                                build_ir_inner(ctx, custom_value, options, visited, depth, false)?;
                            // Wrap with class name if present
                            if let Some(cn) = class_name {
                                return Ok(PrintIR::WithClass {
                                    class_name: cn,
                                    inner: Box::new(inner_ir),
                                });
                            }
                            return Ok(inner_ir);
                        }
                    }
                }
            }

            // Build object/array IR
            let result = build_object_ir(ctx, obj, options, visited, depth, is_array, class_name)?;
            visited.remove(&hash);
            Ok(result)
        },
        _ => Ok(PrintIR::Raw(String::new())),
    }
}

fn build_object_ir<'js>(
    ctx: &Ctx<'js>,
    obj: &Object<'js>,
    options: &BuildOptions<'js>,
    visited: &mut HashSet<usize>,
    depth: usize,
    is_array: bool,
    class_name: Option<String>,
) -> Result<PrintIR> {
    // Get keys
    let mut keys = if options.show_hidden {
        obj.own_keys(options.object_filter)
    } else {
        obj.keys()
    };

    let mut filter_functions = false;
    if !is_array && keys.len() == 0 {
        if let Some(proto) = obj.get_prototype() {
            if proto != options.object_prototype {
                keys = proto.own_keys(options.object_filter);
                filter_functions = true;
            }
        }
    }

    // Collect and optionally sort keys
    let mut key_vec: Vec<String> = keys.flatten().collect();
    if !is_array {
        match &options.sorted {
            SortMode::None => {},
            SortMode::Alphabetical => {
                key_vec.sort();
            },
            SortMode::Custom => {
                if let Some(ref comparator) = options.sort_comparator {
                    key_vec.sort_by(|a, b| {
                        match comparator.call::<_, i32>((a.as_str(), b.as_str())) {
                            Ok(result) => result.cmp(&0),
                            Err(_) => std::cmp::Ordering::Equal,
                        }
                    });
                }
            },
        }
    }

    let total_length = key_vec.len();

    if is_array {
        // Build array IR
        let mut elements = Vec::with_capacity(total_length.min(options.max_array_length));

        for (i, key) in key_vec.into_iter().enumerate() {
            let value: Value = obj.get::<&String, _>(&key)?;
            if !(value.is_function() && filter_functions) {
                let element_ir = build_ir_inner(ctx, value, options, visited, depth + 1, false)?;
                elements.push(element_ir);

                if i >= options.max_array_length.saturating_sub(1)
                    && total_length > options.max_array_length
                {
                    break;
                }
            }
        }

        Ok(PrintIR::Array {
            class_name,
            elements,
            total_length,
        })
    } else {
        // Build object IR
        let mut entries = Vec::with_capacity(total_length.min(options.max_array_length));

        for (i, key) in key_vec.into_iter().enumerate() {
            let value: Value = obj.get::<&String, _>(&key)?;
            if !(value.is_function() && filter_functions) {
                let is_numeric = key.parse::<f64>().is_ok();
                let value_ir = build_ir_inner(ctx, value, options, visited, depth + 1, false)?;

                entries.push((
                    ObjectKey {
                        name: key,
                        is_numeric,
                    },
                    value_ir,
                ));

                if i >= options.max_array_length.saturating_sub(1)
                    && total_length > options.max_array_length
                {
                    break;
                }
            }
        }

        Ok(PrintIR::Object {
            class_name,
            entries,
            total_entries: total_length,
        })
    }
}

/// Get a lossy string from a JS string value, handling invalid UTF-8/UTF-16
pub fn get_lossy_string(string_value: Value) -> Result<String> {
    if !string_value.is_string() {
        return Err(Error::FromJs {
            from: "Value",
            to: "JSString",
            message: Some("Value is not a string".into()),
        });
    }

    let mut len = mem::MaybeUninit::uninit();
    let ctx_ptr = string_value.ctx().as_raw().as_ptr();

    let ptr = unsafe { qjs::JS_ToCStringLen(ctx_ptr, len.as_mut_ptr(), string_value.as_raw()) };
    if ptr.is_null() {
        return Err(Error::Unknown);
    }
    let len = unsafe { len.assume_init() };
    let bytes: &[u8] = unsafe { slice::from_raw_parts(ptr as _, len as _) };
    let string = replace_invalid_utf8_and_utf16(bytes);
    unsafe { qjs::JS_FreeCString(ctx_ptr, ptr) };

    Ok(string)
}

fn replace_invalid_utf8_and_utf16(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        let current = bytes[i];
        match current {
            // ASCII (1-byte)
            0x00..=0x7F => {
                result.push(current as char);
                i += 1;
            },
            // 2-byte UTF-8 sequence
            0xC0..=0xDF => {
                if i + 1 < bytes.len() {
                    let next = bytes[i + 1];
                    if (next & 0xC0) == 0x80 {
                        let code_point = ((current as u32 & 0x1F) << 6) | (next as u32 & 0x3F);
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        } else {
                            result.push('�');
                        }
                        i += 2;
                    } else {
                        result.push('�');
                        i += 1;
                    }
                } else {
                    result.push('�');
                    i += 1;
                }
            },
            // 3-byte UTF-8 sequence
            0xE0..=0xEF => {
                if i + 2 < bytes.len() {
                    let next1 = bytes[i + 1];
                    let next2 = bytes[i + 2];
                    if (next1 & 0xC0) == 0x80 && (next2 & 0xC0) == 0x80 {
                        let code_point = ((current as u32 & 0x0F) << 12)
                            | ((next1 as u32 & 0x3F) << 6)
                            | (next2 as u32 & 0x3F);
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        } else {
                            result.push('�');
                        }
                        i += 3;
                    } else {
                        result.push('�');
                        i += 1;
                    }
                } else {
                    result.push('�');
                    i += 1;
                }
            },
            // 4-byte UTF-8 sequence
            0xF0..=0xF7 => {
                if i + 3 < bytes.len() {
                    let next1 = bytes[i + 1];
                    let next2 = bytes[i + 2];
                    let next3 = bytes[i + 3];
                    if (next1 & 0xC0) == 0x80 && (next2 & 0xC0) == 0x80 && (next3 & 0xC0) == 0x80 {
                        let code_point = ((current as u32 & 0x07) << 18)
                            | ((next1 as u32 & 0x3F) << 12)
                            | ((next2 as u32 & 0x3F) << 6)
                            | (next3 as u32 & 0x3F);
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        } else {
                            result.push('�');
                        }
                        i += 4;
                    } else {
                        result.push('�');
                        i += 1;
                    }
                } else {
                    result.push('�');
                    i += 1;
                }
            },
            // Invalid starting byte
            _ => {
                result.push('�');
                i += 1;
            },
        }
    }

    result
}
