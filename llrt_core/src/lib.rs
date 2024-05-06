#![allow(clippy::new_without_default)]
#![allow(clippy::inherent_to_string)]
#![cfg_attr(feature = "nightly", feature(portable_simd))]

#[macro_use]
mod macros;
mod bytearray_buffer;
mod bytecode;
// #[cfg(not(feature = "lambda"))]
pub mod compiler;
// #[cfg(not(feature = "lambda"))]
mod compiler_common;
pub mod environment;
mod json;
// mod minimal_tracer;
mod module_builder;
pub mod modules;
mod number;
pub mod runtime_client;
mod security;
mod stream;
mod test_utils;
pub mod utils;
pub mod vm;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use rquickjs::{async_with, AsyncContext, CatchResultExt, Module};
