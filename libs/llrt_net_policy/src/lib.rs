// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
//! Matching for the `LLRT_NET_ALLOW` and `LLRT_NET_DENY` entries, shared by
//! every network sink so one entry form behaves identically everywhere.

use std::{error::Error, fmt, net::IpAddr, net::Ipv6Addr, str::FromStr, sync::OnceLock};

use url::Host;

static ALLOW_LIST: OnceLock<Vec<PolicyEntry>> = OnceLock::new();

static DENY_LIST: OnceLock<Vec<PolicyEntry>> = OnceLock::new();

/// Deny rules take precedence over allow rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Allowed,
    NotAllowed,
    Denied,
}

// Set once at start-up; later calls are ignored.
pub fn set_allow_list(entries: Vec<PolicyEntry>) {
    _ = ALLOW_LIST.set(entries);
}

// Set once at start-up; later calls are ignored.
pub fn set_deny_list(entries: Vec<PolicyEntry>) {
    _ = DENY_LIST.set(entries);
}

pub fn has_allow_list() -> bool {
    ALLOW_LIST.get().is_some()
}

/// `default_ports` are the ports an entry without an explicit port covers; an
/// empty slice means every port.
pub fn check_network(host: &Host, port: u16, default_ports: &[u16]) -> Access {
    if let Some(list) = ALLOW_LIST.get() {
        if !list
            .iter()
            .any(|entry| entry.matches_network(host, port, default_ports))
        {
            return Access::NotAllowed;
        }
    }

    if let Some(list) = DENY_LIST.get() {
        if list
            .iter()
            .any(|entry| entry.matches_network(host, port, default_ports))
        {
            return Access::Denied;
        }
    }

    Access::Allowed
}

pub fn check_path(path: &str) -> Access {
    if let Some(list) = ALLOW_LIST.get() {
        if !list.iter().any(|entry| entry.matches_path(path)) {
            return Access::NotAllowed;
        }
    }

    if let Some(list) = DENY_LIST.get() {
        if list.iter().any(|entry| entry.matches_path(path)) {
            return Access::Denied;
        }
    }

    Access::Allowed
}

pub fn check_endpoint(host: &str, port: u16) -> Result<(), String> {
    match check_network(&parse_host(host), port, &[]) {
        Access::Allowed => Ok(()),
        Access::NotAllowed => {
            Err(["Network address not allowed: ", &endpoint(host, port)].concat())
        },
        Access::Denied => Err(["Network address denied: ", &endpoint(host, port)].concat()),
    }
}

pub fn check_socket_path(path: &str) -> Result<(), String> {
    match check_path(path) {
        Access::Allowed => Ok(()),
        Access::NotAllowed => Err(["Network address not allowed: ", path].concat()),
        Access::Denied => Err(["Network address denied: ", path].concat()),
    }
}

fn endpoint(host: &str, port: u16) -> String {
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub fn parse_list<'a>(
    variable: &str,
    entries: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<PolicyEntry>, PolicyError> {
    entries
        .into_iter()
        .map(|entry| PolicyEntry::parse(entry).map_err(|err| err.with_variable(variable)))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyEntry {
    value: String,
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Host { host: Host, port: Option<u16> },
    Path,
}

impl PolicyEntry {
    /// `host`, `host:port`, `[ipv6]`, `[ipv6]:port`, a bare IPv6 address or a
    /// socket path. A scheme prefix is ignored.
    pub fn parse(entry: &str) -> Result<Self, PolicyError> {
        let value = strip_scheme(entry);
        if value.is_empty() {
            return Err(PolicyError::new(entry, "entry is empty"));
        }
        if value.starts_with('/') {
            return Ok(Self {
                value: value.to_owned(),
                kind: Kind::Path,
            });
        }

        // Ignore a trailing path so full URLs keep working as host entries.
        let value = match value.find('/') {
            Some(idx) => &value[..idx],
            None => value,
        };
        if value.is_empty() {
            return Err(PolicyError::new(entry, "entry is empty"));
        }

        let (host, port) =
            split_host_port(value).map_err(|reason| PolicyError::new(entry, reason))?;
        Ok(Self {
            value: value.to_owned(),
            kind: Kind::Host { host, port },
        })
    }

    pub fn matches_network(&self, host: &Host, port: u16, default_ports: &[u16]) -> bool {
        let Kind::Host {
            host: entry_host,
            port: entry_port,
        } = &self.kind
        else {
            return false;
        };

        let port_matches = match entry_port {
            Some(entry_port) => *entry_port == port,
            None => default_ports.is_empty() || default_ports.contains(&port),
        };

        port_matches && host_matches(host, entry_host)
    }

    /// Path entries and plain strings match literally.
    pub fn matches_path(&self, path: &str) -> bool {
        self.value == path
    }
}

/// Canonicalizes IP aliases the same way policy entries are parsed.
pub fn parse_host(host: &str) -> Host {
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);

    if let Ok(ip) = host.parse() {
        return match ip {
            IpAddr::V4(ip) => Host::Ipv4(ip),
            IpAddr::V6(ip) => Host::Ipv6(ip),
        };
    }

    // Hyper accepts DNS names such as service.123 that WHATWG host parsing
    // rejects as malformed IPv4. Keep their explicit policy rules effective.
    Host::parse(host).unwrap_or_else(|_| Host::Domain(host.to_ascii_lowercase()))
}

fn host_matches(host: &Host, entry: &Host) -> bool {
    match (host, entry) {
        (Host::Domain(host), Host::Domain(entry)) => {
            // A final dot denotes the DNS root, not a different host.
            let host = host.strip_suffix('.').unwrap_or(host);
            let entry = entry.strip_suffix('.').unwrap_or(entry);
            host == entry
                || host
                    .strip_suffix(entry)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        },
        // IP literals must match their canonical address, never a suffix.
        _ => host == entry,
    }
}

fn split_host_port(value: &str) -> Result<(Host, Option<u16>), String> {
    if let Some(rest) = value.strip_prefix('[') {
        let Some((host, rest)) = rest.split_once(']') else {
            return Err("missing closing ']'".to_owned());
        };
        let ip =
            Ipv6Addr::from_str(host).map_err(|_| format!("invalid IPv6 address \"{host}\""))?;
        let port = match rest {
            "" => None,
            rest => Some(parse_port_suffix(rest)?),
        };
        return Ok((Host::Ipv6(ip), port));
    }

    // An unbracketed IPv6 address has no port; ports must be bracketed.
    if value.matches(':').count() > 1 {
        let ip = Ipv6Addr::from_str(value)
            .map_err(|_| "invalid IPv6 address, bracket it to add a port".to_owned())?;
        return Ok((Host::Ipv6(ip), None));
    }

    match value.rsplit_once(':') {
        Some((host, port)) => {
            if host.is_empty() {
                return Err("missing host".to_owned());
            }
            let port = u16::from_str(port).map_err(|_| format!("invalid port \"{port}\""))?;
            Ok((parse_host(host), Some(port)))
        },
        None => Ok((parse_host(value), None)),
    }
}

fn parse_port_suffix(rest: &str) -> Result<u16, String> {
    let Some(port) = rest.strip_prefix(':') else {
        return Err(format!("unexpected \"{rest}\" after the IPv6 address"));
    };
    u16::from_str(port).map_err(|_| format!("invalid port \"{port}\""))
}

fn strip_scheme(entry: &str) -> &str {
    match entry.find("://") {
        Some(idx) => &entry[idx + 3..],
        None => entry,
    }
}

pub struct PolicyError {
    variable: String,
    entry: String,
    reason: String,
}

impl PolicyError {
    fn new(entry: &str, reason: impl Into<String>) -> Self {
        Self {
            variable: String::new(),
            entry: entry.to_owned(),
            reason: reason.into(),
        }
    }

    fn with_variable(mut self, variable: &str) -> Self {
        self.variable = variable.to_owned();
        self
    }
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.variable.is_empty() {
            write!(f, "{}: ", self.variable)?;
        }
        write!(
            f,
            "invalid network policy entry \"{}\": {}",
            self.entry, self.reason
        )
    }
}

// The runtime prints start-up failures with `{:?}`, so keep them readable.
impl fmt::Debug for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for PolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(raw: &str) -> PolicyEntry {
        PolicyEntry::parse(raw).unwrap()
    }

    fn host_entry(raw: &str) -> (Host, Option<u16>) {
        match entry(raw).kind {
            Kind::Host { host, port } => (host, port),
            Kind::Path => panic!("{raw} should be a host entry"),
        }
    }

    #[test]
    fn parses_documented_entry_forms() {
        for (raw, expected_host, expected_port) in [
            ("example.com", "example.com", None),
            ("EXAMPLE.com:8080", "example.com", Some(8080)),
            ("127.0.0.1", "127.0.0.1", None),
            ("[::1]", "::1", None),
            ("[::1]:443", "::1", Some(443)),
            ("::1", "::1", None),
            ("https://example.com/path", "example.com", None),
        ] {
            let (host, port) = host_entry(raw);
            assert_eq!(host, parse_host(expected_host), "{raw}");
            assert_eq!(port, expected_port, "{raw}");
        }
    }

    #[test]
    fn parses_socket_paths() {
        for (raw, path) in [
            ("/var/db/app.sock", "/var/db/app.sock"),
            ("unix:///var/db/app.sock", "/var/db/app.sock"),
            ("app.sock", "app.sock"),
        ] {
            assert!(entry(raw).matches_path(path), "{raw}");
        }
    }

    #[test]
    fn rejects_invalid_entries() {
        for raw in [
            "",
            "example.com:",
            "example.com:not-a-port",
            ":8080",
            "[::1",
            "[::1]:",
            "[::1]x",
            "fe80::1%en0",
        ] {
            let err = PolicyEntry::parse(raw).unwrap_err();
            assert!(
                err.to_string().starts_with("invalid network policy entry"),
                "{raw}: {err}"
            );
        }
    }

    #[test]
    fn bare_host_entries_cover_every_socket_port() {
        let entry = entry("127.0.0.1");
        let host = parse_host("127.0.0.1");
        assert!(entry.matches_network(&host, 80, &[]));
        assert!(entry.matches_network(&host, 9998, &[]));
        assert!(!entry.matches_network(&parse_host("127.0.0.2"), 9998, &[]));
    }

    #[test]
    fn bare_host_entries_cover_only_http_defaults_for_fetch() {
        let entry = entry("example.com");
        let host = parse_host("example.com");
        assert!(entry.matches_network(&host, 80, &[80, 443]));
        assert!(entry.matches_network(&host, 443, &[80, 443]));
        assert!(!entry.matches_network(&host, 8080, &[80, 443]));
    }

    #[test]
    fn explicit_ports_restrict_entries() {
        let entry = entry("example.com:8080");
        let host = parse_host("example.com");
        assert!(entry.matches_network(&host, 8080, &[]));
        assert!(entry.matches_network(&host, 8080, &[80, 443]));
        assert!(!entry.matches_network(&host, 80, &[]));
        assert!(!entry.matches_network(&host, 80, &[80, 443]));
    }

    #[test]
    fn reports_the_variable_and_entry() {
        let err = parse_list("LLRT_NET_DENY", ["example.com:not-a-port"]).unwrap_err();
        assert_eq!(
            err.to_string(),
            "LLRT_NET_DENY: invalid network policy entry \"example.com:not-a-port\": \
             invalid port \"not-a-port\""
        );
        assert_eq!(format!("{err:?}"), err.to_string());
    }

    #[test]
    fn matches_canonical_hosts_without_crossing_domain_boundaries() {
        for (entry_host, target, expected) in [
            ("EXAMPLE.COM", "example.com", true),
            ("example.com", "SUB.EXAMPLE.COM", true),
            ("example.com", "otherexample.com", false),
            ("example.com", "example.com.attacker.test", false),
            ("example.com.", "sub.example.com", true),
            ("example.com", "sub.example.com.", true),
            ("127.1", "127.0.0.1", true),
            ("2130706433", "127.0.0.1", true),
            ("0x7f000001", "127.0.0.1", true),
            ("0.0.1", "127.0.0.1", false),
            ("127.0.0.1", "evil.127.0.0.1", false),
            ("localhost", "127.0.0.1", false),
            ("[0:0:0:0:0:0:0:1]", "::1", true),
            ("[::1]", "[::2]", false),
            ("XN--BCHER-KVA.EXAMPLE", "xn--bcher-kva.example", true),
        ] {
            let entry_host = entry(entry_host);
            assert_eq!(
                entry_host.matches_network(&parse_host(target), 8080, &[]),
                expected,
                "{entry_host:?} vs {target}"
            );
        }
    }

    #[test]
    fn preserves_matching_for_hosts_outside_url_parser_grammar() {
        for (entry_host, target, expected) in [
            ("service.123", "service.123", true),
            ("SERVICE.123", "service.123", true),
            ("service.123", "child.service.123", true),
            ("service.123", "otherservice.123", false),
            ("service.123", "service.123.attacker.test", false),
        ] {
            let entry_host = entry(entry_host);
            assert_eq!(
                entry_host.matches_network(&parse_host(target), 49151, &[]),
                expected,
                "{entry_host:?} vs {target}"
            );
        }
    }
}
