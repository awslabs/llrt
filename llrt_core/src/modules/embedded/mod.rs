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

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    let embedded_hook = Function::new(ctx.clone(), move |x: String, y: String| {
        embedded_resolve(&x, &y).map(|res| res.into_owned())
    })?;

    globals.set("__embedded_hook", embedded_hook)?;

    Ok(())
}
