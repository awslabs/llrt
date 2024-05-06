// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

mod minimal_tracer;

use llrt_core::{
    async_with,
    modules::{
        self,
        console::{self, LogLevel},
        process::{get_arch, get_platform},
    },
    runtime_client,
    utils::io::{get_basename_ext_name, get_js_path, DirectoryWalker, JS_EXTENSIONS},
    vm::Vm,
    AsyncContext, CatchResultExt, Module, Value, VERSION,
};
use minimal_tracer::MinimalTracer;
use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::exit,
    sync::atomic::Ordering,
    time::Instant,
};
use tokio::fs;

use tracing::trace;

#[cfg(not(feature = "lambda"))]
use llrt_core::compiler::compile_file;

#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let now = Instant::now();

    MinimalTracer::register()?;
    trace!("Started runtime");

    let vm = Vm::new().await?;
    trace!("Initialized VM in {}ms", now.elapsed().as_millis());

    if env::var("AWS_LAMBDA_RUNTIME_API").is_ok() && env::var("_HANDLER").is_ok() {
        let aws_lambda_json_log_format =
            env::var("AWS_LAMBDA_LOG_FORMAT") == Ok("JSON".to_string());
        let aws_lambda_log_level = env::var("AWS_LAMBDA_LOG_LEVEL").unwrap_or_default();
        let log_level = LogLevel::from_str(&aws_lambda_log_level);

        console::AWS_LAMBDA_JSON_LOG_LEVEL.store(log_level as usize, Ordering::Relaxed);
        console::AWS_LAMBDA_MODE.store(true, Ordering::Relaxed);
        console::AWS_LAMBDA_JSON_LOG_FORMAT.store(aws_lambda_json_log_format, Ordering::Relaxed);

        start_runtime(&vm).await
    } else {
        start_cli(&vm).await;
    }

    vm.idle().await?;

    Ok(())
}

fn print_version() {
    println!("LLRT ({} {}) {}", get_platform(), get_arch(), VERSION);
}

fn usage() {
    print_version();
    println!(
        r#"

Usage:
  llrt <filename>
  llrt -v | --version
  llrt -h | --help
  llrt -e | --eval <source>
  llrt compile input.js [output.lrt]
  llrt test <test_args>

Options:
  -v, --version     Print version information
  -h, --help        Print this help message
  -e, --eval        Evaluate the provided source code
  compile           Compile JS to bytecode and compress it with zstd:
                      if [output.lrt] is omitted, <input>.lrt is used.
                      lrt file is expected to be executed by the llrt version
                      that created it
  test              Run tests with provided arguments:
                      <test_args> -d <directory> <test-filter>
"#
    );
}

async fn start_runtime(vm: &Vm) {
    async_with!(vm.ctx => |ctx|{
        if let Err(err) = runtime_client::start(&ctx).await.catch(&ctx) {
            Vm::print_error_and_exit(&ctx, err)
        }
    })
    .await;
}

async fn start_cli(vm: &Vm) {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        for (i, arg) in args.iter().enumerate() {
            let arg = arg.as_str();
            if i == 1 {
                match arg {
                    "-v" | "--version" => {
                        print_version();
                        return;
                    },
                    "-h" | "--help" => {
                        usage();
                        return;
                    },
                    "-e" | "--eval" => {
                        if let Some(source) = args.get(i + 1) {
                            vm.run(source.as_bytes(), false).await;
                        }
                        return;
                    },
                    "test" => {
                        if let Err(error) = run_tests(vm, &args[i + 1..]).await {
                            eprintln!("{error}");
                            exit(1);
                        }
                        return;
                    },
                    "compile" => {
                        #[cfg(not(feature = "lambda"))]
                        {
                            if let Some(filename) = args.get(i + 1) {
                                let output_filename = if let Some(arg) = args.get(i + 2) {
                                    arg.to_string()
                                } else {
                                    let mut buf = PathBuf::from(filename);
                                    buf.set_extension("lrt");
                                    buf.to_string_lossy().to_string()
                                };

                                let filename = Path::new(filename);
                                let output_filename = Path::new(&output_filename);
                                if let Err(error) = compile_file(filename, output_filename).await {
                                    eprintln!("{error}");
                                    exit(1);
                                }
                                return;
                            } else {
                                eprintln!("compile: input filename is required.");
                                exit(1);
                            }
                        }
                        #[cfg(feature = "lambda")]
                        {
                            eprintln!("Not supported in \"lambda\" version.");
                            exit(1);
                        }
                    },
                    _ => {},
                }

                let (_, ext) = get_basename_ext_name(arg);

                let filename = Path::new(arg);
                let file_exists = filename.exists();
                if let ".js" | ".mjs" | ".cjs" = ext.as_str() {
                    if file_exists {
                        return run_file(vm, filename).await;
                    } else {
                        eprintln!("No such file: {}", arg);
                        exit(1);
                    }
                }
                if file_exists {
                    return run_file(vm, filename).await;
                }
                eprintln!("Unknown command: {}", arg);
                usage();
                exit(1);
            }
        }
    } else if let Some(filename) = get_js_path("index") {
        run_file(vm, &filename).await;
    }
}

async fn run_file(vm: &Vm, filename: &Path) {
    match fs::read(filename).await {
        Ok(source) => {
            vm.run(source, false).await;
        },
        Err(err) => {
            eprintln!(
                "Unable to read: {}, {}",
                filename.to_str().unwrap_or_default(),
                err
            )
        },
    }
}

async fn run_tests(vm: &Vm, args: &[std::string::String]) -> Result<(), String> {
    let mut filters: Vec<&str> = Vec::with_capacity(args.len());

    let mut root = ".";

    let mut skip_next = false;

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "-d" {
            if let Some(dir) = args.get(i + 1) {
                if !Path::new(dir).exists() {
                    return Err(format!("\"{}\" does not exist", dir.as_str()));
                }
                root = dir;
                skip_next = true;
            }
        } else {
            filters.push(arg)
        }
    }

    let now = Instant::now();

    let mut entries: Vec<String> = Vec::with_capacity(100);
    let has_filters = !filters.is_empty();

    if has_filters {
        trace!("Applying filters: {:?}", filters);
    }

    trace!("Scanning directory \"{}\"", root);

    let mut directory_walker = DirectoryWalker::new(PathBuf::from(root), |name| {
        name != "node_modules" || !name.starts_with('.')
    });
    directory_walker.set_recursive(true);

    while let Some((entry, _)) = directory_walker.walk().await.map_err(|e| e.to_string())? {
        if let Some(name) = entry
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
        {
            for ext in JS_EXTENSIONS {
                let ext_name = format!(".test{}", ext);
                let ext_name = ext_name.as_str();
                if name.ends_with(ext_name)
                    && (!has_filters || filters.iter().any(|&f| name.contains(f)))
                {
                    entries.push(entry.to_string_lossy().to_string());
                }
            }
        };
    }

    trace!("Found tests in {}ms", now.elapsed().as_millis());

    vm.run_with(|ctx| {
        ctx.globals().set("__testEntries", entries)?;
        //() = Module::import(&ctx, "@llrt/test")?;

        Ok(())
    })
    .await;

    vm.run(
        r#"
        import "@llrt/test"
    "#,
        false,
    )
    .await;

    Ok(())
}
