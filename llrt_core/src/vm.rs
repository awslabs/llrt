// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{env, result::Result as StdResult};

use ring::rand::SecureRandom;
use rquickjs::{
    context::EvalOptions, loader::FileResolver, prelude::Func, AsyncContext, AsyncRuntime,
    CatchResultExt, Ctx, Error, Result, Value,
};

use crate::libs::{
    context::set_spawn_error_handler,
    hooking::HOOKING_MODE,
    json,
    logging::print_error_and_exit,
    numbers,
    utils::{
        clone::structured_clone,
        primordials::{BasePrimordials, Primordial},
        time,
    },
};
use crate::modules::{
    async_hooks::promise_hook_tracker,
    crypto::SYSTEM_RANDOM,
    embedded::{loader::EmbeddedLoader, resolver::EmbeddedResolver},
    module_builder::ModuleBuilder,
    require::{loader::NpmJsLoader, resolver::NpmJsResolver},
};
use crate::{environment, http, security};

pub struct Vm {
    pub runtime: AsyncRuntime,
    pub ctx: AsyncContext,
}

pub struct VmOptions {
    pub module_builder: ModuleBuilder,
    pub max_stack_size: usize,
    pub gc_threshold_mb: usize,
}

impl Default for VmOptions {
    fn default() -> Self {
        #[allow(unused_mut)]
        let mut module_builder = ModuleBuilder::default()
            .with_global(crate::modules::embedded::init)
            .with_global(crate::modules::module::init)
            .with_module(crate::modules::module::ModuleModule)
            .with_module(crate::modules::llrt::hex::LlrtHexModule)
            .with_module(crate::modules::llrt::util::LlrtUtilModule)
            .with_module(crate::modules::llrt::xml::LlrtXmlModule);

        #[cfg(feature = "lambda")]
        {
            module_builder = module_builder
                .with_global(crate::modules::console::init)
                .with_module(crate::modules::console::ConsoleModule);
        }

        Self {
            module_builder,
            max_stack_size: 512 * 1024,
            gc_threshold_mb: {
                const DEFAULT_GC_THRESHOLD_MB: usize = 20;

                let gc_threshold_mb: usize = env::var(environment::ENV_LLRT_GC_THRESHOLD_MB)
                    .map(|threshold| threshold.parse().unwrap_or(DEFAULT_GC_THRESHOLD_MB))
                    .unwrap_or(DEFAULT_GC_THRESHOLD_MB);

                gc_threshold_mb * 1024 * 1024
            },
        }
    }
}

impl Vm {
    pub const ENV_LAMBDA_TASK_ROOT: &'static str = "LAMBDA_TASK_ROOT";

    pub async fn from_options(
        vm_options: VmOptions,
    ) -> StdResult<Self, Box<dyn std::error::Error + Send + Sync>> {
        time::init();
        http::init()?;
        security::init()?;

        SYSTEM_RANDOM
            .fill(&mut [0; 8])
            .expect("Failed to initialize SystemRandom");

        let mut file_resolver = FileResolver::default();
        let mut paths: Vec<&str> = Vec::with_capacity(10);

        paths.push(".");

        let task_root = env::var(Self::ENV_LAMBDA_TASK_ROOT).unwrap_or_else(|_| String::from(""));
        let task_root = task_root.as_str();
        if cfg!(debug_assertions) {
            paths.push("bundle");
        } else {
            paths.push("/opt");
        }

        if !task_root.is_empty() {
            paths.push(task_root);
        }

        for path in paths.iter() {
            file_resolver.add_path(*path);
        }

        let (module_resolver, module_loader, global_attachment) = vm_options.module_builder.build();
        let resolver = (
            module_resolver,
            EmbeddedResolver,
            NpmJsResolver,
            file_resolver,
        );
        let loader = (module_loader, EmbeddedLoader, NpmJsLoader);

        let runtime = AsyncRuntime::new()?;
        runtime.set_max_stack_size(vm_options.max_stack_size).await;
        runtime.set_gc_threshold(vm_options.gc_threshold_mb).await;
        runtime.set_loader(resolver, loader).await;

        let ctx = AsyncContext::full(&runtime).await?;
        ctx.with(|ctx| {
            (|| {
                BasePrimordials::init(&ctx)?;
                global_attachment.attach(&ctx)?;
                self::init(&ctx)?;
                Ok(())
            })()
            .catch(&ctx)
            .unwrap_or_else(|err| print_error_and_exit(&ctx, err));
            Ok::<_, Error>(())
        })
        .await?;

        if HOOKING_MODE.to_owned() {
            runtime.set_promise_hook(Some(promise_hook_tracker())).await;
        }

        Ok(Vm { runtime, ctx })
    }

    pub async fn new() -> StdResult<Self, Box<dyn std::error::Error + Send + Sync>> {
        let vm = Self::from_options(VmOptions::default()).await?;
        Ok(vm)
    }

    pub async fn run_with<F>(&self, f: F)
    where
        F: for<'js> FnOnce(&Ctx<'js>) -> Result<()> + std::marker::Send,
    {
        self.ctx
            .with(|ctx| {
                if let Err(err) = f(&ctx).catch(&ctx) {
                    print_error_and_exit(&ctx, err);
                }
            })
            .await;
    }

    pub async fn run<S: Into<Vec<u8>> + Send>(&self, source: S, strict: bool, global: bool) {
        self.run_with(|ctx| {
            let mut options = EvalOptions::default();
            options.strict = strict;
            options.promise = true;
            options.global = global;
            let _ = ctx.eval_with_options::<Value, _>(source, options)?;
            Ok::<_, Error>(())
        })
        .await;
    }

    pub async fn run_file(&self, filename: impl AsRef<str>, strict: bool, global: bool) {
        let source = [
            r#"import(""#,
            &filename.as_ref().replace('\\', "/"),
            r#"").catch((e) => {console.error(e);process.exit(1)})"#,
        ]
        .concat();

        self.run(source, strict, global).await;
    }

    pub async fn run_bytecode(&self, bytecode: &[u8]) {
        self.run_with(|ctx| {
            EmbeddedLoader::load_bytecode_module(ctx.clone(), bytecode)
                .map(|module| module.eval())
                .map_err(|err| {
                    eprintln!("Failed to evaluate module: {err:?}");
                    err
                })
                .map(|_| ())
        })
        .await;
    }

    pub async fn idle(self) -> StdResult<(), Box<dyn std::error::Error + Sync + Send>> {
        self.runtime.idle().await;
        Ok(())
    }
}

fn init(ctx: &Ctx<'_>) -> Result<()> {
    set_spawn_error_handler(|ctx, err| {
        print_error_and_exit(ctx, err);
    });

    let globals = ctx.globals();

    globals.set("__gc", Func::from(|ctx: Ctx| ctx.run_gc()))?;
    globals.set("global", ctx.globals())?;
    globals.set("self", ctx.globals())?;
    globals.set(
        "structuredClone",
        Func::from(|ctx, value, options| structured_clone(&ctx, value, options)),
    )?;

    numbers::redefine_prototype(ctx)?;
    json::redefine_static_methods(ctx)?;

    Ok(())
}
