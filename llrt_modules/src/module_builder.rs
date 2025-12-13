// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::HashSet;

use llrt_utils::module::ModuleInfo;
use rquickjs::{module::ModuleDef, Ctx, Result};

use crate::module::{loader::ModuleLoader, resolver::ModuleResolver, ModuleNames};

#[derive(Debug, Default)]
pub struct GlobalAttachment {
    names: HashSet<String>,
    functions: Vec<fn(&Ctx<'_>) -> Result<()>>,
}

impl GlobalAttachment {
    pub fn add_function(mut self, init: fn(&Ctx<'_>) -> Result<()>) -> Self {
        self.functions.push(init);
        self
    }

    pub fn add_name<P: Into<String>>(mut self, path: P) -> Self {
        self.names.insert(path.into());
        self
    }

    pub fn attach(self, ctx: &Ctx<'_>) -> Result<()> {
        if !self.names.is_empty() {
            let _ = ctx.store_userdata(ModuleNames::new(self.names));
        }
        for init in self.functions {
            init(ctx)?;
        }
        Ok(())
    }
}

pub struct ModuleBuilder {
    module_resolver: ModuleResolver,
    module_loader: ModuleLoader,
    global_attachment: GlobalAttachment,
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        let mut builder = Self::new();

        builder = builder
            .with_global(crate::module::init)
            .with_module(crate::module::ModuleModule);

        #[cfg(feature = "abort")]
        {
            builder = builder.with_global(crate::modules::abort::init);
        }
        #[cfg(feature = "assert")]
        {
            builder = builder.with_module(crate::modules::assert::AssertModule);
        }
        #[cfg(feature = "async-hooks")]
        {
            builder = builder
                .with_global(crate::modules::async_hooks::init)
                .with_module(crate::modules::async_hooks::AsyncHooksModule);
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
        #[cfg(feature = "dgram")]
        {
            builder = builder.with_module(crate::modules::dgram::DgramModule);
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
        #[cfg(feature = "https")]
        {
            builder = builder.with_module(crate::modules::https::HttpsModule);
        }
        #[cfg(feature = "fetch")]
        {
            builder = builder.with_global(crate::modules::fetch::init);
        }
        #[cfg(feature = "fs")]
        {
            builder = builder
                .with_module(crate::modules::fs::FsPromisesModule)
                .with_module(crate::modules::fs::FsModule);
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
        #[cfg(feature = "tls")]
        {
            builder = builder.with_module(crate::modules::tls::TlsModule);
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
            module_resolver: ModuleResolver::default(),
            module_loader: ModuleLoader::default(),
            global_attachment: GlobalAttachment::default(),
        }
    }

    pub fn with_module<M: ModuleDef, I: Into<ModuleInfo<M>>>(mut self, module: I) -> Self {
        let module_info: ModuleInfo<M> = module.into();

        self.module_resolver = self.module_resolver.add_name(module_info.name);
        self.module_loader = self
            .module_loader
            .with_module(module_info.name, module_info.module);
        self.global_attachment = self.global_attachment.add_name(module_info.name);
        self
    }

    pub fn with_global(mut self, init: fn(&Ctx<'_>) -> Result<()>) -> Self {
        self.global_attachment = self.global_attachment.add_function(init);
        self
    }

    pub fn build(self) -> (ModuleResolver, ModuleLoader, GlobalAttachment) {
        (
            self.module_resolver,
            self.module_loader,
            self.global_attachment,
        )
    }
}
