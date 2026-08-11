// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use rquickjs::{Ctx, Function, Object, Result};

use crate::modules::{CJS_IMPORT_PREFIX, CJS_LOADER_PREFIX};

use self::resolver::embedded_resolve;

pub mod loader;
pub mod resolver;

pub static COMPRESSION_DICT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compression.dict"));

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));

fn set_aws_sdk_version(ctx: &Ctx, version: Option<&str>) -> Result<()> {
    if let Some(version) = version {
        let process: Object = ctx.globals().get("process")?;
        let versions: Object = process.get("versions")?;
        versions.set("@aws-sdk", version)?;
    }

    Ok(())
}

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    let embedded_hook = Function::new(ctx.clone(), move |x: String, y: String| {
        embedded_resolve(&x, &y).map(|res| res.into_owned())
    })?;

    globals.set("__require_hook", embedded_hook)?;
    set_aws_sdk_version(ctx, option_env!("LLRT_AWS_SDK_VERSION"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context, Runtime};

    use super::*;

    #[test]
    fn sets_aws_sdk_version() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();

        context.with(|ctx| {
            crate::modules::process::init(&ctx).unwrap();
            set_aws_sdk_version(&ctx, Some("3.1057.0")).unwrap();

            let version: String = ctx.eval("process.versions['@aws-sdk']").unwrap();
            assert_eq!(version, "3.1057.0");
        });
    }

    #[test]
    fn omits_aws_sdk_version_when_not_bundled() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();

        context.with(|ctx| {
            crate::modules::process::init(&ctx).unwrap();
            set_aws_sdk_version(&ctx, None).unwrap();

            let has_version: bool = ctx
                .eval("Object.hasOwn(process.versions, '@aws-sdk')")
                .unwrap();
            assert!(!has_version);
        });
    }
}
