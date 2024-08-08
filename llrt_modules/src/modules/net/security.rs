// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::OnceLock;

use rquickjs::{Ctx, Exception, Result};

static NET_ALLOW_LIST: OnceLock<Vec<String>> = OnceLock::new();

static NET_DENY_LIST: OnceLock<Vec<String>> = OnceLock::new();

pub fn set_allow_list(values: Vec<String>) {
    _ = NET_ALLOW_LIST.set(values);
}

pub fn get_allow_list() -> Option<&'static Vec<String>> {
    NET_ALLOW_LIST.get()
}

pub fn set_deny_list(values: Vec<String>) {
    _ = NET_DENY_LIST.set(values);
}

pub fn get_deny_list() -> Option<&'static Vec<String>> {
    NET_DENY_LIST.get()
}

pub fn ensure_access(ctx: &Ctx<'_>, resource: &String) -> Result<()> {
    if let Some(allow_list) = NET_ALLOW_LIST.get() {
        if !allow_list.contains(resource) {
            return Err(Exception::throw_message(
                ctx,
                &["Network address not allowed: ", resource].concat(),
            ));
        }
    }

    if let Some(deny_list) = NET_DENY_LIST.get() {
        if deny_list.contains(resource) {
            return Err(Exception::throw_message(
                ctx,
                &["Network address denied: ", resource].concat(),
            ));
        }
    }
    Ok(())
}
