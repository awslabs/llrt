use std::collections::HashSet;

use crate::modules::{
    assert::AssertModule,
    buffer::BufferModule,
    child_process::ChildProcessModule,
    console::ConsoleModule,
    crypto::CryptoModule,
    events::EventsModule,
    fs::{FsModule, FsPromisesModule},
    llrt::{hex::LlrtHexModule, uuid::LlrtUuidModule, xml::LlrtXmlModule},
    module::ModuleModule,
    net::NetModule,
    os::OsModule,
    path::PathModule,
    perf_hooks::PerfHooksModule,
    process::ProcessModule,
    stream_web::StreamWebModule,
    tty::TtyModule,
    url::UrlModule,
    util::UtilModule,
    zlib::ZlibModule,
};
use llrt_modules::timers::TimersModule;
pub use llrt_utils::module::ModuleInfo;
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
        Self::new()
            .with_module(AssertModule)
            .with_module(CryptoModule)
            .with_global(crate::modules::crypto::init)
            .with_global(crate::modules::util::init)
            .with_module(FsPromisesModule)
            .with_module(FsModule)
            .with_module(OsModule)
            .with_module(TimersModule)
            .with_global(llrt_modules::timers::init)
            .with_module(EventsModule)
            .with_global(crate::modules::events::init)
            .with_global(crate::modules::abort::init)
            .with_module(ModuleModule)
            .with_module(NetModule)
            .with_module(ConsoleModule)
            .with_global(crate::modules::console::init)
            .with_module(PathModule)
            .with_module(BufferModule)
            .with_global(crate::modules::buffer::init)
            .with_module(ChildProcessModule)
            .with_module(UtilModule)
            .with_module(ProcessModule)
            .with_global(crate::modules::process::init)
            .with_global(crate::modules::navigator::init)
            .with_global(crate::modules::url::init)
            .with_module(UrlModule)
            .with_module(StreamWebModule)
            .with_global(crate::modules::http::init)
            .with_global(crate::modules::exceptions::init)
            .with_module(LlrtHexModule)
            .with_module(LlrtUuidModule)
            .with_module(LlrtXmlModule)
            .with_module(PerfHooksModule)
            .with_global(crate::modules::perf_hooks::init)
            .with_module(ZlibModule)
            .with_module(TtyModule)
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
