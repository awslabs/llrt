// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::HashSet;

use llrt_utils::module::ModuleInfo;
use rquickjs::{
    loader::{ModuleLoader, Resolver},
    module::ModuleDef,
    Ctx, Error, Result,
};

#[derive(Debug, Default)]
pub struct ModuleResolver {
    modules: HashSet<String>,
}

impl ModuleResolver {
    #[must_use]
    pub fn with_module<P: Into<String>>(mut self, path: P) -> Self {
        self.modules.insert(path.into());
        self
    }
}

impl Resolver for ModuleResolver {
    fn resolve(&mut self, _: &Ctx<'_>, base: &str, name: &str) -> Result<String> {
        let name = name.trim_start_matches("node:");
        if self.modules.contains(name) {
            Ok(name.into())
        } else {
            Err(Error::new_resolving(base, name))
        }
    }
}

pub type Modules = (
    ModuleResolver,
    ModuleLoader,
    HashSet<&'static str>,
    Vec<fn(&Ctx<'_>) -> Result<()>>,
);

pub struct ModuleBuilder {
    builtin_resolver: ModuleResolver,
    module_loader: ModuleLoader,
    module_names: HashSet<&'static str>,
    init_global: Vec<fn(&Ctx<'_>) -> Result<()>>,
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        let mut builder = Self::new();

        #[cfg(feature = "abort")]
        {
            builder = builder.with_global(crate::modules::abort::init);
        }
        #[cfg(feature = "assert")]
        {
            builder = builder.with_module(crate::modules::assert::AssertModule);
        }
        #[cfg(feature = "buffer")]
        {
            builder = builder
                .with_global(crate::modules::buffer::init)
                .with_module(crate::modules::buffer::BufferModule);
        }
        #[cfg(feature = "child-process")]
        {
            builder = builder.with_module(crate::modules::child_process::ChildProcessModule);
        }
        #[cfg(feature = "console")]
        {
            builder = builder
                .with_global(crate::modules::console::init)
                .with_module(crate::modules::console::ConsoleModule);
        }
        #[cfg(feature = "crypto")]
        {
            builder = builder
                .with_global(crate::modules::crypto::init)
                .with_module(crate::modules::crypto::CryptoModule);
        }
        #[cfg(feature = "dns")]
        {
            builder = builder.with_module(crate::modules::dns::DnsModule);
        }
        #[cfg(feature = "events")]
        {
            builder = builder
                .with_global(crate::modules::events::init)
                .with_module(crate::modules::events::EventsModule);
        }
        #[cfg(feature = "exceptions")]
        {
            builder = builder.with_global(crate::modules::exceptions::init);
        }
        #[cfg(feature = "fs")]
        {
            builder = builder
                .with_module(crate::modules::fs::FsPromisesModule)
                .with_module(crate::modules::fs::FsModule);
        }
        #[cfg(feature = "http")]
        {
            builder = builder.with_global(crate::modules::http::init);
        }
        #[cfg(feature = "navigator")]
        {
            builder = builder.with_global(crate::modules::navigator::init);
        }
        #[cfg(feature = "net")]
        {
            builder = builder.with_module(crate::modules::net::NetModule);
        }
        #[cfg(feature = "os")]
        {
            builder = builder.with_module(crate::modules::os::OsModule);
        }
        #[cfg(feature = "path")]
        {
            builder = builder.with_module(crate::modules::path::PathModule);
        }
        #[cfg(feature = "perf-hooks")]
        {
            builder = builder
                .with_global(crate::modules::perf_hooks::init)
                .with_module(crate::modules::perf_hooks::PerfHooksModule);
        }
        #[cfg(feature = "process")]
        {
            builder = builder
                .with_global(crate::modules::process::init)
                .with_module(crate::modules::process::ProcessModule);
        }
        #[cfg(feature = "stream-web")]
        {
            builder = builder
                .with_global(crate::modules::stream_web::init)
                .with_module(crate::modules::stream_web::StreamWebModule);
        }
        #[cfg(feature = "string-decoder")]
        {
            builder = builder.with_module(crate::modules::string_decoder::StringDecoderModule);
        }
        #[cfg(feature = "timers")]
        {
            builder = builder
                .with_global(crate::modules::timers::init)
                .with_module(crate::modules::timers::TimersModule);
        }
        #[cfg(feature = "tty")]
        {
            builder = builder.with_module(crate::modules::tty::TtyModule);
        }
        #[cfg(feature = "url")]
        {
            builder = builder
                .with_global(crate::modules::url::init)
                .with_module(crate::modules::url::UrlModule);
        }
        #[cfg(feature = "util")]
        {
            builder = builder
                .with_global(crate::modules::util::init)
                .with_module(crate::modules::util::UtilModule);
        }
        #[cfg(feature = "zlib")]
        {
            builder = builder.with_module(crate::modules::zlib::ZlibModule);
        }

        builder
    }
}

impl ModuleBuilder {
    pub fn new() -> Self {
        Self {
            builtin_resolver: ModuleResolver::default(),
            module_loader: ModuleLoader::default(),
            module_names: HashSet::new(),
            init_global: Vec::new(),
        }
    }

    pub fn with_module<M: ModuleDef, I: Into<ModuleInfo<M>>>(mut self, module: I) -> Self {
        let module_info: ModuleInfo<M> = module.into();

        self.builtin_resolver = self.builtin_resolver.with_module(module_info.name);
        self.module_loader = self
            .module_loader
            .with_module(module_info.name, module_info.module);
        self.module_names.insert(module_info.name);

        self
    }

    pub fn with_global(mut self, init_global: fn(&Ctx<'_>) -> Result<()>) -> Self {
        self.init_global.push(init_global);
        self
    }

    pub fn build(self) -> Modules {
        (
            self.builtin_resolver,
            self.module_loader,
            self.module_names,
            self.init_global,
        )
    }
}
