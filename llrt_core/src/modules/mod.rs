// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(not(feature = "lambda"))]
pub use llrt_modules::console;
pub use llrt_modules::{
    abort, assert, async_hooks, buffer, child_process, crypto, dns, events, exceptions, fetch, fs,
    module, navigator, net, os, path, perf_hooks, process, stream_web, string_decoder, timers, tty,
    url, util, zlib,
};
pub use llrt_modules::{module_builder, package, require, CJS_IMPORT_PREFIX, CJS_LOADER_PREFIX};

#[cfg(feature = "lambda")]
pub mod console;
pub mod embedded;
pub mod llrt;
