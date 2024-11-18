// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use once_cell::sync::Lazy;
use rquickjs::{loader::Loader, Ctx, Function, Module, Object, Result, Value};
use std::{
    fs::File,
    io::{self, Read},
    result::Result as StdResult,
};
use tracing::trace;
use zstd::{bulk::Decompressor, dict::DecoderDictionary};

use super::CJS_IMPORT_PREFIX;

use crate::{
    bytecode::{
        BYTECODE_COMPRESSED, BYTECODE_FILE_EXT, BYTECODE_UNCOMPRESSED, BYTECODE_VERSION,
        SIGNATURE_LENGTH,
    },
    module_loader::CJS_LOADER_PREFIX,
    vm::COMPRESSION_DICT,
};

static DECOMPRESSOR_DICT: Lazy<DecoderDictionary> =
    Lazy::new(|| DecoderDictionary::copy(COMPRESSION_DICT));

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));
#[cfg(feature = "lambda")]
include!(concat!(env!("OUT_DIR"), "/sdk_client_endpoints.rs"));

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
            // If name doesn’t start with "__", return defaults
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

        trace!("Loading module: {}", normalized_name);

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

        if let Some(bytes) = BYTECODE_CACHE.get(path) {
            #[cfg(feature = "lambda")]
            init_client_connection(&ctx, path)?;

            trace!("Loading embedded module: {}", path);

            return Ok((Self::load_bytecode_module(ctx, bytes)?, Some(path.into())));
        }

        let bytes = std::fs::read(path)?;
        let mut bytes: &[u8] = &bytes;

        if normalized_name.ends_with(BYTECODE_FILE_EXT) {
            trace!("Loading binary module: {}", path);
            return Ok((Self::load_bytecode_module(ctx, bytes)?, Some(path.into())));
        }
        if !from_cjs_import && bytes.starts_with(b"#!") {
            bytes = bytes.splitn(2, |&c| c == b'\n').nth(1).unwrap_or(bytes);
        }

        let url = ["file://", path].concat();
        Ok((Module::declare(ctx, normalized_name, bytes)?, Some(url)))
    }
}

impl Loader for CustomLoader {
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
    use crate::{
        modules::http::HTTP_CLIENT,
        runtime_client::{check_client_inited, mark_client_inited},
    };
    use http_body_util::BodyExt;
    use llrt_utils::result::ResultExt;
    use rquickjs::qjs;
    use std::{env, time::Instant};

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
