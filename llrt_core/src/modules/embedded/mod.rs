// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use rquickjs::{Ctx, Function, Result};

use self::resolver::embedded_resolve;

pub mod loader;
pub mod resolver;

// added when .cjs files are imported
const CJS_IMPORT_PREFIX: &str = "__cjs:";
// added to force CJS imports in loader
const CJS_LOADER_PREFIX: &str = "__cjsm:";

pub static COMPRESSION_DICT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compression.dict"));

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));

/// Load bytecode as a module
pub fn load_bytecode_as_module<'js>(
    ctx: &rquickjs::Ctx<'js>,
    module_name: &str,
    bytecode: &[u8],
) -> rquickjs::Result<rquickjs::Module<'js>> {
    use tracing::trace;

    trace!("Loading bytecode as module: {}", module_name);

    // Attempt to load the bytecode as a module
    let bytes = loader::CustomLoader::get_module_bytecode(bytecode).map_err(|e| {
        rquickjs::Error::new_loading(format!("Failed to decompress bytecode: {}", e))
    })?;

    // Try to load as a module
    let result = unsafe { rquickjs::Module::load(ctx.clone(), &bytes) };

    // Return the module if successful
    if result.is_ok() {
        return result;
    }

    // If loading as a module fails, return the error
    result
}

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    let embedded_hook = Function::new(ctx.clone(), move |x: String, y: String| {
        embedded_resolve(&x, &y).map(|res| res.into_owned())
    })?;

    globals.set("__embedded_hook", embedded_hook)?;

    Ok(())
}
