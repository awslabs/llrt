// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{io, result::Result as StdResult};

use once_cell::sync::Lazy;
use rquickjs::{loader::Loader, Ctx, Error, Module, Object, Result};
use tracing::trace;
use zstd::{bulk::Decompressor, dict::DecoderDictionary};

use crate::bytecode::{
    BYTECODE_COMPRESSED, BYTECODE_FILE_EXT, BYTECODE_UNCOMPRESSED, BYTECODE_VERSION,
    SIGNATURE_LENGTH,
};

use super::{BYTECODE_CACHE, CJS_IMPORT_PREFIX, CJS_LOADER_PREFIX, COMPRESSION_DICT};

// Legacy bytecode support
const LEGACY_BYTECODE_VERSION: &str = "LLRT0001";
const LEGACY_BYTECODE_COMPRESSED: u8 = 1;
const LEGACY_SIGNATURE_LENGTH: usize = LEGACY_BYTECODE_VERSION.len() + 1;

static DECOMPRESSOR_DICT: Lazy<DecoderDictionary> =
    Lazy::new(|| DecoderDictionary::copy(COMPRESSION_DICT));

#[cfg(feature = "lambda")]
include!(concat!(env!("OUT_DIR"), "/sdk_client_endpoints.rs"));

#[derive(Debug, Default)]
pub struct EmbeddedLoader;

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

impl EmbeddedLoader {
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

    fn get_module_bytecode(input: &[u8]) -> Result<Vec<u8>> {
        let (_, compressed, input) = Self::get_bytecode_signature(input)?;

        if compressed {
            let (size, input) = Self::uncompressed_size(input)?;
            let mut buf = Vec::with_capacity(size);
            let mut decompressor = Decompressor::with_prepared_dictionary(&DECOMPRESSOR_DICT)?;
            decompressor.decompress_to_buffer(input, &mut buf)?;
            return Ok(buf);
        }

        Ok(input.to_vec())
    }

    fn get_bytecode_signature(input: &[u8]) -> StdResult<(&[u8], bool, &[u8]), io::Error> {
        let raw_signature = input
            .get(..SIGNATURE_LENGTH)
            .ok_or(io::Error::new::<String>(
                io::ErrorKind::InvalidInput,
                "Invalid bytecode signature length".into(),
            ))?;

        let (last, signature) = raw_signature.split_last().unwrap();

        if signature != BYTECODE_VERSION.as_bytes() {
            return Err(io::Error::new::<String>(
                io::ErrorKind::InvalidInput,
                "Invalid bytecode version".into(),
            ));
        }

        let mut compressed = None;
        if *last == BYTECODE_COMPRESSED {
            compressed = Some(true)
        } else if *last == BYTECODE_UNCOMPRESSED {
            compressed = Some(false)
        }

        let rest = &input[SIGNATURE_LENGTH..];
        Ok((
            signature,
            compressed.ok_or(io::Error::new::<String>(
                io::ErrorKind::InvalidInput,
                "Invalid bytecode signature".into(),
            ))?,
            rest,
        ))
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

        let (_, _, normalized_name, path) = Self::normalize_name(name);

        if let Some(bytes) = BYTECODE_CACHE.get(path) {
            #[cfg(feature = "lambda")]
            init_client_connection(&ctx, path)?;

            trace!("Loading embedded module: {}\n", path);

            return Ok((Self::load_bytecode_module(ctx, bytes)?, Some(path.into())));
        }

        let bytes = std::fs::read(path)?;
        let bytes: &[u8] = &bytes;

        if normalized_name.ends_with(BYTECODE_FILE_EXT) {
            trace!("Loading binary module: {}\n", path);
            return Ok((Self::load_bytecode_module(ctx, bytes)?, Some(path.into())));
        }

        Err(Error::new_loading_message(path, "unable to load"))
    }
}

impl Loader for EmbeddedLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js>> {
        let (module, url) = Self::load_module(name, ctx)?;
        if let Some(url) = url {
            let meta: Object = module.meta()?;
            meta.prop("url", url)?;
        }

        Ok(module)
    }
}

#[cfg(feature = "lambda")]
fn init_client_connection(ctx: &Ctx<'_>, specifier: &str) -> Result<()> {
    use std::{env, time::Instant};

    use http_body_util::BodyExt;
    use rquickjs::qjs;

    use crate::libs::utils::result::ResultExt;
    use crate::modules::fetch::HTTP_CLIENT;
    use crate::runtime_client::{check_client_inited, mark_client_inited};

    if let Some(sdk_import) = specifier.strip_prefix("@aws-sdk/") {
        let client_name = sdk_import.trim_start_matches("client-");
        if let Some(endpoint) = SDK_CLIENT_ENDPOINTS.get(client_name) {
            let endpoint = if endpoint.is_empty() {
                client_name
            } else {
                endpoint
            };

            let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
            let rt_ptr = rt as usize; //hack to move, is safe since runtime is still alive in spawn

            if !check_client_inited(rt, endpoint) {
                let client = HTTP_CLIENT.as_ref().or_throw(ctx)?;

                trace!("Started client init {}", client_name);
                let region = env::var("AWS_REGION").unwrap();

                let url = ["https://", endpoint, ".", &region, ".amazonaws.com/sping"].concat();

                tokio::task::spawn(async move {
                    let start = Instant::now();

                    if let Ok(url) = url.parse() {
                        if let Ok(mut res) = client.get(url).await {
                            if let Ok(res) = res.body_mut().collect().await {
                                let _ = res;

                                mark_client_inited(rt_ptr as _);

                                trace!("Client connection initialized in {:?}", start.elapsed());
                            }
                        }
                    }
                });
            }
        }
    }

    Ok(())
}
