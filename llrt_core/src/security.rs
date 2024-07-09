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

pub static NET_ALLOW_LIST: Lazy<Option<Vec<String>>> =
    Lazy::new(|| build_access_list(env::var(ENV_LLRT_NET_ALLOW)));

pub static NET_DENY_LIST: Lazy<Option<Vec<String>>> =
    Lazy::new(|| build_access_list(env::var(ENV_LLRT_NET_DENY)));

// Create global Lazy variables for allowlist and denylist
pub static HTTP_ALLOW_LIST: Lazy<Option<StdResult<Vec<Uri>, InvalidUri>>> =
    Lazy::new(|| build_http_access_list(NET_ALLOW_LIST.clone()));

pub static HTTP_DENY_LIST: Lazy<Option<StdResult<Vec<Uri>, InvalidUri>>> =
    Lazy::new(|| build_http_access_list(NET_DENY_LIST.clone()));

fn build_http_access_list(list: Option<Vec<String>>) -> Option<StdResult<Vec<Uri>, InvalidUri>> {
    list.map(|list| {
        list.iter()
            .flat_map(|entry| {
                let with_http = ["http://", entry].concat();
                let with_https = ["https://", entry].concat();
                vec![with_http, with_https]
            })
            .map(|url| url.parse())
            .collect()
    })
}

fn build_access_list(env_value: StdResult<String, VarError>) -> Option<Vec<String>> {
    env_value.ok().map(|env_value| {
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
    })
}

pub fn ensure_net_access(ctx: &Ctx<'_>, resource: &String) -> Result<()> {
    if let Some(allow_list) = &*NET_ALLOW_LIST {
        if !allow_list.contains(resource) {
            return Err(Exception::throw_message(
                ctx,
                &["Network address not allowed: ", resource].concat(),
            ));
        }
    }

    if let Some(deny_list) = &*NET_DENY_LIST {
        if deny_list.contains(resource) {
            return Err(Exception::throw_message(
                ctx,
                &["Network address denied: ", resource].concat(),
            ));
        }
    }
    Ok(())
}

pub fn ensure_url_access(ctx: &Ctx<'_>, uri: &Uri) -> Result<()> {
    if let Some(allow_list) = &*HTTP_ALLOW_LIST {
        let allow_list = allow_list.as_ref().unwrap();

        if !url_match(allow_list, uri) {
            return Err(url_restricted_error(ctx, "URL not allowed", uri));
        }
    }

    if let Some(deny_list) = &*HTTP_DENY_LIST {
        let deny_list = deny_list.as_ref().unwrap();
        if url_match(deny_list, uri) {
            return Err(url_restricted_error(ctx, "URL denied", uri));
        }
    }
    Ok(())
}

fn url_restricted_error(ctx: &Ctx<'_>, message: &str, uri: &Uri) -> Error {
    let uri_host = uri.host().unwrap_or_default();
    let mut message_string = String::with_capacity(message.len() + 100);
    message_string.push_str(message);
    message_string.push_str(": ");
    message_string.push_str(uri_host);
    if let Some(port) = uri.port_u16() {
        message_string.push(':');
        message_string.push_str(itoa::Buffer::new().format(port))
    }

    Exception::throw_message(ctx, &message_string)
}

fn url_match(list: &[Uri], uri: &Uri) -> bool {
    let host = uri.host().unwrap_or_default();
    let port = uri.port_u16().unwrap_or(80);
    list.iter().any(|entry| {
        host.ends_with(entry.host().unwrap_or_default()) && entry.port_u16().unwrap_or(80) == port
    })
}
