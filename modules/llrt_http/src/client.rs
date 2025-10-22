use std::{convert::Infallible, io};

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioTimer},
};
use llrt_dns_cache::CachedDnsResolver;
use llrt_tls::TLS_CONFIG;
use once_cell::sync::Lazy;
use rustls::ClientConfig;

use crate::{get_http_version, get_pool_idle_timeout, HttpVersion};

pub type HyperClient =
    Client<HttpsConnector<HttpConnector<CachedDnsResolver>>, BoxBody<Bytes, Infallible>>;
pub static HTTP_CLIENT: Lazy<io::Result<HyperClient>> = Lazy::new(|| build_client(None));

pub fn build_client(tls_config: Option<ClientConfig>) -> io::Result<HyperClient> {
    let pool_idle_timeout = get_pool_idle_timeout();

    let config = if let Some(tls_config) = tls_config {
        tls_config
    } else {
        match &*TLS_CONFIG {
            Ok(tls_config) => tls_config.clone(),
            Err(e) => return io::Result::Err(io::Error::new(e.kind(), e.to_string())),
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

    Ok(Client::builder(TokioExecutor::new())
        .pool_idle_timeout(pool_idle_timeout)
        .pool_timer(TokioTimer::new())
        .build(https))
}
