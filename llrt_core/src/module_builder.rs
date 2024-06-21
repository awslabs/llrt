use std::collections::HashSet;

use crate::modules::{
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
    util::UtilModule,
    uuid::UuidModule,
    xml::XmlModule,
};
use rquickjs::{
    loader::{BuiltinResolver, ModuleLoader, Resolver},
    module::ModuleDef,
    Ctx, Result,
};

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

pub type Modules = (
    ModuleResolver,
    ModuleLoader,
    HashSet<&'static str>,
    Vec<fn(&Ctx<'_>) -> Result<()>>,
);

pub struct ModuleInfo<T: ModuleDef> {
    pub name: &'static str,
    pub module: T,
}

pub struct ModuleBuilder {
    builtin_resolver: ModuleResolver,
    module_loader: ModuleLoader,
    module_names: HashSet<&'static str>,
    init_global: Vec<fn(&Ctx<'_>) -> Result<()>>,
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
            .with_module(CryptoModule)
            .with_global(crate::modules::crypto::init)
            .with_module(HexModule)
            .with_global(crate::modules::encoding::init)
            .with_module(FsPromisesModule)
            .with_module(FsModule)
            .with_module(OsModule)
            .with_module(TimersModule)
            .with_global(crate::modules::timers::init)
            .with_module(EventsModule)
            .with_global(crate::modules::events::init)
            .with_module(ModuleModule)
            .with_module(NetModule)
            .with_module(ConsoleModule)
            .with_global(crate::modules::console::init)
            .with_module(PathModule)
            .with_module(XmlModule)
            .with_module(BufferModule)
            .with_global(crate::modules::buffer::init)
            .with_module(ChildProcessModule)
            .with_module(UtilModule)
            .with_module(UuidModule)
            .with_module(ProcessModule)
            .with_global(crate::modules::process::init)
            .with_module(NavigatorModule)
            .with_global(crate::modules::navigator::init)
            .with_module(UrlModule)
            .with_module(PerformanceModule)
            .with_global(crate::modules::performance::init)
            .with_global(crate::modules::http::init)
            .with_global(crate::modules::exceptions::init)
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
