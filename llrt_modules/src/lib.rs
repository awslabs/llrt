// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::HashSet, env, marker::PhantomData};

use rquickjs::JsLifetime;

pub mod module_builder;

pub use self::modules::*;

mod modules {
    #[cfg(feature = "abort")]
    pub use llrt_abort as abort;
    #[cfg(feature = "assert")]
    pub use llrt_assert as assert;
    #[cfg(feature = "async-hooks")]
    pub use llrt_async_hooks as async_hooks;
    #[cfg(feature = "buffer")]
    pub use llrt_buffer as buffer;
    #[cfg(feature = "child-process")]
    pub use llrt_child_process as child_process;
    #[cfg(feature = "console")]
    pub use llrt_console as console;
    #[cfg(feature = "crypto")]
    pub use llrt_crypto as crypto;
    #[cfg(feature = "dns")]
    pub use llrt_dns as dns;
    #[cfg(feature = "events")]
    pub use llrt_events as events;
    #[cfg(feature = "exceptions")]
    pub use llrt_exceptions as exceptions;
    #[cfg(feature = "fetch")]
    pub use llrt_fetch as fetch;
    #[cfg(feature = "fs")]
    pub use llrt_fs as fs;
    #[cfg(feature = "https")]
    pub use llrt_http as https;
    #[cfg(feature = "navigator")]
    pub use llrt_navigator as navigator;
    #[cfg(feature = "net")]
    pub use llrt_net as net;
    #[cfg(feature = "os")]
    pub use llrt_os as os;
    #[cfg(feature = "path")]
    pub use llrt_path as path;
    #[cfg(feature = "perf-hooks")]
    pub use llrt_perf_hooks as perf_hooks;
    #[cfg(feature = "process")]
    pub use llrt_process as process;
    #[cfg(feature = "stream-web")]
    pub use llrt_stream_web as stream_web;
    #[cfg(feature = "string-decoder")]
    pub use llrt_string_decoder as string_decoder;
    #[cfg(feature = "timers")]
    pub use llrt_timers as timers;
    #[cfg(feature = "tls")]
    pub use llrt_tls as tls;
    #[cfg(feature = "tty")]
    pub use llrt_tty as tty;
    #[cfg(feature = "url")]
    pub use llrt_url as url;
    #[cfg(feature = "util")]
    pub use llrt_util as util;
    #[cfg(feature = "zlib")]
    pub use llrt_zlib as zlib;
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ModuleNames<'js> {
    list: HashSet<String>,
    _marker: PhantomData<&'js ()>,
}

unsafe impl<'js> JsLifetime<'js> for ModuleNames<'js> {
    type Changed<'to> = ModuleNames<'to>;
}

impl ModuleNames<'_> {
    pub fn new(names: HashSet<String>) -> Self {
        Self {
            list: names,
            _marker: PhantomData,
        }
    }

    pub fn get_list(&self) -> HashSet<String> {
        self.list.clone()
    }
}
