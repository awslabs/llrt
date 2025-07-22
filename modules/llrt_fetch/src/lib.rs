// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    borrow::Cow,
    convert::Infallible,
    io,
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::Duration,
};

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioTimer},
};
use llrt_buffer::Blob;
use llrt_dns_cache::CachedDnsResolver;
use llrt_utils::{class::CustomInspectExtension, result::ResultExt};
use once_cell::sync::Lazy;
use rquickjs::{Class, Ctx, Result};
use rustls::{
    crypto::ring, pki_types::CertificateDer, ClientConfig, RootCertStore, SupportedProtocolVersion,
};
use webpki_roots::TLS_SERVER_ROOTS;

pub use self::security::{get_allow_list, get_deny_list, set_allow_list, set_deny_list};
use self::{headers::Headers, request::Request, response::Response};

mod body;
pub mod fetch;
pub mod headers;
mod incoming;
pub mod request;
pub mod response;
mod security;

static CONNECTION_POOL_IDLE_TIMEOUT: AtomicU64 = AtomicU64::new(15);

const MIME_TYPE_APPLICATION: &str = "application/x-www-form-urlencoded;charset=UTF-8";
const MIME_TYPE_TEXT: &str = "text/plain;charset=UTF-8";

pub fn set_pool_idle_timeout_seconds(seconds: u64) {
    CONNECTION_POOL_IDLE_TIMEOUT.store(seconds, Ordering::Relaxed);
}

fn get_pool_idle_timeout() -> Duration {
    Duration::from_secs(CONNECTION_POOL_IDLE_TIMEOUT.load(Ordering::Relaxed))
}

static EXTRA_CA_CERTS: OnceLock<Vec<CertificateDer<'static>>> = OnceLock::new();

pub fn set_extra_ca_certs(certs: Vec<CertificateDer<'static>>) {
    _ = EXTRA_CA_CERTS.set(certs);
}

fn get_extra_ca_certs() -> Option<Vec<CertificateDer<'static>>> {
    let certs = EXTRA_CA_CERTS.get_or_init(Vec::new).clone();
    if certs.is_empty() {
        None
    } else {
        Some(certs)
    }
}

static TLS_VERSIONS: OnceLock<Vec<&'static SupportedProtocolVersion>> = OnceLock::new();

pub fn set_tls_versions(versions: Vec<&'static SupportedProtocolVersion>) {
    _ = TLS_VERSIONS.set(versions);
}

fn get_tls_versions() -> Option<Vec<&'static SupportedProtocolVersion>> {
    let versions = TLS_VERSIONS.get_or_init(Vec::new).clone();
    if versions.is_empty() {
        None
    } else {
        Some(versions)
    }
}

static TLS_CONFIG: Lazy<io::Result<ClientConfig>> = Lazy::new(|| {
    let mut root_certificates = RootCertStore::empty();

    for cert in TLS_SERVER_ROOTS.iter().cloned() {
        root_certificates.roots.push(cert)
    }

    if let Some(extra_ca_certs) = get_extra_ca_certs() {
        root_certificates.add_parsable_certificates(extra_ca_certs);
    }

    let builder = ClientConfig::builder_with_provider(ring::default_provider().into());

    let client_config = match get_tls_versions() {
        Some(versions) => builder.with_protocol_versions(&versions),
        None => builder.with_safe_default_protocol_versions(),
    }
    .expect("TLS configuration failed")
    .with_root_certificates(root_certificates)
    .with_no_client_auth();
    Ok(client_config)
});

#[derive(Debug, Clone, Copy)]
pub enum HttpVersion {
    Http1_1,
    Http2,
}

static HTTP_VERSION: OnceLock<HttpVersion> = OnceLock::new();

pub fn set_http_version(version: HttpVersion) {
    _ = HTTP_VERSION.set(version);
}

fn get_http_version() -> HttpVersion {
    *HTTP_VERSION.get_or_init(|| {
        #[cfg(all(feature = "http2", feature = "http1"))]
        {
            HttpVersion::Http1_1
        }
        #[cfg(all(not(feature = "http1"), feature = "http2"))]
        {
            HttpVersion::Http2
        }
        #[cfg(all(not(feature = "http2"), feature = "http1"))]
        {
            HttpVersion::Http1_1
        }
        #[cfg(all(not(feature = "http2"), not(feature = "http1")))]
        {
            compile_error!("Either the `http1` or `http2` feature must be enabled")
        }
    })
}

pub(crate) fn strip_bom<'a>(bytes: impl Into<Cow<'a, [u8]>>) -> Cow<'a, [u8]> {
    let cow = bytes.into();
    if cow.starts_with(&[0xEF, 0xBB, 0xBF]) {
        match cow {
            Cow::Borrowed(b) => Cow::Borrowed(&b[3..]),
            Cow::Owned(b) => Cow::Owned(b[3..].to_vec()),
        }
    } else {
        cow
    }
}

pub type HyperClient =
    Client<HttpsConnector<HttpConnector<CachedDnsResolver>>, BoxBody<Bytes, Infallible>>;
pub static HTTP_CLIENT: Lazy<io::Result<HyperClient>> = Lazy::new(|| {
    let pool_idle_timeout = get_pool_idle_timeout();

    let maybe_tls_config = match &*TLS_CONFIG {
        Ok(tls_config) => io::Result::Ok(tls_config.clone()),
        Err(e) => io::Result::Err(io::Error::new(e.kind(), e.to_string())),
    };

    let builder = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(maybe_tls_config?)
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
});

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = ctx.globals();

    //init eagerly
    fetch::init(HTTP_CLIENT.as_ref().or_throw(ctx)?.clone(), &globals)?;

    Class::<Request>::define(&globals)?;
    Class::<Response>::define(&globals)?;
    Class::<Headers>::define_with_custom_inspect(&globals)?;

    Ok(())
}
