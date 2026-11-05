// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use hyper::Uri;
use llrt_net_policy::{check_network, has_allow_list, parse_host, Access};
use rquickjs::{Ctx, Error, Exception, Result};

/// Ports an entry without an explicit port covers for HTTP requests.
const DEFAULT_HTTP_PORTS: &[u16] = &[80, 443];

pub fn ensure_url_access(ctx: &Ctx<'_>, uri: &Uri) -> Result<()> {
    let Some(host) = uri.host() else {
        // Without a host there is nothing to match, so an allow list rejects.
        return if has_allow_list() {
            Err(url_restricted_error(ctx, "URL not allowed", uri))
        } else {
            Ok(())
        };
    };

    match check_network(&parse_host(host), effective_port(uri), DEFAULT_HTTP_PORTS) {
        Access::Allowed => Ok(()),
        Access::NotAllowed => Err(url_restricted_error(ctx, "URL not allowed", uri)),
        Access::Denied => Err(url_restricted_error(ctx, "URL denied", uri)),
    }
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

fn effective_port(uri: &Uri) -> u16 {
    uri.port_u16().unwrap_or_else(|| {
        if uri.scheme_str() == Some("https") {
            443
        } else {
            80
        }
    })
}
