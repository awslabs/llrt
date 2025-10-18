// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{borrow::Cow, env, path::Path};

use once_cell::sync::Lazy;
use rquickjs::{loader::Resolver, Ctx, Error, Result};
use tracing::trace;

use crate::modules::{path, require::CJS_IMPORT_PREFIX};
use crate::utils::io::JS_EXTENSIONS;

static LLRT_PSEUDO_MODULE_DIR: Lazy<Option<String>> =
    Lazy::new(|| env::var(crate::environment::ENV_LLRT_PSEUDO_MODULE_DIR).ok());

#[derive(Debug, Default)]
pub struct PseudoResolver;

#[allow(clippy::manual_strip)]
impl Resolver for PseudoResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        let name = name.trim_start_matches(CJS_IMPORT_PREFIX);
        let name = name.trim_start_matches("node:").trim_end_matches("/");

        let base = base.trim_start_matches(CJS_IMPORT_PREFIX);

        trace!("Try resolve '{}' from '{}'", name, base);

        pseudo_resolve(name, base).map(|name| name.into_owned())
    }
}

pub fn pseudo_resolve<'a>(x: &'a str, y: &str) -> Result<Cow<'a, str>> {
    trace!("pseudo_resolve(x, y):({}, {})", x, y);

    let x_normalized = path::normalize(x);

    if let Some(pseudo_module_dir) = LLRT_PSEUDO_MODULE_DIR.as_ref() {
        let mut base_path = String::with_capacity(pseudo_module_dir.len() + x_normalized.len() + 4); //add capacity for extention
        base_path.push_str(pseudo_module_dir);
        base_path.push_str(&x_normalized);
        let base_path_length = base_path.len();

        let mut path = Some(base_path);

        for extension in JS_EXTENSIONS.iter() {
            if let Some(mut current_path) = path.take() {
                current_path.truncate(base_path_length);
                current_path.push_str(extension);
                if Path::new(&current_path).is_file() {
                    trace!("+- Resolved by `pseudo_module`: {}", current_path);
                    return Ok(current_path.into());
                }
                path = Some(current_path);
            }
        }
    }

    Err(Error::new_resolving(y.to_string(), x.to_string()))
}
