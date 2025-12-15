use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioTimer},
};
use llrt_dns_cache::CachedDnsResolver;
use once_cell::sync::Lazy;

use crate::{get_http_version, get_pool_idle_timeout};

// Rustls-based TLS backends
#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
mod rustls_client {
    use super::*;
    use hyper_rustls::HttpsConnector;
    use llrt_tls::TLS_CONFIG;
    use rustls::ClientConfig;

    #[cfg(feature = "http2")]
    use crate::HttpVersion;

    pub type HyperClient =
        Client<HttpsConnector<HttpConnector<CachedDnsResolver>>, BoxBody<Bytes, Infallible>>;

    pub static HTTP_CLIENT: Lazy<Result<HyperClient, Box<dyn std::error::Error + Send + Sync>>> =
        Lazy::new(|| build_client(None));

    pub fn build_client(
        tls_config: Option<ClientConfig>,
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

        Ok(Client::builder(TokioExecutor::new())
            .pool_idle_timeout(pool_idle_timeout)
            .pool_timer(TokioTimer::new())
            .build(https))
    }
}

// OpenSSL TLS backend
#[cfg(feature = "tls-openssl")]
mod openssl_client {
    use super::*;
    use hyper_openssl::client::legacy::HttpsConnector;
    use llrt_tls::TLS_CONFIG;
    use openssl::ssl::SslConnectorBuilder;

    pub type HyperClient =
        Client<HttpsConnector<HttpConnector<CachedDnsResolver>>, BoxBody<Bytes, Infallible>>;

    pub static HTTP_CLIENT: Lazy<Result<HyperClient, Box<dyn std::error::Error + Send + Sync>>> =
        Lazy::new(|| build_client(None));

    pub fn build_client(
        tls_config: Option<SslConnectorBuilder>,
    ) -> Result<HyperClient, Box<dyn std::error::Error + Send + Sync>> {
        let pool_idle_timeout = get_pool_idle_timeout();

        let connector = if let Some(tls_config) = tls_config {
            tls_config
        } else {
            match TLS_CONFIG.as_ref() {
                Ok(builder) => {
                    // Clone the builder by creating a new one with same settings
                    llrt_tls::build_client_config(llrt_tls::BuildClientConfigOptions {
                        reject_unauthorized: true,
                        ca: None,
                    })?
                },
                Err(e) => return Err(e.to_string().into()),
            }
        };

        let mut cache_dns_connector = CachedDnsResolver::new().into_http_connector();
        cache_dns_connector.enforce_http(false);

        let mut https = HttpsConnector::with_connector(cache_dns_connector, connector)?;
        https.set_callback(|ssl, _| {
            ssl.set_alpn_protos(b"\x02h2\x08http/1.1")?;
            Ok(())
        });

        Ok(Client::builder(TokioExecutor::new())
            .pool_idle_timeout(pool_idle_timeout)
            .pool_timer(TokioTimer::new())
            .build(https))
    }
}

// Re-export based on feature
#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
pub use rustls_client::*;

#[cfg(feature = "tls-openssl")]
pub use openssl_client::*;
