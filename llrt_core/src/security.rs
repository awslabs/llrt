// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use hyper::{http::uri::InvalidUri, Uri};
use once_cell::sync::Lazy;
use rquickjs::{Ctx, Error, Exception, Result};
use std::{
    env::{self, VarError},
    result::Result as StdResult,
};

use crate::environment::{ENV_LLRT_NET_ALLOW, ENV_LLRT_NET_DENY};

pub fn init() -> StdResult<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(env_value) = env::var(ENV_LLRT_NET_ALLOW) {
        if let Some(allow_list) = build_access_list(env_value) {
            llrt_modules::http::set_allow_list(build_http_access_list(&allow_list)?);
            llrt_modules::net::set_allow_list(allow_list);
        }
    }

    if let Ok(env_value) = env::var(ENV_LLRT_NET_DENY) {
        if let Some(deny_list) = build_access_list(env_value) {
            llrt_modules::http::set_deny_list(build_http_access_list(&deny_list)?);
            llrt_modules::net::set_deny_list(deny_list);
        }
    }

    Ok(())
}

fn build_http_access_list(list: &[String]) -> StdResult<Vec<Uri>, InvalidUri> {
    list.iter()
        .flat_map(|entry| {
            let with_http = ["http://", entry].concat();
            let with_https = ["https://", entry].concat();
            vec![with_http, with_https]
        })
        .map(|url| url.parse())
        .collect()
}

fn build_access_list(env_value: String) -> Vec<String> {
    env_value
        .split_whitespace()
        .map(|entry| {
            //remove protocol
            if let Some(idx) = entry.find("://") {
                entry[idx + 3..].to_string()
            } else {
                entry.to_string()
            }
        })
        .collect()
}
