// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::new_without_default)]
pub use self::module_info::ModuleInfo;
pub use self::modules::*;
use std::sync::atomic::AtomicUsize;

mod module_info;
mod modules;
#[cfg(feature = "__stream")]
pub mod stream;
#[cfg(test)]
mod test;
mod utils;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub static TIME_ORIGIN: AtomicUsize = AtomicUsize::new(0);
