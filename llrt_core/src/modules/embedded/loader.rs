// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{io, result::Result as StdResult, time::Duration};

use once_cell::sync::Lazy;
use rquickjs::{loader::Loader, Ctx, Error, Module, Object, Result};
use tracing::trace;
use zstd::{bulk::Decompressor, dict::DecoderDictionary};

use crate::bytecode::{
    BYTECODE_COMPRESSED, BYTECODE_FILE_EXT, BYTECODE_UNCOMPRESSED, BYTECODE_VERSION,
    SIGNATURE_LENGTH,
};

use super::{BYTECODE_CACHE, CJS_IMPORT_PREFIX, CJS_LOADER_PREFIX, COMPRESSION_DICT};

static DECOMPRESSOR_DICT: Lazy<DecoderDictionary> =
    Lazy::new(|| DecoderDictionary::copy(COMPRESSION_DICT));

#[cfg(feature = "lambda")]
include!(concat!(env!("OUT_DIR"), "/sdk_client_endpoints.rs"));

#[derive(Debug, Default)]
pub struct EmbeddedLoader;

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

        let (_, _, normalized_name, path) = Self::normalize_name(name);

        #[cfg(feature = "lambda")]
        #[cfg(test)]
        init_client_connection(&ctx, path)?;

        if let Some(bytes) = BYTECODE_CACHE.get(path) {
            #[cfg(not(test))]
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

#[cfg(test)]
#[cfg(feature = "lambda")]
fn init_client_connection(ctx: &Ctx<'_>, specifier: &str) -> Result<()> {
    use crate::runtime_client::{check_client_inited, mark_client_inited};
    use rquickjs::qjs;

    if specifier.ends_with("sdk-runtime-init.mjs") {
        let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
        let rt_ptr = rt as usize; //hack to move, is safe since runtime is still alive in spawn
        if !check_client_inited(rt, "endpoint") {
            tokio::task::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                mark_client_inited(rt_ptr as _);
            });
        }
    };
    Ok(())
}

#[cfg(not(test))]
#[cfg(feature = "lambda")]
fn init_client_connection(ctx: &Ctx<'_>, specifier: &str) -> Result<()> {
    use std::{env, time::Instant};

    use http_body_util::BodyExt;
    use hyper::Uri;
    use rquickjs::qjs;

    use crate::environment::ENV_LLRT_SDK_CONNECTION_WARMUP;
    use crate::libs::utils::result::ResultExt;
    use crate::modules::fetch::HTTP_CLIENT;
    use crate::runtime_client::{check_client_inited, mark_client_inited};

    let disable_warmup = env::var(ENV_LLRT_SDK_CONNECTION_WARMUP).unwrap_or_default();
    if disable_warmup == "0" || disable_warmup == "false" {
        return Ok(());
    }

    let Some(sdk_import) = specifier.strip_prefix("@aws-sdk/") else {
        return Ok(());
    };

    let client_name = sdk_import.trim_start_matches("client-");

    let Some(endpoint) = SDK_CLIENT_ENDPOINTS.get(client_name) else {
        return Ok(());
    };

    let endpoint = if endpoint.is_empty() {
        client_name
    } else {
        endpoint
    };

    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let rt_ptr = rt as usize; //hack to move, is safe since runtime is still alive in spawn

    if check_client_inited(rt, endpoint) {
        return Ok(());
    }

    let client = HTTP_CLIENT.as_ref().or_throw(ctx)?;

    trace!("Started client init {}", client_name);
    let mut region = env::var("AWS_REGION").unwrap();
    let mut region_separator = ".";

    //do not use regional endpoint for global services https://docs.aws.amazon.com/general/latest/gr/rande.html#global-endpoints
    if matches!(
        client_name,
        "iam"
            | "route-53"
            | "cloudfront"
            | "waf"
            | "shield"
            | "global-accelerator"
            | "organizations"
            | "networkmanager"
    ) {
        region_separator = "";
        region.clear();
    };

    let url_string = [
        "https://",
        endpoint,
        region_separator,
        &region,
        ".amazonaws.com/sping",
    ]
    .concat();

    if let Ok(url) = url_string.parse::<Uri>() {
        tokio::task::spawn(async move {
            let start = Instant::now();
            let get_future = client.get(url);

            let result = tokio::time::timeout(Duration::from_secs(1), get_future).await;

            let res = if let Ok(Ok(mut res)) = result {
                if let Ok(_) = res.body_mut().collect().await {
                    trace!("Client connection initialized in {:?}", start.elapsed())
                } else {
                    trace!("Failed to connect for client init {}", &url_string)
                }
            } else {
                trace!("Failed to connect for client init {}", &url_string)
            };
            mark_client_inited(rt_ptr as _);
        });
    } else {
        trace!("Failed to parse url for init");
        mark_client_inited(rt_ptr as _);
    }
    Ok(())
}
