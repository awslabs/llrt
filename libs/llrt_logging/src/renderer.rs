// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Renderer - converts PrintIR to formatted strings.
//!
//! This module handles the second phase of the two-phase approach:
//! taking the intermediate representation and rendering it to a string
//! with appropriate formatting (indentation, line breaks, colors, etc.)
//! based on terminal width and other options.

use llrt_numbers::float_to_string;

use crate::ir::{NumberIR, PrintIR, PromiseStateIR, TruncatedKind};

pub const NEWLINE: char = '\n';
pub const CARRIAGE_RETURN: char = '\r';
const SPACING: char = ' ';
const MAX_INDENTATION_LEVEL: usize = 4;
const INDENTATION_LOOKUP: [&str; MAX_INDENTATION_LEVEL + 1] =
    ["", "  ", "    ", "      ", "        "];

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

/// Options controlling how the IR is rendered to a string
#[derive(Clone)]
pub struct RenderOptions {
    /// Whether to use ANSI colors
    pub colors: bool,
    /// Whether to use newlines (true) or carriage returns (false)
    pub newline: bool,
    /// Line length for breaking (default 80)
    pub break_length: usize,
    /// Compact output level (default 3)
    /// - 0: Always multiline
    /// - 1-N: Try inline up to this depth
    pub compact: usize,
    /// Whether to use breakLength/compact heuristics for line breaking.
    /// When false (console mode), uses simple depth-based multiline.
    /// When true (util.inspect mode), uses breakLength/compact to decide.
    pub use_break_heuristics: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            colors: false,
            newline: true,
            break_length: 80,
            compact: 3,
            use_break_heuristics: false,
        }
    }
}

impl RenderOptions {
    /// Create options for util.inspect mode
    pub fn for_inspect() -> Self {
        Self {
            use_break_heuristics: true,
            ..Self::default()
        }
    }

    /// Create options for console mode
    pub fn for_console(newline: bool) -> Self {
        Self {
            newline,
            use_break_heuristics: false,
            ..Self::default()
        }
    }
}

/// Render PrintIR to a formatted string
pub fn render(ir: &PrintIR, options: &RenderOptions) -> String {
    let mut result = String::with_capacity(128);
    render_inner(&mut result, ir, options, 0);
    result
}

/// Render PrintIR to an existing string buffer
pub fn render_to(result: &mut String, ir: &PrintIR, options: &RenderOptions) {
    render_inner(result, ir, options, 0);
}

fn render_inner(result: &mut String, ir: &PrintIR, options: &RenderOptions, depth: usize) {
    let color = options.colors;

    match ir {
        PrintIR::Null => {
            if color {
                Color::BOLD.push(result);
            }
            result.push_str("null");
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Undefined => {
            if color {
                Color::BLACK.push(result);
            }
            result.push_str("undefined");
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Bool(b) => {
            if color {
                Color::YELLOW.push(result);
            }
            result.push_str(if *b { "true" } else { "false" });
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Number(num) => {
            if color {
                Color::YELLOW.push(result);
            }
            match num {
                NumberIR::Int(n) => {
                    let mut buffer = itoa::Buffer::new();
                    result.push_str(buffer.format(*n));
                },
                NumberIR::Float(f) => {
                    result.push_str(&float_to_string(*f));
                },
                NumberIR::BigInt(n) => {
                    let mut buffer = itoa::Buffer::new();
                    result.push_str(buffer.format(*n));
                    result.push('n');
                },
            }
            if color {
                Color::reset(result);
            }
        },
        PrintIR::String { value, quoted } => {
            if *quoted {
                if color {
                    Color::GREEN.push(result);
                }
                result.push('\'');
                result.push_str(value);
                result.push('\'');
                if color {
                    Color::reset(result);
                }
            } else {
                result.push_str(value);
            }
        },
        PrintIR::Symbol(desc) => {
            if color {
                Color::YELLOW.push(result);
            }
            result.push_str("Symbol(");
            result.push_str(desc);
            result.push(')');
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Function { name, is_class } => {
            if color {
                Color::CYAN.push(result);
            }
            result.push_str(if *is_class { "[class: " } else { "[function: " });
            result.push_str(name);
            result.push(']');
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Promise(state) => {
            result.push_str("Promise {");

            let is_pending = matches!(state, PromiseStateIR::Pending);
            let apply_indentation = depth < 2 && !is_pending;

            write_sep(result, false, apply_indentation, options.newline);
            if apply_indentation {
                push_indentation(result, depth + 1);
            }

            match state {
                PromiseStateIR::Pending => {
                    if color {
                        Color::CYAN.push(result);
                    }
                    result.push_str("<pending>");
                    if color {
                        Color::reset(result);
                    }
                },
                PromiseStateIR::Resolved(inner) => {
                    render_inner(result, inner, options, depth + 1);
                },
                PromiseStateIR::Rejected(inner) => {
                    if color {
                        Color::RED.push(result);
                    }
                    result.push_str("<rejected> ");
                    if color {
                        Color::reset(result);
                    }
                    render_inner(result, inner, options, depth + 1);
                },
            }

            write_sep(result, false, apply_indentation, options.newline);
            if apply_indentation {
                push_indentation(result, depth);
            }
            result.push('}');
        },
        PrintIR::Error {
            name,
            message,
            stack,
        } => {
            result.push_str(name);
            result.push_str(": ");
            result.push_str(message);

            if color {
                Color::BLACK.push(result);
            }

            if let Some(stack_lines) = stack {
                for line in stack_lines {
                    result.push(if options.newline {
                        NEWLINE
                    } else {
                        CARRIAGE_RETURN
                    });
                    push_indentation(result, depth + 1);
                    result.push_str(line);
                }
            }

            if color {
                Color::reset(result);
            }
        },
        PrintIR::Date(iso_string) => {
            if color {
                Color::MAGENTA.push(result);
            }
            result.push_str(iso_string);
            if color {
                Color::reset(result);
            }
        },
        PrintIR::RegExp { source, flags } => {
            if color {
                Color::RED.push(result);
            }
            result.push('/');
            result.push_str(source);
            result.push('/');
            result.push_str(flags);
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Array {
            class_name,
            elements,
            total_length,
        } => {
            if let Some(cn) = class_name {
                result.push_str(cn);
                result.push(SPACING);
            }

            result.push('[');

            let should_inline = should_format_inline(ir, options, depth);

            for (i, elem) in elements.iter().enumerate() {
                write_sep(result, i > 0, !should_inline, options.newline);

                if !should_inline {
                    push_indentation(result, depth + 1);
                }
                if depth > MAX_INDENTATION_LEVEL - 1 && !should_inline {
                    result.push(SPACING);
                }

                render_inner(result, elem, options, depth + 1);

                // Check for truncation
                if i == elements.len() - 1 && *total_length > elements.len() {
                    result.push_str(", ... ");
                    let mut buffer = itoa::Buffer::new();
                    result.push_str(buffer.format(total_length - elements.len()));
                    result.push_str(" more items");
                }
            }

            if !elements.is_empty() {
                if !should_inline {
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

            result.push(']');
        },
        PrintIR::Object {
            class_name,
            entries,
            total_entries,
        } => {
            if let Some(cn) = class_name {
                result.push_str(cn);
                result.push(SPACING);
            }

            result.push('{');

            let should_inline = should_format_inline(ir, options, depth);

            for (i, (key, val)) in entries.iter().enumerate() {
                write_sep(result, i > 0, !should_inline, options.newline);

                if !should_inline {
                    push_indentation(result, depth + 1);
                }
                if depth > MAX_INDENTATION_LEVEL - 1 && !should_inline {
                    result.push(SPACING);
                }

                // Render key
                if key.is_numeric {
                    if color {
                        Color::GREEN.push(result);
                    }
                    result.push('\'');
                    result.push_str(&key.name);
                    result.push('\'');
                    if color {
                        Color::reset(result);
                    }
                } else {
                    result.push_str(&key.name);
                }

                result.push(':');
                result.push(SPACING);

                // Render value
                render_inner(result, val, options, depth + 1);

                // Check for truncation
                if i == entries.len() - 1 && *total_entries > entries.len() {
                    result.push_str(", ... ");
                    let mut buffer = itoa::Buffer::new();
                    result.push_str(buffer.format(total_entries - entries.len()));
                    result.push_str(" more items");
                }
            }

            if !entries.is_empty() {
                if !should_inline {
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

            result.push('}');
        },
        PrintIR::Circular => {
            if color {
                Color::CYAN.push(result);
            }
            result.push_str("[Circular]");
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Truncated { remaining, kind } => {
            result.push_str("... ");
            let mut buffer = itoa::Buffer::new();
            result.push_str(buffer.format(*remaining));
            match kind {
                TruncatedKind::Items => result.push_str(" more items"),
                TruncatedKind::Characters => result.push_str(" more characters"),
            }
        },
        PrintIR::Custom(s) | PrintIR::Raw(s) => {
            result.push_str(s);
        },
        PrintIR::MaxDepth { is_array } => {
            if color {
                Color::CYAN.push(result);
            }
            result.push_str(if *is_array { "[Array]" } else { "[Object]" });
            if color {
                Color::reset(result);
            }
        },
        PrintIR::Group(items) => {
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    result.push(SPACING);
                }
                render_inner(result, item, options, depth);
            }
        },
        PrintIR::WithClass { class_name, inner } => {
            // Render class name followed by the inner content
            result.push_str(class_name);
            result.push(SPACING);
            render_inner(result, inner, options, depth);
        },
    }
}

/// Determine if an IR should be formatted inline (compact) or multiline
fn should_format_inline(ir: &PrintIR, options: &RenderOptions, depth: usize) -> bool {
    if options.use_break_heuristics {
        // util.inspect mode: Use breakLength and compact options to decide
        let compact = options.compact;

        // If compact is 0, never format inline
        if compact == 0 {
            return false;
        }

        // If we're beyond the compact depth, always try to format inline
        if depth >= compact {
            return true;
        }

        // For arrays, default to inline unless very long
        if matches!(ir, PrintIR::Array { .. }) {
            if let Some(len) = ir.estimate_inline_length() {
                return len <= options.break_length;
            }
            return false;
        }

        // For objects at depths less than compact, check if inline would fit
        if let Some(inline_len) = ir.estimate_inline_length() {
            let current_line_approx = depth * 2;
            let remaining_length = options.break_length.saturating_sub(current_line_approx);
            inline_len <= remaining_length
        } else {
            false
        }
    } else {
        // Console mode: Simple depth-based logic
        // Arrays always inline, objects multiline only at shallow depths
        match ir {
            PrintIR::Array { .. } => true,
            PrintIR::Object { .. } => depth >= 2,
            _ => true,
        }
    }
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
    if depth <= MAX_INDENTATION_LEVEL {
        result.push_str(INDENTATION_LOOKUP[depth]);
    } else {
        // For deeper levels, just use max indentation
        result.push_str(INDENTATION_LOOKUP[MAX_INDENTATION_LEVEL]);
    }
}

/// Replace newlines with carriage returns (for Lambda log format)
pub fn replace_newline_with_carriage_return(result: &mut str) {
    let str_bytes = unsafe { result.as_bytes_mut() };
    let mut pos = 0;
    while let Some(index) = str_bytes[pos..].iter().position(|b| *b == b'\n') {
        str_bytes[pos + index] = b'\r';
        pos += index + 1;
    }
}
