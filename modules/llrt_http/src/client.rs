use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper_http_proxy::ProxyConnector;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioTimer},
};
use llrt_dns_cache::CachedDnsResolver;
use llrt_tls::TLS_CONFIG;
use once_cell::sync::Lazy;
use rustls::ClientConfig;

use crate::{
    get_http_version, get_pool_idle_timeout,
    proxy::{configure_proxies, ProxyConfig, PROXY_CONFIG},
    HttpVersion,
};

/// The inner HTTPS connector type (without proxy)
pub type HttpsConnectorType = HttpsConnector<HttpConnector<CachedDnsResolver>>;

/// The main HTTP client type with optional proxy support.
/// When no proxy is configured, ProxyConnector acts as a passthrough.
pub type HyperClient = Client<ProxyConnector<HttpsConnectorType>, BoxBody<Bytes, Infallible>>;

/// Global HTTP client, lazily initialized with proxy support from environment variables.
pub static HTTP_CLIENT: Lazy<Result<HyperClient, Box<dyn std::error::Error + Send + Sync>>> =
    Lazy::new(|| {
        let proxy_config = if PROXY_CONFIG.is_enabled() {
            Some(&*PROXY_CONFIG)
        } else {
            None
        };
        build_client(None, proxy_config)
    });

/// Build an HTTP client with optional custom TLS config and proxy configuration.
///
/// # Arguments
/// * `tls_config` - Optional custom TLS configuration. If None, uses global TLS config.
/// * `proxy_config` - Optional proxy configuration. If None, no proxies are configured.
pub fn build_client(
    tls_config: Option<ClientConfig>,
    proxy_config: Option<&ProxyConfig>,
) -> Result<HyperClient, Box<dyn std::error::Error + Send + Sync>> {
    let pool_idle_timeout = get_pool_idle_timeout();

    let config = if let Some(tls_config) = tls_config {
        tls_config
    } else {
        match &*TLS_CONFIG {
            Ok(tls_config) => tls_config.clone(),
            Err(e) => return Err(e.to_string().into()),
        }
    };

    let builder = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(config)
        .https_or_http();

    let mut cache_dns_connector = CachedDnsResolver::new().into_http_connector();
    cache_dns_connector.enforce_http(false);

    let https = match get_http_version() {
        #[cfg(feature = "http2")]
        HttpVersion::Http2 => builder
            .enable_all_versions()
            .wrap_connector(cache_dns_connector),
        _ => builder.enable_http1().wrap_connector(cache_dns_connector),
    };

    // Wrap the HTTPS connector with ProxyConnector
    // When no proxies are configured, it acts as a passthrough
    let mut proxy_connector = ProxyConnector::unsecured(https);

    if let Some(proxy_config) = proxy_config {
        configure_proxies(&mut proxy_connector, proxy_config);
    }

    Ok(Client::builder(TokioExecutor::new())
        .pool_idle_timeout(pool_idle_timeout)
        .pool_timer(TokioTimer::new())
        .build(proxy_connector))
}
