// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub use llrt_modules::{
    abort, assert, buffer, child_process, crypto, dns, events, exceptions, fs, http, navigator,
    net, os, path, perf_hooks, process, stream_web, tty, url, zlib,
};

#[cfg(not(feature = "lambda"))]
pub use llrt_modules::console;

#[cfg(feature = "lambda")]
pub mod console;

pub mod llrt;
pub mod module;
pub mod util;
