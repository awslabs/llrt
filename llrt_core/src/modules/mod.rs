// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(not(feature = "lambda"))]
pub use llrt_modules::console;
pub use llrt_modules::{
    abort, assert, buffer, child_process, crypto, dns, events, exceptions, fs, http, navigator,
    net, os, path, perf_hooks, process, stream_web, string_decoder, timers, tty, url, util, zlib,
};

#[cfg(feature = "lambda")]
pub mod console;
pub mod llrt;
pub mod module;
pub mod require;
