// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{env, result::Result as StdResult};

use hyper::{http::uri::InvalidUri, Uri};

use crate::environment::{
    ENV_LLRT_FS_ALLOW, ENV_LLRT_FS_DENY, ENV_LLRT_NET_ALLOW, ENV_LLRT_NET_DENY,
};
use crate::modules::{fetch, fs, net};

pub fn init() -> StdResult<(), Box<dyn std::error::Error + Send + Sync>> {
    // Network isolation
    if let Ok(env_value) = env::var(ENV_LLRT_NET_ALLOW) {
        let allow_list = build_access_list(env_value);
        fetch::set_allow_list(build_http_access_list(&allow_list)?);
        net::set_allow_list(allow_list);
    }

    if let Ok(env_value) = env::var(ENV_LLRT_NET_DENY) {
        let deny_list = build_access_list(env_value);
        fetch::set_deny_list(build_http_access_list(&deny_list)?);
        net::set_deny_list(deny_list);
    }

    // Filesystem isolation
    if let Ok(env_value) = env::var(ENV_LLRT_FS_ALLOW) {
        let allow_list = build_fs_access_list(env_value);
        fs::security::set_allow_list(allow_list);
    }

    if let Ok(env_value) = env::var(ENV_LLRT_FS_DENY) {
        let deny_list = build_fs_access_list(env_value);
        fs::security::set_deny_list(deny_list);
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

fn build_fs_access_list(env_value: String) -> Vec<String> {
    env_value
        .split_whitespace()
        .map(|entry| entry.to_string())
        .collect()
}
