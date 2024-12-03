// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cmp::min, env, fmt::Write, process::exit, result::Result as StdResult};

use llrt_json::{parse::json_parse, stringify::json_stringify_replacer_space};
use llrt_numbers::number_to_string;
use llrt_utils::{
    bytes::ObjectBytes,
    error::ErrorExtensions,
    object::ObjectExt,
    primordials::{BasePrimordials, Primordial},
};
use ring::rand::SecureRandom;
use rquickjs::{
    atom::PredefinedAtom,
    context::EvalOptions,
    function::Opt,
    loader::FileResolver,
    prelude::{Func, Rest},
    AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, Ctx, Error, Function, IntoJs, Object,
    Result, Value,
};

pub static COMPRESSION_DICT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compression.dict"));

use crate::{
    environment, http,
    module_loader::{loader::CustomLoader, require, resolver::CustomResolver},
    modules::{console, crypto::SYSTEM_RANDOM, repl::run_repl},
    security,
    utils::clone::structured_clone,
};

fn print(value: String, stdout: Opt<bool>) {
    if stdout.0.unwrap_or_default() {
        println!("{value}");
    } else {
        eprintln!("{value}")
    }
}

pub struct Vm {
    pub runtime: AsyncRuntime,
    pub ctx: AsyncContext,
}

#[allow(dead_code)]
struct ExportArgs<'js>(Ctx<'js>, Object<'js>, Value<'js>, Value<'js>);

pub struct VmOptions {
    pub module_builder: crate::module_builder::ModuleBuilder,
    pub max_stack_size: usize,
    pub gc_threshold_mb: usize,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            module_builder: crate::module_builder::ModuleBuilder::default(),
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
        llrt_modules::time::init();
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
                init(&ctx)?;
                require::init(&ctx, module_names)?;
                Ok(())
            })()
            .catch(&ctx)
            .unwrap_or_else(|err| Self::print_error_and_exit(&ctx, err));
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
                    Self::print_error_and_exit(&ctx, err);
                }
            })
            .await;
    }

    pub async fn run_file(&self, filename: impl AsRef<str>, strict: bool, global: bool) {
        let source = [
            r#"import(""#,
            &filename.as_ref().replace('\\', "/"),
            r#"").catch((e) => {{console.error(e);process.exit(1)}})"#,
        ]
        .concat();

        self.run(source, strict, global).await;
    }

    pub async fn run_repl(&self) {
        run_repl(&self.ctx).await;
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

    pub fn print_error_and_exit<'js>(ctx: &Ctx<'js>, err: CaughtError<'js>) -> ! {
        let mut error_str = String::new();
        write!(error_str, "Error: {:?}", err).unwrap();
        if let Ok(error) = err.into_value(ctx) {
            if console::log_std_err(ctx, Rest(vec![error.clone()]), console::LogLevel::Fatal)
                .is_err()
            {
                eprintln!("{}", error_str);
            };
            if cfg!(test) {
                panic!("{:?}", error);
            } else {
                exit(1)
            }
        } else if cfg!(test) {
            panic!("{}", error_str);
        } else {
            eprintln!("{}", error_str);
            exit(1)
        };
    }

    pub async fn idle(self) -> StdResult<(), Box<dyn std::error::Error + Sync + Send>> {
        self.runtime.idle().await;
        Ok(())
    }
}

fn json_parse_string<'js>(ctx: Ctx<'js>, bytes: ObjectBytes<'js>) -> Result<Value<'js>> {
    let bytes = bytes.as_bytes();
    json_parse(&ctx, bytes)
}

fn init(ctx: &Ctx<'_>) -> Result<()> {
    llrt_context::set_spawn_error_handler(|ctx, err| {
        Vm::print_error_and_exit(ctx, err);
    });

    let globals = ctx.globals();

    globals.set("__gc", Func::from(|ctx: Ctx| ctx.run_gc()))?;

    let number: Function = globals.get(PredefinedAtom::Number)?;
    let number_proto: Object = number.get(PredefinedAtom::Prototype)?;
    number_proto.set(PredefinedAtom::ToString, Func::from(number_to_string))?;

    let readable_stream_stub = ctx.eval::<Value,_>(
            r#"class ReadableStream{constructor(){throw Error(`ReadableStream is not supported via global scope. Enable this by adding this to your code:\nimport { ReadableStream } from "stream";\nglobalThis.ReadableStream = ReadableStream;`)}};"#
    )?;

    globals.set("ReadableStream", readable_stream_stub)?;
    globals.set("global", ctx.globals())?;
    globals.set("self", ctx.globals())?;
    globals.set("load", Func::from(load))?;
    globals.set("print", Func::from(print))?;
    globals.set(
        "structuredClone",
        Func::from(|ctx, value, options| structured_clone(&ctx, value, options)),
    )?;

    let json_module: Object = globals.get(PredefinedAtom::JSON)?;
    json_module.set("parse", Func::from(json_parse_string))?;
    json_module.set(
        "stringify",
        Func::from(|ctx, value, replacer, space| {
            struct StringifyArgs<'js>(Ctx<'js>, Value<'js>, Opt<Value<'js>>, Opt<Value<'js>>);
            let StringifyArgs(ctx, value, replacer, space) =
                StringifyArgs(ctx, value, replacer, space);

            let mut space_value = None;
            let mut replacer_value = None;

            if let Some(replacer) = replacer.0 {
                if let Some(space) = space.0 {
                    if let Some(space) = space.as_string() {
                        let mut space = space.clone().to_string()?;
                        space.truncate(20);
                        space_value = Some(space);
                    }
                    if let Some(number) = space.as_int() {
                        if number > 0 {
                            space_value = Some(" ".repeat(min(10, number as usize)));
                        }
                    }
                }
                replacer_value = Some(replacer);
            }

            json_stringify_replacer_space(&ctx, value, replacer_value, space_value)
                .map(|v| v.into_js(&ctx))?
        }),
    )?;

    //init base primordials
    let _ = BasePrimordials::get(ctx)?;

    Ok(())
}

fn load<'js>(ctx: Ctx<'js>, filename: String, options: Opt<Object<'js>>) -> Result<Value<'js>> {
    let mut eval_options = EvalOptions::default();
    eval_options.strict = false;
    eval_options.promise = true;

    if let Some(options) = options.0 {
        if let Some(global) = options.get_optional("global")? {
            eval_options.global = global;
        }

        if let Some(strict) = options.get_optional("strict")? {
            eval_options.strict = strict;
        }
    }

    ctx.eval_file_with_options(filename, eval_options)
}
