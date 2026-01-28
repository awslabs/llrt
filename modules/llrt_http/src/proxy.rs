// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::sync::Arc;

use headers::{authorization::Basic, Authorization};
use hyper::Uri;
use hyper_http_proxy::{Intercept, Proxy, ProxyConnector};
use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use tracing::debug;

/// Global proxy configuration read from environment variables at startup
pub static PROXY_CONFIG: Lazy<ProxyConfig> = Lazy::new(ProxyConfig::from_env);

/// Default hosts that should bypass the proxy
const DEFAULT_BYPASS_HOSTS: &[&str] = &["localhost", "127.0.0.1", "::1"];

/// Proxy configuration parsed from environment variables
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub http_proxy: Option<Uri>,
    pub https_proxy: Option<Uri>,
    /// Pre-processed NO_PROXY patterns for efficient matching
    no_proxy_patterns: Vec<NoProxyPattern>,
}

/// A pre-processed NO_PROXY pattern for efficient matching
#[derive(Debug, Clone)]
enum NoProxyPattern {
    /// Matches all hosts
    Wildcard,
    /// Exact host match (lowercase), with optional port
    Exact(String, Option<u16>),
    /// Suffix match - pattern starts with dot (e.g., ".example.com"), with optional port
    Suffix(String, Option<u16>),
    /// Domain match - matches domain and all subdomains, with optional port
    Domain {
        exact: String,
        suffix: String,
        port: Option<u16>,
    },
}

/// Check if a request port matches a pattern port.
///
/// - If pattern has no port (None), it matches any request port
/// - If pattern has a port, request must have the same port
#[inline]
fn port_matches(request_port: Option<u16>, pattern_port: Option<u16>) -> bool {
    match pattern_port {
        None => true, // Pattern without port matches any port
        Some(p) => request_port == Some(p),
    }
}

/// Parse a NO_PROXY pattern into host and optional port.
///
/// Handles IPv6 addresses in brackets: `[::1]:8080`
fn parse_host_port(pattern: &str) -> (&str, Option<u16>) {
    // Handle IPv6 in brackets: [::1]:8080
    if pattern.starts_with('[') {
        if let Some(bracket_end) = pattern.find(']') {
            let host = &pattern[1..bracket_end];
            let remainder = &pattern[bracket_end + 1..];
            if let Some(port_str) = remainder.strip_prefix(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    return (host, Some(port));
                }
            }
            return (host, None);
        }
    }

    // Handle regular host:port (but not IPv6 without brackets)
    // Count colons - if more than one, it's likely IPv6 without port
    let colon_count = pattern.chars().filter(|&c| c == ':').count();
    if colon_count == 1 {
        if let Some(colon_pos) = pattern.rfind(':') {
            let host = &pattern[..colon_pos];
            let port_str = &pattern[colon_pos + 1..];
            if let Ok(port) = port_str.parse::<u16>() {
                return (host, Some(port));
            }
        }
    }

    (pattern, None)
}

impl ProxyConfig {
    /// Read proxy configuration from environment variables.
    ///
    /// Supports:
    /// - `HTTP_PROXY` / `http_proxy` - Proxy URL for HTTP requests
    /// - `HTTPS_PROXY` / `https_proxy` - Proxy URL for HTTPS requests
    /// - `ALL_PROXY` / `all_proxy` - Fallback proxy for both HTTP and HTTPS
    /// - `NO_PROXY` / `no_proxy` - Comma-separated list of hosts to bypass
    ///
    /// The proxy URL can include credentials: `http://user:pass@proxy:8080`
    ///
    /// Default bypass hosts (localhost, 127.0.0.1, ::1) are always included.
    pub fn from_env() -> Self {
        // Try specific proxy first, then fall back to ALL_PROXY
        let all_proxy = env::var("ALL_PROXY")
            .or_else(|_| env::var("all_proxy"))
            .ok()
            .and_then(|s| s.parse::<Uri>().ok());

        // Clone all_proxy for http since we need it again for https
        let http_proxy = env::var("HTTP_PROXY")
            .or_else(|_| env::var("http_proxy"))
            .ok()
            .and_then(|s| s.parse::<Uri>().ok())
            .or_else(|| all_proxy.clone());

        // Consume all_proxy here (no clone needed)
        let https_proxy = env::var("HTTPS_PROXY")
            .or_else(|_| env::var("https_proxy"))
            .ok()
            .and_then(|s| s.parse::<Uri>().ok())
            .or(all_proxy);

        // Parse NO_PROXY patterns
        let no_proxy_str = env::var("NO_PROXY")
            .or_else(|_| env::var("no_proxy"))
            .unwrap_or_default();

        // Default bypass hosts match any port
        let mut no_proxy_patterns: Vec<NoProxyPattern> = DEFAULT_BYPASS_HOSTS
            .iter()
            .map(|h| NoProxyPattern::Exact(h.to_lowercase(), None))
            .collect();

        for pattern in no_proxy_str.split(',').map(|s| s.trim().to_lowercase()) {
            if pattern.is_empty() {
                continue;
            }
            let parsed = if pattern == "*" {
                NoProxyPattern::Wildcard
            } else if pattern.starts_with('.') {
                // Suffix pattern like ".example.com" or ".example.com:8080"
                let (host, port) = parse_host_port(&pattern);
                NoProxyPattern::Suffix(host.to_string(), port)
            } else {
                // Domain pattern like "example.com" or "example.com:8080"
                let (host, port) = parse_host_port(&pattern);
                NoProxyPattern::Domain {
                    exact: host.to_string(),
                    suffix: format!(".{}", host),
                    port,
                }
            };
            no_proxy_patterns.push(parsed);
        }

        let config = Self {
            http_proxy,
            https_proxy,
            no_proxy_patterns,
        };

        // Log proxy configuration for debugging
        if config.is_enabled() {
            debug!(
                http_proxy = ?config.http_proxy.as_ref().and_then(strip_userinfo),
                https_proxy = ?config.https_proxy.as_ref().and_then(strip_userinfo),
                no_proxy_count = config.no_proxy_patterns.len(),
                "Proxy configuration loaded from environment"
            );
        }

        config
    }

    /// Check if a host should bypass the proxy based on NO_PROXY rules.
    ///
    /// Matching rules:
    /// - `*` matches all hosts
    /// - `.example.com` matches `foo.example.com` but not `example.com`
    /// - `example.com` matches `example.com` and `foo.example.com`
    /// - `example.com:8080` matches only when port is 8080
    /// - `localhost`, `127.0.0.1`, `::1` are always bypassed
    pub fn should_bypass(&self, host: &str, port: Option<u16>) -> bool {
        let host_lower = host.to_lowercase();
        self.no_proxy_patterns.iter().any(|pattern| match pattern {
            NoProxyPattern::Wildcard => true,
            NoProxyPattern::Exact(exact, pattern_port) => {
                host_lower == *exact && port_matches(port, *pattern_port)
            },
            NoProxyPattern::Suffix(suffix, pattern_port) => {
                host_lower.ends_with(suffix) && port_matches(port, *pattern_port)
            },
            NoProxyPattern::Domain {
                exact,
                suffix,
                port: pattern_port,
            } => {
                (host_lower == *exact || host_lower.ends_with(suffix))
                    && port_matches(port, *pattern_port)
            },
        })
    }

    /// Check if any proxy is configured
    pub fn is_enabled(&self) -> bool {
        self.http_proxy.is_some() || self.https_proxy.is_some()
    }
}

/// Extract basic auth credentials from a proxy URI.
///
/// Parses URIs like `http://user:password@proxy:8080` and returns
/// an Authorization header. Properly handles URL-encoded credentials.
pub fn extract_basic_auth(uri: &Uri) -> Option<Authorization<Basic>> {
    let authority = uri.authority()?;
    let authority_str = authority.as_str();

    // Find the @ separator
    let at_pos = authority_str.find('@')?;
    let userinfo = &authority_str[..at_pos];

    // Split into username and password
    let colon_pos = userinfo.find(':')?;
    let username = &userinfo[..colon_pos];
    let password = &userinfo[colon_pos + 1..];

    // URL decode the credentials (handles UTF-8 properly)
    let username = percent_decode_str(username).decode_utf8().ok()?.to_string();
    let password = percent_decode_str(password).decode_utf8().ok()?.to_string();

    Some(Authorization::basic(&username, &password))
}

/// Create a proxy URI without credentials (for the actual proxy connection).
///
/// Strips the userinfo from URIs like `http://user:password@proxy:8080`
/// to produce `http://proxy:8080`.
pub fn strip_userinfo(uri: &Uri) -> Option<Uri> {
    let authority = uri.authority()?;
    let authority_str = authority.as_str();

    // Check if there's userinfo to strip
    if let Some(at_pos) = authority_str.find('@') {
        let host_port = &authority_str[at_pos + 1..];
        let scheme = uri.scheme_str().unwrap_or("http");
        let path = uri.path();

        format!("{}://{}{}", scheme, host_port, path)
            .parse::<Uri>()
            .ok()
    } else {
        Some(uri.clone())
    }
}

/// Create an Intercept that respects NO_PROXY for HTTP requests.
fn create_http_intercept(config: Arc<ProxyConfig>) -> Intercept {
    let custom: hyper_http_proxy::Custom =
        (move |scheme: Option<&str>, host: Option<&str>, port: Option<u16>| {
            // Only intercept HTTP requests (not HTTPS)
            if scheme != Some("http") {
                return false;
            }
            if let Some(host) = host {
                !config.should_bypass(host, port)
            } else {
                false
            }
        })
        .into();
    Intercept::Custom(custom)
}

/// Create an Intercept that respects NO_PROXY for HTTPS requests.
fn create_https_intercept(config: Arc<ProxyConfig>) -> Intercept {
    let custom: hyper_http_proxy::Custom =
        (move |scheme: Option<&str>, host: Option<&str>, port: Option<u16>| {
            // Only intercept HTTPS requests
            if scheme != Some("https") {
                return false;
            }
            if let Some(host) = host {
                !config.should_bypass(host, port)
            } else {
                false
            }
        })
        .into();
    Intercept::Custom(custom)
}

/// Configure proxies on a ProxyConnector based on the given ProxyConfig.
pub fn configure_proxies<C>(connector: &mut ProxyConnector<C>, config: &ProxyConfig) {
    let config_arc = Arc::new(config.clone());

    if let Some(http_uri) = &config.http_proxy {
        let proxy_uri = strip_userinfo(http_uri).unwrap_or_else(|| http_uri.clone());
        let intercept = create_http_intercept(Arc::clone(&config_arc));
        let mut proxy = Proxy::new(intercept, proxy_uri);

        if let Some(auth) = extract_basic_auth(http_uri) {
            proxy.set_authorization(auth);
        }

        connector.add_proxy(proxy);
    }

    if let Some(https_uri) = &config.https_proxy {
        let proxy_uri = strip_userinfo(https_uri).unwrap_or_else(|| https_uri.clone());
        let intercept = create_https_intercept(Arc::clone(&config_arc));
        let mut proxy = Proxy::new(intercept, proxy_uri);

        if let Some(auth) = extract_basic_auth(https_uri) {
            proxy.set_authorization(auth);
        }

        connector.add_proxy(proxy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_bypass_exact_match() {
        let config = ProxyConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy_patterns: vec![
                NoProxyPattern::Exact("localhost".to_string(), None),
                NoProxyPattern::Domain {
                    exact: "example.com".to_string(),
                    suffix: ".example.com".to_string(),
                    port: None,
                },
            ],
        };

        assert!(config.should_bypass("localhost", None));
        assert!(config.should_bypass("example.com", None));
        assert!(config.should_bypass("LOCALHOST", None));
        assert!(config.should_bypass("sub.example.com", None));
        assert!(!config.should_bypass("other.com", None));
    }

    #[test]
    fn test_should_bypass_dot_prefix() {
        let config = ProxyConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy_patterns: vec![NoProxyPattern::Suffix(".internal.com".to_string(), None)],
        };

        assert!(config.should_bypass("host.internal.com", None));
        assert!(config.should_bypass("deep.host.internal.com", None));
        assert!(!config.should_bypass("internal.com", None));
    }

    #[test]
    fn test_should_bypass_wildcard() {
        let config = ProxyConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy_patterns: vec![NoProxyPattern::Wildcard],
        };

        assert!(config.should_bypass("anything.com", None));
        assert!(config.should_bypass("localhost", Some(8080)));
    }

    #[test]
    fn test_default_bypass_hosts() {
        let config = ProxyConfig::from_env();

        // These should always be bypassed (any port)
        assert!(config.should_bypass("localhost", None));
        assert!(config.should_bypass("localhost", Some(3000)));
        assert!(config.should_bypass("127.0.0.1", Some(8080)));
        assert!(config.should_bypass("::1", None));
        assert!(config.should_bypass("LOCALHOST", Some(443)));
    }

    #[test]
    fn test_should_bypass_with_port() {
        let config = ProxyConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy_patterns: vec![
                // localhost:3000 - only matches port 3000
                NoProxyPattern::Exact("localhost".to_string(), Some(3000)),
                // example.com:8080 - domain with specific port
                NoProxyPattern::Domain {
                    exact: "example.com".to_string(),
                    suffix: ".example.com".to_string(),
                    port: Some(8080),
                },
                // .internal.com:443 - suffix with specific port
                NoProxyPattern::Suffix(".internal.com".to_string(), Some(443)),
            ],
        };

        // localhost:3000 matches
        assert!(config.should_bypass("localhost", Some(3000)));
        // localhost:8080 does NOT match (wrong port)
        assert!(!config.should_bypass("localhost", Some(8080)));
        // localhost without port does NOT match
        assert!(!config.should_bypass("localhost", None));

        // example.com:8080 matches
        assert!(config.should_bypass("example.com", Some(8080)));
        // sub.example.com:8080 matches
        assert!(config.should_bypass("sub.example.com", Some(8080)));
        // example.com:443 does NOT match (wrong port)
        assert!(!config.should_bypass("example.com", Some(443)));

        // host.internal.com:443 matches
        assert!(config.should_bypass("host.internal.com", Some(443)));
        // host.internal.com:8080 does NOT match (wrong port)
        assert!(!config.should_bypass("host.internal.com", Some(8080)));
    }

    #[test]
    fn test_should_bypass_pattern_without_port_matches_any() {
        let config = ProxyConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy_patterns: vec![
                // localhost without port - matches any port
                NoProxyPattern::Exact("localhost".to_string(), None),
            ],
        };

        assert!(config.should_bypass("localhost", None));
        assert!(config.should_bypass("localhost", Some(80)));
        assert!(config.should_bypass("localhost", Some(443)));
        assert!(config.should_bypass("localhost", Some(8080)));
    }

    #[test]
    fn test_parse_host_port() {
        // Regular host without port
        assert_eq!(parse_host_port("example.com"), ("example.com", None));

        // Host with port
        assert_eq!(
            parse_host_port("example.com:8080"),
            ("example.com", Some(8080))
        );

        // Suffix with port
        assert_eq!(
            parse_host_port(".example.com:443"),
            (".example.com", Some(443))
        );

        // IPv6 without brackets (no port)
        assert_eq!(parse_host_port("::1"), ("::1", None));

        // IPv6 in brackets without port
        assert_eq!(parse_host_port("[::1]"), ("::1", None));

        // IPv6 in brackets with port
        assert_eq!(parse_host_port("[::1]:8080"), ("::1", Some(8080)));

        // IPv6 in brackets with port
        assert_eq!(
            parse_host_port("[2001:db8::1]:443"),
            ("2001:db8::1", Some(443))
        );
    }

    #[test]
    fn test_extract_basic_auth() {
        let uri: Uri = "http://user:pass@proxy.example.com:8080".parse().unwrap();
        let auth = extract_basic_auth(&uri);
        assert!(auth.is_some());
    }

    #[test]
    fn test_extract_basic_auth_encoded() {
        // Test URL-encoded credentials with special characters
        let uri: Uri = "http://user%40domain:p%40ss%3Aword@proxy.example.com:8080"
            .parse()
            .unwrap();
        let auth = extract_basic_auth(&uri);
        assert!(auth.is_some());
    }

    #[test]
    fn test_extract_basic_auth_no_credentials() {
        let uri: Uri = "http://proxy.example.com:8080".parse().unwrap();
        let auth = extract_basic_auth(&uri);
        assert!(auth.is_none());
    }

    #[test]
    fn test_strip_userinfo() {
        let uri: Uri = "http://user:pass@proxy.example.com:8080/path"
            .parse()
            .unwrap();
        let stripped = strip_userinfo(&uri).unwrap();
        assert_eq!(stripped.to_string(), "http://proxy.example.com:8080/path");
    }

    #[test]
    fn test_strip_userinfo_no_credentials() {
        let uri: Uri = "http://proxy.example.com:8080".parse().unwrap();
        let stripped = strip_userinfo(&uri).unwrap();
        // When no credentials are present, returns the original URI
        assert_eq!(stripped.to_string(), uri.to_string());
    }

    #[test]
    fn test_is_enabled() {
        let config = ProxyConfig {
            http_proxy: Some("http://proxy:8080".parse().unwrap()),
            https_proxy: None,
            no_proxy_patterns: vec![],
        };
        assert!(config.is_enabled());

        let config = ProxyConfig {
            http_proxy: None,
            https_proxy: None,
            no_proxy_patterns: vec![],
        };
        assert!(!config.is_enabled());
    }
}
