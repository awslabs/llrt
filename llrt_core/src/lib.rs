// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::new_without_default)]
#![allow(clippy::inherent_to_string)]
#![cfg_attr(rust_nightly, feature(portable_simd))]

pub mod bytecode;
pub mod compiler;
mod compiler_common;
pub mod environment;
mod http;
pub mod libs;
mod module_builder;
pub mod modules;
pub mod runtime_client;
mod security;
pub mod utils;
pub mod vm;

pub use llrt_modules::VERSION;

pub use rquickjs::*;
