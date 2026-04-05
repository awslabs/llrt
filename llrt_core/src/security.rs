// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(any(feature = "fetch", feature = "net"))]
use std::env;
use std::result::Result as StdResult;

#[cfg(any(feature = "fetch", feature = "net"))]
use crate::environment::{ENV_LLRT_NET_ALLOW, ENV_LLRT_NET_DENY};
#[cfg(feature = "fetch")]
use hyper::{http::uri::InvalidUri, Uri};

pub fn init() -> StdResult<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(any(feature = "fetch", feature = "net"))]
    {
        if let Ok(env_value) = env::var(ENV_LLRT_NET_ALLOW) {
            let allow_list = build_access_list(env_value);
            #[cfg(feature = "fetch")]
            crate::modules::fetch::set_allow_list(build_http_access_list(&allow_list)?);
            #[cfg(feature = "net")]
            crate::modules::net::set_allow_list(allow_list);
        }

        if let Ok(env_value) = env::var(ENV_LLRT_NET_DENY) {
            let deny_list = build_access_list(env_value);
            #[cfg(feature = "fetch")]
            crate::modules::fetch::set_deny_list(build_http_access_list(&deny_list)?);
            #[cfg(feature = "net")]
            crate::modules::net::set_deny_list(deny_list);
        }
    }

    Ok(())
}

#[cfg(feature = "fetch")]
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

#[cfg(any(feature = "fetch", feature = "net"))]
fn build_access_list(env_value: String) -> Vec<String> {
    env_value
        .split_whitespace()
        .map(|entry| {
            if let Some(idx) = entry.find("://") {
                entry[idx + 3..].to_string()
            } else {
                entry.to_string()
            }
        })
        .collect()
}
