// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(rust_nightly, feature(array_chunks))]
pub mod bytearray_buffer;
pub mod bytes;
pub mod class;
#[cfg(feature = "ctx")]
pub mod ctx;
#[cfg(feature = "encoding")]
pub mod encoding;
pub mod error;
pub mod error_messages;
#[cfg(feature = "fs")]
pub mod fs;
pub mod macros;
pub mod mc_oneshot;
pub mod module;
pub mod object;
pub mod result;
pub mod sysinfo;
pub mod time;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
