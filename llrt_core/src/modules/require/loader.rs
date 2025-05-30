// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    fs::File,
    io::{self, Read},
    result::Result as StdResult,
};

use rquickjs::{loader::Loader, Ctx, Function, Module, Object, Result, Value};
use tracing::trace;
use zstd::bulk::Decompressor;

use super::{CJS_IMPORT_PREFIX, CJS_LOADER_PREFIX};
use crate::bytecode::{BYTECODE_COMPRESSED, BYTECODE_VERSION, SIGNATURE_LENGTH};
use crate::modules::embedded::COMPRESSION_DICT;

// These are used for backward compatibility
const LEGACY_BYTECODE_VERSION: &str = "v1";
const LEGACY_BYTECODE_COMPRESSED: u8 = 1;
const LEGACY_SIGNATURE_LENGTH: usize = 3;

#[derive(Debug, Default)]
pub struct NpmJsLoader;

#[derive(Debug, Default)]
pub struct CustomLoader;

impl CustomLoader {
    pub fn load_bytecode_module<'js>(ctx: Ctx<'js>, buf: &[u8]) -> Result<Module<'js>> {
        let bytes = Self::get_module_bytecode(buf)?;
        unsafe { Module::load(ctx, &bytes) }
    }

    #[inline]
    pub fn uncompressed_size(input: &[u8]) -> StdResult<(usize, &[u8]), io::Error> {
        let size = input.get(..4).ok_or(io::ErrorKind::InvalidInput)?;
        let size: &[u8; 4] = size.try_into().map_err(|_| io::ErrorKind::InvalidInput)?;
        let uncompressed_size = u32::from_le_bytes(*size) as usize;
        let rest = &input[4..];
        Ok((uncompressed_size, rest))
    }

    pub fn get_module_bytecode(input: &[u8]) -> Result<Vec<u8>> {
        // Check for LLRT_EXE marker at the end (self-contained executable)
        let marker = b"LLRT_EXE";
        if input.len() > marker.len() + 8 {
            let marker_pos = input.len() - marker.len();
            if &input[marker_pos..] == marker {
                trace!("Found LLRT_EXE marker at position {}", marker_pos);

                let size_bytes = &input[marker_pos - 8..marker_pos];
                let bytecode_size = u64::from_le_bytes([
                    size_bytes[0],
                    size_bytes[1],
                    size_bytes[2],
                    size_bytes[3],
                    size_bytes[4],
                    size_bytes[5],
                    size_bytes[6],
                    size_bytes[7],
                ]) as usize;

                trace!("Bytecode size from footer: {} bytes", bytecode_size);

                if bytecode_size > 0 && bytecode_size < input.len() {
                    let bytecode_start = marker_pos - 8 - bytecode_size;
                    trace!("Bytecode starts at offset {}", bytecode_start);

                    let bytecode = &input[bytecode_start..bytecode_start + bytecode_size];
                    trace!("Checking bytecode signature and format");

                    // Return the raw bytecode for further processing
                    return Self::extract_bytecode(bytecode);
                } else {
                    trace!("Invalid bytecode size: {}", bytecode_size);
                }
            }
        }

        // Regular bytecode processing
        Self::extract_bytecode(input)
    }

    fn extract_bytecode(input: &[u8]) -> Result<Vec<u8>> {
        trace!("Extracting bytecode, input size: {} bytes", input.len());

        // Try the current bytecode format
        if input.len() >= SIGNATURE_LENGTH
            && &input[..BYTECODE_VERSION.len()] == BYTECODE_VERSION.as_bytes()
        {
            trace!(
                "Recognized current bytecode format signature: {}",
                BYTECODE_VERSION
            );
            let compressed = input[BYTECODE_VERSION.len()] == BYTECODE_COMPRESSED;
            trace!("Bytecode is compressed: {}", compressed);
            let rest = &input[SIGNATURE_LENGTH..];

            if compressed {
                let (size, data) = Self::uncompressed_size(rest)?;
                trace!("Uncompressed size will be: {} bytes", size);
                let mut buf = Vec::with_capacity(size);
                let mut decompressor = Decompressor::with_dictionary(COMPRESSION_DICT)?;
                decompressor.decompress_to_buffer(data, &mut buf)?;
                trace!("Successfully decompressed bytecode");
                return Ok(buf);
            } else {
                trace!("Using uncompressed bytecode");
                return Ok(rest.to_vec());
            }
        }

        // Try the legacy bytecode format
        if input.len() >= LEGACY_SIGNATURE_LENGTH
            && &input[..LEGACY_BYTECODE_VERSION.len()] == LEGACY_BYTECODE_VERSION.as_bytes()
        {
            trace!(
                "Recognized legacy bytecode format signature: {}",
                LEGACY_BYTECODE_VERSION
            );
            let compressed = input[LEGACY_BYTECODE_VERSION.len()] == LEGACY_BYTECODE_COMPRESSED;
            trace!("Legacy bytecode is compressed: {}", compressed);
            let rest = &input[LEGACY_SIGNATURE_LENGTH..];

            if compressed {
                let (size, data) = Self::uncompressed_size(rest)?;
                trace!("Uncompressed size will be: {} bytes", size);
                let mut buf = Vec::with_capacity(size);
                let mut decompressor = Decompressor::with_dictionary(COMPRESSION_DICT)?;
                decompressor.decompress_to_buffer(data, &mut buf)?;
                trace!("Successfully decompressed legacy bytecode");
                return Ok(buf);
            } else {
                trace!("Using uncompressed legacy bytecode");
                return Ok(rest.to_vec());
            }
        }

        // If both formats fail, return an error
        trace!("Failed to recognize bytecode format");
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid bytecode signature".to_string(),
        )
        .into())
    }
}

impl NpmJsLoader {
    fn load_cjs_module<'js>(name: &str, ctx: Ctx<'js>) -> Result<Module<'js>> {
        let cjs_specifier = [CJS_IMPORT_PREFIX, name].concat();
        let require: Function = ctx.globals().get("require")?;
        let export_object: Value = require.call((&cjs_specifier,))?;
        let mut module = String::with_capacity(name.len() + 512);
        module.push_str("const value = require(\"");

        module.push_str(name);
        module.push_str("\");export default value.default||value;");
        if let Some(obj) = export_object.as_object() {
            let keys: Result<Vec<String>> = obj.keys().collect();
            let keys = keys?;

            if !keys.is_empty() {
                module.push_str("const{");

                for p in keys.iter() {
                    if p == "default" {
                        continue;
                    }
                    module.push_str(p);
                    module.push(',');
                }
                module.truncate(module.len() - 1);
                module.push_str("}=value;");
                module.push_str("export{");
                for p in keys.iter() {
                    if p == "default" {
                        continue;
                    }
                    module.push_str(p);
                    module.push(',');
                }
                module.truncate(module.len() - 1);
                module.push_str("};");
            }
        }
        Module::declare(ctx, name, module)
    }

    fn normalize_name(name: &str) -> (bool, bool, &str, &str) {
        if !name.starts_with("__") {
            // If name doesn't start with "__", return defaults
            return (false, false, name, name);
        }

        if let Some(cjs_path) = name.strip_prefix(CJS_IMPORT_PREFIX) {
            // If it starts with CJS_IMPORT_PREFIX, mark as from_cjs_import
            return (true, false, name, cjs_path);
        }

        if let Some(cjs_path) = name.strip_prefix(CJS_LOADER_PREFIX) {
            // If it starts with CJS_LOADER_PREFIX, mark as is_cjs
            return (false, true, cjs_path, cjs_path);
        }

        // Default return if no prefixes match
        (false, false, name, name)
    }

    fn load_module<'js>(name: &str, ctx: &Ctx<'js>) -> Result<(Module<'js>, Option<String>)> {
        let ctx = ctx.clone();

        let (from_cjs_import, is_cjs, normalized_name, path) = Self::normalize_name(name);

        trace!("Loading npm module: {}\n", normalized_name);

        //json files can never be from CJS imports as they are handled by require
        if !from_cjs_import {
            if normalized_name.ends_with(".json") {
                let mut file = File::open(path)?;
                let prefix = "export default JSON.parse(`";
                let suffix = "`);";
                let mut json = String::with_capacity(prefix.len() + suffix.len());
                json.push_str(prefix);
                file.read_to_string(&mut json)?;
                json.push_str(suffix);

                return Ok((Module::declare(ctx, path, json)?, None));
            }
            if is_cjs || normalized_name.ends_with(".cjs") {
                let url = ["file://", path].concat();
                return Ok((Self::load_cjs_module(path, ctx)?, Some(url)));
            }
        }

        let bytes = std::fs::read(path)?;
        let mut bytes: &[u8] = &bytes;

        if !from_cjs_import && bytes.starts_with(b"#!") {
            bytes = bytes.splitn(2, |&c| c == b'\n').nth(1).unwrap_or(bytes);
        }

        let url = ["file://", path].concat();
        Ok((Module::declare(ctx, normalized_name, bytes)?, Some(url)))
    }
}

impl Loader for NpmJsLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js>> {
        let (module, url) = Self::load_module(name, ctx)?;
        if let Some(url) = url {
            let meta: Object = module.meta()?;
            meta.prop("url", url)?;
        }

        Ok(module)
    }
}
