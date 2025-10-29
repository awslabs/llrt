// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::{exit, ExitCode},
    string::String,
    sync::atomic::Ordering,
    time::Instant,
};

mod base;
mod minimal_tracer;
#[cfg(not(feature = "lambda"))]
mod repl;

use constcat::concat;
use llrt_core::modules::process::EXIT_CODE;
use minimal_tracer::MinimalTracer;
use once_cell::sync::Lazy;
use tracing::trace;

#[cfg(not(feature = "lambda"))]
use crate::base::compiler::compile_file;
use crate::base::{
    bytecode::BYTECODE_EXT,
    environment::ENV_LLRT_REGISTER_HOOKS,
    libs::{
        logging::print_error_and_exit,
        utils::{
            fs::DirectoryWalker,
            io::{is_supported_ext, SUPPORTED_EXTENSIONS},
            sysinfo::{ARCH, PLATFORM},
        },
    },
    modules::path::name_extname,
    runtime_client,
    vm::Vm,
    VERSION,
};

// rquickjs components
use crate::base::{async_with, CatchResultExt};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

static LLRT_REGISTER_HOOKS: Lazy<Option<String>> =
    Lazy::new(|| env::var(ENV_LLRT_REGISTER_HOOKS).ok());

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn Error + Send + Sync>> {
    let now = Instant::now();

    MinimalTracer::register()?;
    trace!("Started runtime");

    let vm = Vm::new().await?;
    trace!("Initialized VM in {}ms", now.elapsed().as_millis());

    if let Some(filename) = LLRT_REGISTER_HOOKS.as_ref() {
        vm.run_file(filename, true, true).await;
    }

    if env::var("AWS_LAMBDA_RUNTIME_API").is_ok() && env::var("_HANDLER").is_ok() {
        start_runtime(&vm).await
    } else {
        start_cli(&vm).await;
    }

    vm.idle().await?;

    Ok(ExitCode::from(EXIT_CODE.load(Ordering::Relaxed)))
}

pub const VERSION_STRING: &str = concat!("LLRT v", VERSION, " (", PLATFORM, ", ", ARCH, ")");

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
                      --executable      Create a self-contained executable that includes
                                        the LLRT runtime
  test              Run tests with provided arguments:
                      <test_args> -d <directory> <test-filter>
"#
    );
}

async fn start_runtime(vm: &Vm) {
    async_with!(vm.ctx => |ctx|{
        if let Err(err) = runtime_client::start(&ctx).await.catch(&ctx) {
            print_error_and_exit(&ctx, err)
        }
    })
    .await;
}

async fn start_cli(vm: &Vm) {
    #[cfg(not(feature = "lambda"))]
    {
        use crate::base::bytecode::BYTECODE_SELF_CONTAINED_EXECUTABLE_MARKER;
        use std::io::{Read, Seek, SeekFrom};

        let executable_path = env::current_exe()
            .unwrap_or_else(|_| PathBuf::from(env::args().next().unwrap_or_default()));
        trace!(
            "Checking if {} is a self-contained executable",
            executable_path.display()
        );

        if let Ok(mut f) = std::fs::File::open(executable_path) {
            let size_bytes_length: usize = size_of::<u64>();
            let marker_length: usize = BYTECODE_SELF_CONTAINED_EXECUTABLE_MARKER.len();
            let offset: usize = marker_length + size_bytes_length;
            let negative_offset = -i64::from_ne_bytes(offset.to_ne_bytes());
            let _ = f.seek(SeekFrom::End(negative_offset));
            let mut end = vec![0; offset];
            f.read_exact(&mut end).unwrap_or_else(|error| {
                eprintln!("Failed to read end of the executable: {error:?}");
                exit(1);
            });

            if &end[size_bytes_length..] == BYTECODE_SELF_CONTAINED_EXECUTABLE_MARKER {
                let size_bytes: [u8; size_of::<u64>()] =
                    end[..size_bytes_length].try_into().unwrap_or_else(|error| {
                        eprintln!("Failed to read length bytes: {error:?}");
                        exit(1);
                    });
                let size_number = u64::from_le_bytes(size_bytes);
                let metadata = f.metadata().unwrap_or_else(|error| {
                    eprintln!("Failed to get metadata of executable: {error:?}");
                    exit(1);
                });
                let unsigned_offset = u64::from_ne_bytes(offset.to_ne_bytes());
                let start = metadata.len() - size_number - unsigned_offset;
                let _ = f.seek(SeekFrom::Start(start));
                let size = usize::from_ne_bytes(size_number.to_ne_bytes());
                let mut module = vec![0; size];
                f.read_exact(&mut module).unwrap_or_else(|error| {
                    eprintln!("Failed to read embedded module: {error:?}");
                    exit(1);
                });
                return vm.run_bytecode(&module).await;
            }
        }
    }

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
                                // Parse args for output_filename and --executable
                                let mut output_filename = String::new();
                                let mut create_executable = false;

                                // Parse remaining arguments
                                for arg in args.iter().skip(i + 2) {
                                    if arg == "--executable" {
                                        create_executable = true;
                                    } else if output_filename.is_empty() && !arg.starts_with("--") {
                                        output_filename = arg.clone();
                                    }
                                }

                                // If no output filename was explicitly provided, generate one
                                if output_filename.is_empty() {
                                    let mut buf = PathBuf::from(filename);
                                    buf.set_extension("lrt");
                                    output_filename = buf.to_string_lossy().to_string();
                                }

                                let filename = Path::new(filename);
                                let output_filename = Path::new(&output_filename);
                                if let Err(error) =
                                    compile_file(filename, output_filename, create_executable).await
                                {
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
