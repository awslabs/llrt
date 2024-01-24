use std::{fs, io, path::Path};

use rquickjs::{Context, Module, Runtime};
use tracing::trace;
use zstd::bulk::Compressor;

use crate::{
    compiler_common::{human_file_size, DummyLoader, DummyResolver},
    vm::COMPRESSION_DICT,
};

fn compress_module(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut compressor = Compressor::with_dictionary(22, COMPRESSION_DICT)?;
    let raw: Vec<u8> = compressor.compress(bytes)?;
    let uncompressed_len = bytes.len() as u32;

    let mut compressed = Vec::with_capacity(4);
    compressed.extend_from_slice(&uncompressed_len.to_le_bytes());
    compressed.extend_from_slice(&raw);

    Ok(compressed)
}

pub async fn compile_file(input_filename: &Path, output_filename: &Path) -> Result<(), String> {
    let resolver = (DummyResolver,);
    let loader = (DummyLoader,);

    let rt = Runtime::new().unwrap();
    rt.set_loader(resolver, loader);
    let ctx = Context::full(&rt).unwrap();

    let mut total_bytes: usize = 0;
    let mut compressed_bytes: usize = 0;
    let mut js_bytes: usize = 0;

    ctx.with(|ctx| {
        let source = fs::read_to_string(input_filename)
            .unwrap_or_else(|_| panic!("Unable to load: {}", input_filename.to_string_lossy()));
        js_bytes = source.len();

        let module_name = input_filename
            .with_extension("")
            .to_string_lossy()
            .to_string();

        trace!("Compiling module: {}", module_name);

        let module = unsafe { Module::unsafe_declare(ctx.clone(), module_name, source).unwrap() };
        let bytes = module.write_object(false).unwrap();
        let filename = output_filename.to_string_lossy().to_string();
        let compressed = compress_module(&bytes).unwrap();
        fs::write(filename, &compressed).unwrap();

        total_bytes += bytes.len();
        compressed_bytes += compressed.len();
    });

    trace!("JS size: {}", human_file_size(js_bytes));
    trace!("Bytecode size: {}", human_file_size(total_bytes));
    trace!(
        "Compressed bytecode size: {}",
        human_file_size(compressed_bytes)
    );

    Ok(())
}
