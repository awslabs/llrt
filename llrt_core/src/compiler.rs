// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{fs, io, path::Path};

use rquickjs::{CatchResultExt, Context, Module, Runtime, WriteOptions};
use tracing::trace;
use zstd::bulk::Compressor;

use crate::bytecode::add_bytecode_header;
use crate::compiler_common::{human_file_size, DummyLoader, DummyResolver};
use crate::libs::{logging::print_error_and_exit, utils::result::ResultExt};
use crate::modules::embedded::COMPRESSION_DICT;

fn compress_module(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut compressor = Compressor::with_dictionary(22, COMPRESSION_DICT)?;
    let compressed_bytes = compressor.compress(bytes)?;
    let uncompressed_len = bytes.len() as u32;

    let compressed = add_bytecode_header(compressed_bytes, Some(uncompressed_len));
    Ok(compressed)
}

pub async fn compile_file(
    input_filename: &Path,
    output_filename: &Path,
    create_executable: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resolver = (DummyResolver,);
    let loader = (DummyLoader,);

    let rt = Runtime::new()?;
    rt.set_loader(resolver, loader);
    let ctx = Context::full(&rt)?;

    let mut total_bytes: usize = 0;
    let mut compressed_bytes: usize = 0;
    let mut js_bytes: usize = 0;

    ctx.with(|ctx| {
        (|| {
            let source = fs::read_to_string(input_filename).or_throw_msg(
                &ctx,
                &["Unable to load: ", &input_filename.to_string_lossy()].concat(),
            )?;
            js_bytes = source.len();

            let module_name = input_filename
                .with_extension("")
                .to_string_lossy()
                .to_string();

            trace!("Compiling module: {}", module_name);

            let module = Module::declare(ctx.clone(), module_name, source)?;
            let bytes = module.write(WriteOptions::default())?;
            let compressed = compress_module(&bytes)?;

            if create_executable {
                // Create a self-contained executable
                let exe_path = if output_filename.extension().unwrap_or_default() == "lrt" {
                    output_filename.with_extension("")
                } else {
                    output_filename.to_path_buf()
                };

                // Look for the runtime binary in various locations
                let runtime_path = std::env::var("LLRT_RUNTIME_PATH")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| {
                        // Try the executable's path
                        std::env::current_exe()
                            .unwrap_or_else(|_| std::path::PathBuf::from("llrt"))
                    });

                trace!("Using runtime binary from: {}", runtime_path.display());

                if runtime_path.exists() && runtime_path.metadata()?.len() > 0 {
                    // Create the executable by prepending the LLRT runtime to the bytecode
                    let mut exe_content = Vec::new();

                    // Read the LLRT runtime binary
                    let runtime_binary = fs::read(&runtime_path)?;
                    trace!("Read runtime binary, size: {} bytes", runtime_binary.len());
                    exe_content.extend_from_slice(&runtime_binary);

                    // Add the bytecode directly without extra headers
                    trace!("Adding raw bytecode, size: {} bytes", bytes.len());
                    exe_content.extend_from_slice(&bytes);

                    // Add a footer at the end with size and magic number
                    let bytecode_size = bytes.len() as u64;
                    exe_content.extend_from_slice(&bytecode_size.to_le_bytes());
                    exe_content.extend_from_slice(b"LLRT_EXE");
                    trace!("Total executable size: {} bytes", exe_content.len());

                    // Write the executable
                    fs::write(&exe_path, exe_content)?;

                    // Set executable permissions on Unix systems
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = fs::metadata(&exe_path)?.permissions();
                        perms.set_mode(0o755);
                        fs::set_permissions(&exe_path, perms)?;
                    }

                    trace!("Created self-contained executable: {}", exe_path.display());
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("LLRT runtime binary not found or empty at path: {}. Build LLRT first before using --executable flag.", runtime_path.display()),
                    )
                    .into());
                }
            } else {
                fs::write(output_filename, &compressed)?;
            }

            total_bytes += bytes.len();
            compressed_bytes += compressed.len();
            Ok(())
        })()
        .catch(&ctx)
        .unwrap_or_else(|err| print_error_and_exit(&ctx, err))
    });

    trace!("JS size: {}", human_file_size(js_bytes));
    trace!("Bytecode size: {}", human_file_size(total_bytes));
    trace!(
        "Compressed bytecode size: {}",
        human_file_size(compressed_bytes)
    );

    Ok(())
}
