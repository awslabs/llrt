// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use rquickjs::{CatchResultExt, Context, Module, Runtime, WriteOptions};
use tracing::trace;
use zstd::bulk::Compressor;

use crate::bytecode::{add_bytecode_header, BYTECODE_SELF_CONTAINED_EXECUTABLE_MARKER};
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
            let mut compressed = compress_module(&bytes)?;

            total_bytes += bytes.len();
            compressed_bytes += compressed.len();

            if create_executable {
                // Create the executable by prepending the LLRT runtime to the bytecode
                let mut exe_content = Vec::new();

                let executable_path = env::current_exe()
                    .unwrap_or_else(|_| PathBuf::from(env::args().next().unwrap_or_default()));
                let mut content = fs::read(executable_path)?;
                exe_content.append(&mut content);

                exe_content.append(&mut compressed);

                let size = u64::try_from(compressed_bytes).unwrap();
                let size_bytes = size.to_le_bytes();
                exe_content.extend_from_slice(&size_bytes);

                exe_content.extend_from_slice(BYTECODE_SELF_CONTAINED_EXECUTABLE_MARKER);

                fs::write(output_filename, &exe_content)?;

                // Set executable permissions on Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(output_filename)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(output_filename, perms)?;
                }
            } else {
                fs::write(output_filename, &compressed)?;
            }

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
