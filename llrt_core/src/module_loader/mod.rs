// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use once_cell::sync::Lazy;

use crate::environment;

pub mod loader;
pub mod resolver;

// added when .cjs files are imported
pub const CJS_IMPORT_PREFIX: &str = "__cjs:";
// added to force CJS imports in loader
pub const CJS_LOADER_PREFIX: &str = "__cjsm:";

pub static LLRT_PLATFORM: Lazy<String> = Lazy::new(|| {
    let platform =
        env::var(environment::ENV_LLRT_PLATFORM).unwrap_or_else(|_| "browser".to_string());
    if platform == "node" {
        "node".to_string()
    } else {
        "browser".to_string()
    }
});
