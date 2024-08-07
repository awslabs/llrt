// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashSet,
    env,
    error::Error,
    fs::{self},
    io::{self},
    path::{Path, PathBuf},
    process::Command,
    result::Result as StdResult,
};

use jwalk::WalkDir;
use rquickjs::{CatchResultExt, CaughtError, Context, Module, Runtime};

const BUNDLE_JS_DIR: &str = "../bundle/js";
const BUNDLE_LRT_DIR: &str = "../bundle/lrt";

include!("src/bytecode.rs");

macro_rules! info {
    ($($tokens: tt)*) => {
        println!("cargo:info={}", format!($($tokens)*))
    }
}

macro_rules! rerun_if_changed {
    ($file: expr) => {
        println!("cargo:rerun-if-changed={}", $file)
    };
}

include!("src/compiler_common.rs");

#[tokio::main]
async fn main() -> StdResult<(), Box<dyn Error>> {
    set_nightly_cfg();

    rerun_if_changed!(BUNDLE_JS_DIR);

    cargo_patch::patch()?;

    let resolver = (DummyResolver,);
    let loader = (DummyLoader,);

    let rt = Runtime::new()?;
    rt.set_loader(resolver, loader);
    let ctx = Context::full(&rt)?;

    let mut lrt_filenames = vec![];
    let mut total_bytes: usize = 0;

    fs::write("../VERSION", env!("CARGO_PKG_VERSION")).expect("Unable to write VERSION file");

    ctx.with(|ctx| {
        for dir_ent in WalkDir::new(BUNDLE_JS_DIR).into_iter().flatten() {
            let path = dir_ent.path();

            let path = path.strip_prefix(BUNDLE_JS_DIR)?.to_owned();
            let path_str = path.to_string_lossy().to_string();

            if path_str.starts_with("__tests__") || path.extension().unwrap_or_default() != "js" {
                continue;
            }

            #[cfg(feature = "lambda")]
            {
                info!("Path is: {:?}", path);
                if path == PathBuf::new().join("@llrt").join("test.js") {
                    info!("SKIPPING TEST!!!");
                    continue;
                }
            }

            #[cfg(feature = "no-sdk")]
            {
                if path_str.starts_with("@aws-sdk")
                    || path_str.starts_with("@smithy")
                    || path_str.starts_with("llrt-chunk-sdk")
                {
                    continue;
                }
            }

            let source = fs::read_to_string(dir_ent.path())
                .unwrap_or_else(|_| panic!("Unable to load: {}", dir_ent.path().to_string_lossy()));

            let module_name = if !path_str.starts_with("llrt-chunk-") {
                path.with_extension("").to_string_lossy().to_string()
            } else {
                path.to_string_lossy().to_string()
            };

            info!("Compiling modules: {}", module_name);

            let lrt_path = PathBuf::from(BUNDLE_LRT_DIR).join(path.with_extension(BYTECODE_EXT));
            let lrt_filename = lrt_path.to_string_lossy().to_string();
            lrt_filenames.push(lrt_filename.clone());
            let bytes = {
                {
                    let module = Module::declare(ctx.clone(), module_name.clone(), source)?;
                    module.write(false)
                }
            }
            .catch(&ctx)
            .map_err(|err| match err {
                CaughtError::Error(error) => error.to_string(),
                CaughtError::Exception(ex) => ex.to_string(),
                CaughtError::Value(value) => format!("{:?}", value),
            })?;

            total_bytes += bytes.len();

            fs::create_dir_all(lrt_path.parent().unwrap())?;
            println!("Writing: {:?}", lrt_path);
            if cfg!(feature = "uncompressed") {
                let uncompressed = add_bytecode_header(bytes, None);
                fs::write(&lrt_path, uncompressed)?;
            } else {
                fs::write(&lrt_path, bytes)?;
            }

            info!("Done!");
        }

        StdResult::<_, Box<dyn Error>>::Ok(())
    })?;

    info!(
        "\n===============================\nUncompressed bytecode size: {}\n===============================",
        human_file_size(total_bytes)
    );

    let compression_dictionary_path = Path::new(BUNDLE_LRT_DIR)
        .join("compression.dict")
        .to_string_lossy()
        .to_string();

    if cfg!(feature = "uncompressed") {
        generate_compression_dictionary(&compression_dictionary_path, &lrt_filenames)?;
    } else {
        total_bytes = compress_bytecode(compression_dictionary_path, lrt_filenames)?;

        info!(
            "\n===============================\nCompressed bytecode size: {}\n===============================",
            human_file_size(total_bytes)
        );
    }

    Ok(())
}

fn set_nightly_cfg() {
    let rustc = std::env::var("RUSTC").unwrap();
    let version = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .unwrap();

    assert!(version.status.success());

    let stdout = String::from_utf8(version.stdout).unwrap();
    assert!(stdout.contains("rustc"));
    let nightly = stdout.contains("nightly") || stdout.contains("dev");
    println!("cargo::rustc-check-cfg=cfg(rust_nightly)");
    if nightly {
        println!("cargo::rustc-cfg=rust_nightly");
    }
}

fn compress_bytecode(dictionary_path: String, source_files: Vec<String>) -> io::Result<usize> {
    generate_compression_dictionary(&dictionary_path, &source_files)?;

    let mut total_size = 0;
    let tmp_dir = env::temp_dir();

    for filename in source_files {
        info!("Compressing {}...", filename);

        let tmp_filename = tmp_dir
            .join(nanoid::nanoid!())
            .to_string_lossy()
            .to_string();

        fs::copy(&filename, &tmp_filename)?;

        let uncompressed_file_size = PathBuf::from(&filename).metadata()?.len() as u32;

        let output = Command::new("zstd")
            .args([
                "--ultra",
                "-22",
                "-f",
                "-D",
                &dictionary_path,
                &tmp_filename,
                "-o",
                &filename,
            ])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to compress file",
            ));
        }

        let bytes = fs::read(&filename)?;
        let compressed = add_bytecode_header(bytes, Some(uncompressed_file_size));
        fs::write(&filename, compressed)?;

        let compressed_file_size = PathBuf::from(&filename).metadata().unwrap().len() as usize;

        total_size += compressed_file_size;
    }

    Ok(total_size)
}

fn generate_compression_dictionary(
    dictionary_path: &str,
    source_files: &Vec<String>,
) -> Result<(), io::Error> {
    info!("Generating compression dictionary...");
    let file_count = source_files.len();
    let mut dictionary_filenames = source_files.clone();
    let mut dictionary_file_set: HashSet<String> = HashSet::from_iter(dictionary_filenames.clone());
    let mut cmd = Command::new("zstd");
    cmd.args([
        "--train",
        "--train-fastcover=steps=60",
        "--maxdict=40K",
        "-o",
        dictionary_path,
    ]);
    if file_count < 5 {
        dictionary_file_set.retain(|file_path| {
            let metadata = fs::metadata(file_path).unwrap();
            let file_size = metadata.len();
            file_size >= 1024 // 1 kilobyte = 1024 bytes
        });
        cmd.arg("-B1K");
        dictionary_filenames = dictionary_file_set.into_iter().collect();
    }
    cmd.args(&dictionary_filenames);
    let mut cmd = cmd.args(source_files).spawn()?;
    let exit_status = cmd.wait()?;
    if !exit_status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to generate compression dictionary",
        ));
    };
    Ok(())
}
