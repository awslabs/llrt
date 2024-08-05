// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::new_without_default)]
#![allow(clippy::inherent_to_string)]
pub use self::module_info::ModuleInfo;
pub use self::modules::*;

mod module_info;
mod modules;
#[cfg(feature = "__stream")]
pub mod stream;
mod sysinfo;
#[cfg(test)]
mod test;
pub mod time;
mod utils;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
