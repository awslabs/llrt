#![allow(clippy::new_without_default)]
#![allow(clippy::inherent_to_string)]
#![cfg_attr(rust_nightly, feature(portable_simd))]

#[macro_use]
mod macros;
mod bytecode;
// #[cfg(not(feature = "lambda"))]
pub mod compiler;
// #[cfg(not(feature = "lambda"))]
mod compiler_common;
pub mod environment;
pub mod json;
// mod minimal_tracer;
mod module_builder;
pub mod modules;
pub mod number;
pub mod runtime_client;
mod security;
mod test_utils;
pub mod utils;
pub mod vm;

pub use llrt_modules::VERSION;

pub use rquickjs::{async_with, AsyncContext, CatchResultExt, Module, Value};
