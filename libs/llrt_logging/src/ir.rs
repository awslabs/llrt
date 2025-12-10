// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Intermediate Representation (IR) for console/inspect output.
//!
//! This module defines the `PrintIR` enum which represents JavaScript values
//! in a format-agnostic way. The IR is built from JS values first, then
//! rendered to a string by the renderer, allowing formatting decisions
//! (line breaks, indentation, terminal width) to be made during rendering.

/// Represents a number value in the IR
#[derive(Debug, Clone)]
pub enum NumberIR {
    Int(i32),
    Float(f64),
    BigInt(i64),
}

/// Represents a Promise state in the IR
#[derive(Debug, Clone)]
pub enum PromiseStateIR {
    Pending,
    Resolved(Box<PrintIR>),
    Rejected(Box<PrintIR>),
}

/// What kind of truncation occurred
#[derive(Debug, Clone, Copy)]
pub enum TruncatedKind {
    Items,
    Characters,
}

/// Key type for object entries
#[derive(Debug, Clone)]
pub struct ObjectKey {
    pub name: String,
    pub is_numeric: bool,
}

/// Intermediate Representation for JavaScript values.
///
/// This enum captures all the information needed to render a JS value
/// without making formatting decisions. The renderer then uses this
/// IR along with formatting options to produce the final string.
#[derive(Debug, Clone)]
pub enum PrintIR {
    /// null value
    Null,

    /// undefined value
    Undefined,

    /// Boolean value
    Bool(bool),

    /// Numeric value (int, float, or bigint)
    Number(NumberIR),

    /// String value with quoting info
    String {
        value: String,
        /// Whether to quote the string (true for nested strings)
        quoted: bool,
    },

    /// Symbol value
    Symbol(String),

    /// Function or constructor
    Function { name: String, is_class: bool },

    /// Promise with its state
    Promise(PromiseStateIR),

    /// Error object
    Error {
        name: String,
        message: String,
        stack: Option<Vec<String>>,
    },

    /// Date object (pre-formatted ISO string)
    Date(String),

    /// RegExp object
    RegExp { source: String, flags: String },

    /// Array (includes typed arrays)
    Array {
        /// Optional class name for typed arrays (e.g., "Uint8Array")
        class_name: Option<String>,
        /// Array elements
        elements: Vec<PrintIR>,
        /// Total length if truncated
        total_length: usize,
    },

    /// Object with key-value pairs
    Object {
        /// Optional class name (e.g., "Map", "Set", custom class)
        class_name: Option<String>,
        /// Key-value entries
        entries: Vec<(ObjectKey, PrintIR)>,
        /// Total number of entries if truncated
        total_entries: usize,
    },

    /// Circular reference marker
    Circular,

    /// Truncation marker for arrays/strings
    Truncated {
        remaining: usize,
        kind: TruncatedKind,
    },

    /// Custom inspect result (string returned by custom inspect function)
    Custom(String),

    /// Max depth reached placeholder
    MaxDepth { is_array: bool },

    /// A formatted group (used for format string substitutions)
    Group(Vec<PrintIR>),

    /// Raw pre-formatted text (for special cases)
    Raw(String),

    /// A value with a class name prefix (for custom inspect results)
    /// Renders as "ClassName <inner>"
    WithClass {
        class_name: String,
        inner: Box<PrintIR>,
    },
}

impl PrintIR {
    /// Check if this IR represents a "simple" value that should be rendered inline
    pub fn is_simple(&self) -> bool {
        matches!(
            self,
            PrintIR::Null
                | PrintIR::Undefined
                | PrintIR::Bool(_)
                | PrintIR::Number(_)
                | PrintIR::Symbol(_)
                | PrintIR::Date(_)
                | PrintIR::RegExp { .. }
                | PrintIR::Circular
                | PrintIR::MaxDepth { .. }
                | PrintIR::Custom(_)
                | PrintIR::Raw(_)
        )
    }

    /// Estimate the inline length of this IR (for compact formatting decisions)
    pub fn estimate_inline_length(&self) -> Option<usize> {
        match self {
            PrintIR::Null => Some(4),
            PrintIR::Undefined => Some(9),
            PrintIR::Bool(true) => Some(4),
            PrintIR::Bool(false) => Some(5),
            PrintIR::Number(NumberIR::Int(n)) => Some(if *n < 0 {
                ((*n as i64).abs() as f64).log10().floor() as usize + 2
            } else if *n == 0 {
                1
            } else {
                (*n as f64).log10().floor() as usize + 1
            }),
            PrintIR::Number(NumberIR::Float(_)) => Some(10), // Approximate
            PrintIR::Number(NumberIR::BigInt(n)) => {
                Some(
                    if *n < 0 {
                        (n.abs() as f64).log10().floor() as usize + 2
                    } else if *n == 0 {
                        1
                    } else {
                        (*n as f64).log10().floor() as usize + 1
                    } + 1,
                ) // +1 for 'n'
            },
            PrintIR::String { value, quoted } => Some(value.len() + if *quoted { 2 } else { 0 }),
            PrintIR::Symbol(desc) => Some(8 + desc.len()), // "Symbol(" + desc + ")"
            PrintIR::Function { name, is_class } => {
                Some(if *is_class { 9 } else { 12 } + name.len()) // "[class: " or "[function: " + name + "]"
            },
            PrintIR::Date(s) => Some(s.len()),
            PrintIR::RegExp { source, flags } => Some(source.len() + flags.len() + 2), // /source/flags
            PrintIR::Circular => Some(10), // "[Circular]"
            PrintIR::MaxDepth { is_array } => Some(if *is_array { 7 } else { 8 }), // "[Array]" or "[Object]"
            PrintIR::Custom(s) | PrintIR::Raw(s) => Some(s.len()),
            PrintIR::Array {
                class_name,
                elements,
                ..
            } => {
                let prefix_len = class_name.as_ref().map(|n| n.len() + 1).unwrap_or(0);
                let mut total = prefix_len + 2; // brackets
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        total += 2; // ", "
                    }
                    total += elem.estimate_inline_length()?;
                    if total > 200 {
                        // Bail out for very long arrays
                        return None;
                    }
                }
                Some(total)
            },
            PrintIR::Object {
                class_name,
                entries,
                ..
            } => {
                let prefix_len = class_name.as_ref().map(|n| n.len() + 1).unwrap_or(0);
                let mut total = prefix_len + 2; // braces
                for (i, (key, val)) in entries.iter().enumerate() {
                    if i > 0 {
                        total += 2; // ", "
                    }
                    total += key.name.len() + 2; // "key: "
                    total += val.estimate_inline_length()?;
                    if total > 200 {
                        return None;
                    }
                }
                Some(total)
            },
            PrintIR::Promise(_) => None, // Promises are complex
            PrintIR::Error { .. } => None,
            PrintIR::Truncated { .. } => Some(20), // Approximate
            PrintIR::Group(items) => {
                let mut total = 0;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        total += 1; // space
                    }
                    total += item.estimate_inline_length()?;
                }
                Some(total)
            },
            PrintIR::WithClass { class_name, inner } => {
                Some(class_name.len() + 1 + inner.estimate_inline_length()?)
            },
        }
    }
}
