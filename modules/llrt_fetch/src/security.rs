// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use hyper::Uri;
use rquickjs::{Ctx, Error, Exception, Result};
use std::sync::OnceLock;

static HTTP_ALLOW_LIST: OnceLock<Vec<Uri>> = OnceLock::new();

static HTTP_DENY_LIST: OnceLock<Vec<Uri>> = OnceLock::new();

pub fn set_allow_list(values: Vec<Uri>) {
    _ = HTTP_ALLOW_LIST.set(values);
}

pub fn get_allow_list() -> Option<&'static Vec<Uri>> {
    HTTP_ALLOW_LIST.get()
}

pub fn set_deny_list(values: Vec<Uri>) {
    _ = HTTP_DENY_LIST.set(values);
}

pub fn get_deny_list() -> Option<&'static Vec<Uri>> {
    HTTP_DENY_LIST.get()
}

pub fn ensure_url_access(ctx: &Ctx<'_>, uri: &Uri) -> Result<()> {
    if let Some(allow_list) = HTTP_ALLOW_LIST.get() {
        if !url_match(allow_list, uri) {
            return Err(url_restricted_error(ctx, "URL not allowed", uri));
        }
    }

    if let Some(deny_list) = HTTP_DENY_LIST.get() {
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
    let Some(host) = uri.host().map(policy_host) else {
        return false;
    };
    let port = effective_port(uri);
    list.iter().any(|entry| {
        if effective_port(entry) != port {
            return false;
        }
        let Some(entry_host) = entry.host().map(policy_host) else {
            return false;
        };
        match (&host, &entry_host) {
            (url::Host::Domain(host), url::Host::Domain(entry)) => {
                // A final dot denotes the DNS root, not a different host.
                let host = host.strip_suffix('.').unwrap_or(host);
                let entry = entry.strip_suffix('.').unwrap_or(entry);
                host == entry
                    || host
                        .strip_suffix(entry)
                        .is_some_and(|prefix| prefix.ends_with('.'))
            },
            // IP literals must match their canonical address, never a suffix.
            _ => host == entry_host,
        }
    })
}

fn policy_host(host: &str) -> url::Host {
    // Hyper accepts DNS names such as service.123 that WHATWG host parsing
    // rejects as malformed IPv4. Keep their explicit policy rules effective.
    url::Host::parse(host).unwrap_or_else(|_| url::Host::Domain(host.to_ascii_lowercase()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_canonical_hosts_without_crossing_domain_boundaries() {
        for (entry, target, expected) in [
            ("EXAMPLE.COM", "example.com", true),
            ("example.com", "SUB.EXAMPLE.COM", true),
            ("example.com", "otherexample.com", false),
            ("example.com", "example.com.attacker.test", false),
            ("example.com.", "sub.example.com", true),
            ("example.com", "sub.example.com.", true),
            ("127.1", "127.0.0.1", true),
            ("2130706433", "127.0.0.1", true),
            ("0.0.1", "127.0.0.1", false),
            ("0x7f000001", "127.0.0.1", true),
            ("127.0.0.1", "127.1", true),
            ("127.0.0.1", "127.0.0.2", false),
            ("127.0.0.1", "evil.127.0.0.1", false),
            ("[0:0:0:0:0:0:0:1]", "[::1]", true),
            ("[::1]", "[::2]", false),
            ("XN--BCHER-KVA.EXAMPLE", "xn--bcher-kva.example", true),
        ] {
            let entry: Uri = format!("http://{entry}:8080").parse().unwrap();
            let target: Uri = format!("http://{target}:8080/path").parse().unwrap();
            assert_eq!(
                url_match(std::slice::from_ref(&entry), &target),
                expected,
                "{entry} vs {target}"
            );
        }
    }

    #[test]
    fn preserves_matching_for_hosts_outside_url_parser_grammar() {
        for (entry, target, expected) in [
            ("service.123", "service.123", true),
            ("SERVICE.123", "service.123", true),
            ("service.123", "child.service.123", true),
            ("service.123.", "child.service.123", true),
            ("service.123", "otherservice.123", false),
            ("service.123", "service.123.attacker.test", false),
            ("service.123", "service.124", false),
            ("127.0.0.1", "child.127.0.0.1", false),
        ] {
            let entry: Uri = format!("http://{entry}:49151").parse().unwrap();
            let target: Uri = format!("http://{target}:49151/path").parse().unwrap();
            assert_eq!(
                url_match(std::slice::from_ref(&entry), &target),
                expected,
                "{entry} vs {target}"
            );
        }
    }

    #[test]
    fn matches_effective_http_ports() {
        for (entry, target, expected) in [
            ("https://example.com:443", "https://example.com/path", true),
            ("https://example.com", "https://example.com:443/path", true),
            ("https://example.com", "https://example.com/path", true),
            ("http://example.com:80", "http://example.com/path", true),
            ("http://example.com", "http://example.com:80/path", true),
            ("http://example.com:80", "https://example.com/path", false),
            ("https://example.com:443", "http://example.com/path", false),
            (
                "https://example.com",
                "https://example.com:8443/path",
                false,
            ),
            (
                "https://example.com:8443",
                "https://example.com:8443/path",
                true,
            ),
            ("http://example.com:443", "https://example.com/path", true),
            ("https://example.com:80", "http://example.com/path", true),
            (
                "https://example.com",
                "https://sub.example.com:443/path",
                true,
            ),
            (
                "https://example.com",
                "https://otherexample.com:443/path",
                false,
            ),
        ] {
            let list = [entry.parse().unwrap()];
            assert_eq!(
                url_match(&list, &target.parse().unwrap()),
                expected,
                "{entry} vs {target}"
            );
        }
    }
}
