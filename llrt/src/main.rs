// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

mod minimal_tracer;
#[cfg(not(feature = "lambda"))]
mod repl;

use constcat::concat;
use llrt_core::{
    async_with,
    bytecode::BYTECODE_EXT,
    modules::{
        console::{self, LogLevel},
        path::name_extname,
    },
    runtime_client,
    utils::io::{is_supported_ext, DirectoryWalker, SUPPORTED_EXTENSIONS},
    vm::Vm,
    CatchResultExt, VERSION,
};
use llrt_utils::sysinfo::{ARCH, PLATFORM};
use minimal_tracer::MinimalTracer;
use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::exit,
    sync::atomic::Ordering,
    time::Instant,
};

use tracing::trace;

#[cfg(not(feature = "lambda"))]
use llrt_core::compiler::compile_file;

#[cfg(not(target_os = "windows"))]
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

pub const VERSION_STRING: &'static str =
    concat!("LLRT v", VERSION, " (", PLATFORM, ", ", ARCH, ")");

fn print_version() {
    println!("{VERSION_STRING}");
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
                            vm.run(source.as_bytes(), false, false).await;
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

                let (_, ext) = name_extname(arg);

                let filename = Path::new(arg);
                let file_exists = filename.exists();

                let global = ext == ".cjs";

                if is_supported_ext(ext) {
                    if file_exists {
                        return vm.run_file(arg, true, global).await;
                    } else {
                        eprintln!("No such file: {}", arg);
                        exit(1);
                    }
                } else {
                    if file_exists {
                        return vm.run_file(arg, true, false).await;
                    }
                    eprintln!("Unknown command: {}", arg);
                    usage();
                    exit(1);
                }
            }
        }
    } else {
        #[cfg(not(feature = "lambda"))]
        {
            repl::run_repl(&vm.ctx).await;
        }

        #[cfg(feature = "lambda")]
        {
            eprintln!("REPL not supported in \"lambda\" version.");
            exit(1);
        }
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
                    return Err(["\"", dir.as_str(), "\" does not exist"].concat());
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
        name != "node_modules" && !name.starts_with('.')
    });
    directory_walker.set_recursive(true);

    let test_js_extensions: Vec<String> = SUPPORTED_EXTENSIONS
        .iter()
        .filter(|&ext| *ext != BYTECODE_EXT)
        .map(|ext| [".test", ext].concat())
        .collect();

    let pwd = env::current_dir().map_err(|e| e.to_string())?;
    let pwd = pwd.to_string_lossy();
    while let Some((entry, _)) = directory_walker.walk().await.map_err(|e| e.to_string())? {
        if let Some(name) = entry.file_name() {
            let name = name.to_string_lossy();
            let name = name.as_ref();
            for ext_name in &test_js_extensions {
                if name.ends_with(ext_name)
                    && (!has_filters || filters.iter().any(|&f| name.contains(f)))
                {
                    entries.push([pwd.as_ref(), "/", entry.to_string_lossy().as_ref()].concat());
                }
            }
        };
    }

    entries.sort_unstable();

    trace!("Found tests in {}ms", now.elapsed().as_millis());

    vm.run_with(|ctx| {
        ctx.globals().set("__testEntries", entries)?;
        Ok(())
    })
    .await;

    vm.run(
        r#"
        import "llrt:test/index"
    "#,
        false,
        false,
    )
    .await;

    Ok(())
}
