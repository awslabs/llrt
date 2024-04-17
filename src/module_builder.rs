use std::collections::HashSet;

use rquickjs::{loader::{BuiltinResolver, ModuleLoader, Resolver}, module::ModuleDef, Ctx, Result};
use crate::{modules::{
    buffer::BufferModule,
    child_process::ChildProcessModule,
    console::ConsoleModule,
    crypto::CryptoModule,
    encoding::HexModule,
    events::EventsModule,
    fs::{FsModule, FsPromisesModule},
    module::ModuleModule,
    navigator::NavigatorModule,
    net::NetModule,
    os::OsModule,
    path::PathModule,
    performance::PerformanceModule,
    process::ProcessModule,
    timers::TimersModule,
    url::UrlModule,
    uuid::UuidModule,
    xml::XmlModule,
}, utils::UtilModule};

#[derive(Debug, Default)]
pub struct ModuleResolver {
    builtin_resolver: BuiltinResolver,
}

impl ModuleResolver {
    #[must_use]
    pub fn with_module<P: Into<String>>(mut self, path: P) -> Self {
        self.builtin_resolver.add_module(path.into());
        self
    }
}

impl Resolver for ModuleResolver {
    fn resolve(&mut self, ctx: &Ctx<'_>, base: &str, name: &str) -> Result<String> {
        // Strip node prefix so that we support both with and without
        let name = name.strip_prefix("node:").unwrap_or(name);

        self.builtin_resolver.resolve(ctx, base, name)
    }
}

struct ModuleInfo<T: ModuleDef> {
    name: &'static str,
    module: T,
}

pub struct ModuleBuilder {
    builtin_resolver: ModuleResolver,
    module_loader: ModuleLoader,
    module_names: HashSet<&'static str>,
    init_global: Vec<fn(&Ctx<'_>) -> Result<()>>
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

    pub fn with_default() -> Self {
        Self::new()
            .with_module("crypto", CryptoModule)
            .with_module("hex", HexModule)
            .with_global(crate::modules::encoding::init)
            .with_module("fs/promises", FsPromisesModule)
            .with_module("fs", FsModule)
            .with_module("os", OsModule)
            .with_module("timers", TimersModule)
            .with_global(crate::modules::timers::init)
            .with_module("events", EventsModule)
            .with_global(crate::modules::events::init)
            .with_module("module", ModuleModule)
            .with_module("net", NetModule)
            .with_module("console", ConsoleModule)
            .with_global(crate::modules::console::init)
            .with_module("path", PathModule)
            .with_module("xml", XmlModule)
            .with_module("buffer", BufferModule)
            .with_global(crate::modules::buffer::init)
            .with_module("child_process", ChildProcessModule)
            .with_module("util", UtilModule)
            .with_module("uuid", UuidModule)
            .with_module("process", ProcessModule)
            .with_global(crate::modules::process::init)
            .with_module("navigator", NavigatorModule)
            .with_global(crate::modules::navigator::init)
            .with_module("url", UrlModule)
            .with_module("performance", PerformanceModule)
            .with_global(crate::modules::performance::init)
            .with_global(crate::modules::http::init)
            .with_global(crate::modules::exceptions::init)
    }

    pub fn with_module<M: ModuleDef>(mut self, name: &'static str, module: M) -> Self {
        let module_info = ModuleInfo {
            name,
            module,
        };

        self.builtin_resolver = self.builtin_resolver.with_module(module_info.name);
        self.module_loader = self.module_loader.with_module(module_info.name, module_info.module);
        self.module_names.insert(module_info.name);

        self
    }

    pub fn with_global(mut self, init_global: fn(&Ctx<'_>) -> Result<()>) -> Self {
        self.init_global.push(init_global);
        self
    }

    pub fn build(self) -> (ModuleResolver, ModuleLoader, HashSet<&'static str>, Vec<fn(&Ctx<'_>) -> Result<()>>){
        (
            self.builtin_resolver,
            self.module_loader,
            self.module_names,
            self.init_global
        )
    }    
}

