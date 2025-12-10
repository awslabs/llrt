// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! LLRT Logging Library
//!
//! This library provides formatting and logging utilities for LLRT,
//! implementing a two-phase approach similar to Node.js:
//!
//! 1. **Build Phase**: Convert JavaScript values to an intermediate
//!    representation (`PrintIR`) that captures all the information
//!    needed for rendering.
//!
//! 2. **Render Phase**: Convert the IR to a formatted string,
//!    making formatting decisions (line breaks, indentation, colors)
//!    based on terminal width and other options.
//!
//! This approach allows console.log and util.inspect to share the
//! same underlying machinery while supporting different formatting
//! behaviors.

#![allow(clippy::uninlined_format_args)]

pub mod builder;
pub mod ir;
pub mod renderer;

use std::{
    io::{stderr, stdout, IsTerminal, Write},
    process::exit,
};

use llrt_json::stringify::json_stringify;
use std::ops::Deref;

use llrt_utils::{
    error::ErrorExtensions,
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{object::Filter, prelude::Rest, Coerced, Ctx, Function, Result, Value};

// Re-export key types
pub use builder::{build_ir, BuildOptions, SortMode};
pub use ir::PrintIR;
pub use renderer::{
    render, render_to, replace_newline_with_carriage_return, RenderOptions, CARRIAGE_RETURN,
    NEWLINE,
};

pub const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

const DEFAULT_DEPTH: usize = 2;
const DEFAULT_CONSOLE_DEPTH: usize = 4;
const DEFAULT_MAX_ARRAY_LENGTH: usize = 100;
const DEFAULT_MAX_STRING_LENGTH: usize = 10000;
const DEFAULT_BREAK_LENGTH: usize = 80;

/// Sort mode re-export for backwards compatibility
pub use builder::SortMode as InspectSortMode;

/// Options for util.inspect() - mirrors Node.js util.inspect options
#[derive(Clone)]
pub struct InspectOptions {
    /// Whether to show non-enumerable properties. Default: false
    pub show_hidden: bool,
    /// Recursion depth for objects. Default: 2. Use usize::MAX for infinite.
    pub depth: usize,
    /// Whether to use ANSI colors. Default: false
    pub colors: bool,
    /// Whether to call custom inspect functions. Default: true
    pub custom_inspect: bool,
    /// Max array/set/map elements to show. Default: 100
    pub max_array_length: usize,
    /// Max string characters to show. Default: 10000
    pub max_string_length: usize,
    /// Line length for breaking. Default: 80
    pub break_length: usize,
    /// How to sort object keys. Default: None
    pub sorted: SortMode,
    /// Compact output level. Default: 3
    pub compact: usize,
    /// Whether to use breakLength/compact heuristics for line breaking.
    /// When false (default for console), uses simple depth-based multiline.
    /// When true (for util.inspect), uses breakLength/compact to decide.
    pub use_break_heuristics: bool,
}

impl Default for InspectOptions {
    fn default() -> Self {
        // Default is for console/format: deeper depth, no break heuristics
        Self {
            show_hidden: false,
            depth: DEFAULT_CONSOLE_DEPTH,
            colors: false,
            custom_inspect: true,
            max_array_length: DEFAULT_MAX_ARRAY_LENGTH,
            max_string_length: DEFAULT_MAX_STRING_LENGTH,
            break_length: DEFAULT_BREAK_LENGTH,
            sorted: SortMode::None,
            compact: 3,
            use_break_heuristics: false,
        }
    }
}

impl InspectOptions {
    /// Create options for util.inspect() with Node.js-compatible defaults
    pub fn for_inspect() -> Self {
        Self {
            depth: DEFAULT_DEPTH,
            use_break_heuristics: true,
            ..Self::default()
        }
    }
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

/// Format options - used for building and rendering
pub struct FormatOptions<'js> {
    pub color: bool,
    pub newline: bool,
    number_function: Function<'js>,
    parse_float: Function<'js>,
    parse_int: Function<'js>,
    object_filter: Filter,
    pub inspect_options: InspectOptions,
    sort_comparator: Option<Function<'js>>,
}

impl<'js> FormatOptions<'js> {
    pub fn new(ctx: &Ctx<'js>, color: bool, newline: bool) -> Result<Self> {
        Self::with_inspect_options(ctx, color, newline, InspectOptions::default(), None)
    }

    pub fn with_inspect_options(
        ctx: &Ctx<'js>,
        color: bool,
        newline: bool,
        inspect_options: InspectOptions,
        sort_comparator: Option<Function<'js>>,
    ) -> Result<Self> {
        let primordials = BasePrimordials::get(ctx)?;

        let parse_float = primordials.function_parse_float.clone();
        let parse_int = primordials.function_parse_int.clone();

        let object_filter = if inspect_options.show_hidden {
            Filter::new().private().string().symbol()
        } else {
            Filter::new().private().string().enum_only()
        };

        let number_function = primordials.constructor_number.deref().clone();

        Ok(FormatOptions {
            color,
            newline,
            object_filter,
            number_function,
            parse_float,
            parse_int,
            inspect_options,
            sort_comparator,
        })
    }

    /// Create BuildOptions from FormatOptions
    fn to_build_options(&self, ctx: &Ctx<'js>) -> Result<BuildOptions<'js>> {
        let primordials = BasePrimordials::get(ctx)?;

        Ok(BuildOptions {
            max_depth: self.inspect_options.depth,
            max_array_length: self.inspect_options.max_array_length,
            max_string_length: self.inspect_options.max_string_length,
            show_hidden: self.inspect_options.show_hidden,
            custom_inspect: self.inspect_options.custom_inspect,
            object_filter: self.object_filter,
            object_prototype: primordials.prototype_object.clone(),
            custom_inspect_symbol: primordials.symbol_custom_inspect.clone(),
            get_own_property_desc_fn: primordials.function_get_own_property_descriptor.clone(),
            sorted: self.inspect_options.sorted.clone(),
            sort_comparator: self.sort_comparator.clone(),
        })
    }

    /// Create RenderOptions from FormatOptions
    fn to_render_options(&self) -> RenderOptions {
        RenderOptions {
            colors: self.color,
            newline: self.newline,
            break_length: self.inspect_options.break_length,
            compact: self.inspect_options.compact,
            use_break_heuristics: self.inspect_options.use_break_heuristics,
        }
    }
}

/// Format values without colors
pub fn format_plain<'js>(ctx: Ctx<'js>, newline: bool, args: Rest<Value<'js>>) -> Result<String> {
    format_values(&ctx, args, false, newline)
}

/// Format values with TTY detection for colors
pub fn format<'js>(ctx: &Ctx<'js>, newline: bool, args: Rest<Value<'js>>) -> Result<String> {
    format_values(ctx, args, stdout().is_terminal(), newline)
}

/// Format values with explicit TTY setting
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

/// Inspect a single value with custom options - used by util.inspect()
pub fn inspect_value<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    inspect_options: InspectOptions,
    sort_comparator: Option<Function<'js>>,
) -> Result<String> {
    let color = inspect_options.colors;
    let options =
        FormatOptions::with_inspect_options(ctx, color, true, inspect_options, sort_comparator)?;

    // Two-phase approach: build IR then render
    let build_opts = options.to_build_options(ctx)?;
    let render_opts = options.to_render_options();

    let ir = build_ir(ctx, value, &build_opts)?;
    Ok(render(&ir, &render_opts))
}

/// Build a formatted string from multiple values (handles format string substitution)
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

    let build_opts = options.to_build_options(ctx)?;
    let render_opts = options.to_render_options();

    while let Some((index, arg)) = iter.next() {
        if index == 0 && size > 1 {
            if let Some(str) = arg.as_string() {
                let str = str.to_string()?;

                // Fast check for format strings
                if str.find('%').is_none() {
                    let max_string_length = options.inspect_options.max_string_length;
                    format_raw_string(result, str, false, options.color, max_string_length);
                    continue;
                }

                let bytes = str.as_bytes();
                let mut i = 0;
                let len = bytes.len();

                while i < len {
                    let byte = bytes[i];
                    if byte == b'%' && i + 1 < len {
                        let next_byte = bytes[i + 1];
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
                                    push_byte(result, byte);
                                    continue;
                                },
                                _ => {
                                    push_byte(result, byte);
                                    push_byte(result, next_byte);
                                    continue;
                                },
                            };
                            options.color = false;

                            // Build and render IR for the value
                            let ir = build_ir(ctx, value, &build_opts)?;
                            render_to(result, &ir, &render_opts);

                            options.object_filter = current_filter;
                            continue;
                        }
                        push_byte(result, byte);
                        push_byte(result, next_byte);
                    } else {
                        push_byte(result, byte);
                    }

                    i += 1;
                }
                continue;
            }
        }
        if index != 0 {
            result.push(' ');
        }

        // Build and render IR for the value
        let ir = build_ir(ctx, arg, &build_opts)?;
        render_to(result, &ir, &render_opts);
    }

    Ok(())
}

fn push_byte(result: &mut String, byte: u8) {
    unsafe { result.as_mut_vec() }.push(byte);
}

fn format_raw_string(
    result: &mut String,
    value: String,
    quoted: bool,
    color_enabled: bool,
    max_string_length: usize,
) {
    if quoted {
        if color_enabled {
            result.push_str("\x1b[32m"); // Green
        }
        result.push('\'');
    }

    let char_count = value.chars().count();
    if max_string_length < char_count {
        let truncated: String = value.chars().take(max_string_length).collect();
        result.push_str(&truncated);
        if quoted {
            result.push('\'');
        }
        if color_enabled {
            result.push_str("\x1b[0m"); // Reset
        }
        result.push_str("... ");
        let mut buffer = itoa::Buffer::new();
        result.push_str(buffer.format(char_count - max_string_length));
        result.push_str(" more characters");
    } else {
        result.push_str(&value);
        if quoted {
            result.push('\'');
        }
    }
}

// Re-export get_lossy_string from builder for backwards compatibility
pub use builder::get_lossy_string;

use rquickjs::CaughtError;

pub fn print_error_and_exit<'js>(ctx: &Ctx<'js>, err: CaughtError<'js>) -> ! {
    use std::fmt::Write;

    let mut error_str = String::new();
    write!(error_str, "Error: {:?}", err).unwrap();

    if let Ok(error) = err.into_value(ctx) {
        if print_error(ctx, Rest(vec![error.clone()])).is_err() {
            eprintln!("{}", error_str);
        };
        if cfg!(test) {
            panic!("{:?}", error);
        } else {
            exit(1)
        }
    } else if cfg!(test) {
        panic!("{}", error_str);
    } else {
        eprintln!("{}", error_str);
        exit(1)
    };
}

fn print_error<'js>(ctx: &Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    let is_tty = stderr().is_terminal();
    let mut result = String::new();

    let mut options = FormatOptions::new(ctx, is_tty, true)?;
    build_formatted_string(&mut result, ctx, args, &mut options)?;

    result.push(NEWLINE);

    let _ = stderr().write_all(result.as_bytes());

    Ok(())
}
