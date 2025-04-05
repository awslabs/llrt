// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{env, result::Result as StdResult};

use ring::rand::SecureRandom;
use rquickjs::{
    context::EvalOptions, loader::FileResolver, prelude::Func, AsyncContext, AsyncRuntime,
    CatchResultExt, Ctx, Error, Object, Result, Value,
};

use crate::libs::{
    context::set_spawn_error_handler,
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
    crypto::SYSTEM_RANDOM,
    llrt::{hex::LlrtHexModule, util::LlrtUtilModule, uuid::LlrtUuidModule, xml::LlrtXmlModule},
    module::{self, ModuleModule},
    module_builder::ModuleBuilder,
    require::{loader::CustomLoader, resolver::CustomResolver},
};
use crate::{environment, http, security};

pub struct Vm {
    pub runtime: AsyncRuntime,
    pub ctx: AsyncContext,
}

#[allow(dead_code)]
struct ExportArgs<'js>(Ctx<'js>, Object<'js>, Value<'js>, Value<'js>);

pub struct VmOptions {
    pub module_builder: ModuleBuilder,
    pub max_stack_size: usize,
    pub gc_threshold_mb: usize,
}

impl Default for VmOptions {
    fn default() -> Self {
        let module_builder = ModuleBuilder::default()
            .with_module(ModuleModule)
            .with_module(LlrtHexModule)
            .with_module(LlrtUtilModule)
            .with_module(LlrtUuidModule)
            .with_module(LlrtXmlModule);

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

        let (builtin_resolver, module_loader, module_names, init_globals) =
            vm_options.module_builder.build();

        let resolver = (builtin_resolver, CustomResolver, file_resolver);

        let loader = (module_loader, CustomLoader);

        let runtime = AsyncRuntime::new()?;
        runtime.set_max_stack_size(vm_options.max_stack_size).await;
        runtime.set_gc_threshold(vm_options.gc_threshold_mb).await;
        runtime.set_loader(resolver, loader).await;

        let ctx = AsyncContext::full(&runtime).await?;
        ctx.with(|ctx| {
            (|| {
                for init_global in init_globals {
                    init_global(&ctx)?;
                }
                self::init(&ctx)?;
                module::init(&ctx, module_names)?;
                Ok(())
            })()
            .catch(&ctx)
            .unwrap_or_else(|err| print_error_and_exit(&ctx, err));
            Ok::<_, Error>(())
        })
        .await?;

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

    //init base primordials
    let _ = BasePrimordials::get(ctx)?;

    Ok(())
}
