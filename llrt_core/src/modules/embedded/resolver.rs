// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::borrow::Cow;

use rquickjs::{loader::Resolver, Ctx, Error, Result};
use tracing::trace;

use crate::modules::path;

use super::{BYTECODE_CACHE, CJS_IMPORT_PREFIX};

#[derive(Debug, Default)]
pub struct EmbeddedResolver;

#[allow(clippy::manual_strip)]
impl Resolver for EmbeddedResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        let name = name.trim_start_matches(CJS_IMPORT_PREFIX);
        let base = base.trim_start_matches(CJS_IMPORT_PREFIX);

        trace!("Try resolve '{}' from '{}'", name, base);

        embedded_resolve(name, base).map(|name| name.into_owned())
    }
}

pub fn embedded_resolve<'a>(x: &'a str, y: &str) -> Result<Cow<'a, str>> {
    trace!("embedded_resolve(x, y):({}, {})", x, y);

    // If X is a bytecode cache,
    if BYTECODE_CACHE.contains_key(x) {
        trace!("+- Resolved by `BYTECODE_CACHE`: {}", x);
        return Ok(x.into());
    }

    let x_normalized = path::normalize(x);
    if BYTECODE_CACHE.contains_key(&x_normalized) {
        trace!("+- Resolved by `BYTECODE_CACHE`: {}", x_normalized);
        return Ok(x_normalized.into());
    }

    Err(Error::new_resolving(y.to_string(), x.to_string()))
}
