// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::convert::Infallible;
use std::{io, sync::OnceLock, time::Duration};

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioTimer},
};
use llrt_utils::class::CustomInspectExtension;
use llrt_utils::result::ResultExt;
use once_cell::sync::Lazy;
use rquickjs::{Class, Ctx, Result};
use rustls::{
    crypto::ring, pki_types::CertificateDer, ClientConfig, RootCertStore, SupportedProtocolVersion,
};
use webpki_roots::TLS_SERVER_ROOTS;

pub use self::security::{get_allow_list, get_deny_list, set_allow_list, set_deny_list};
use self::{file::File, headers::Headers, request::Request, response::Response};

mod blob;
mod body;
mod fetch;
mod file;
mod headers;
mod incoming;
mod request;
mod response;
mod security;

const DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(15);

static CONNECTION_POOL_IDLE_TIMEOUT: OnceLock<Duration> = OnceLock::new();

pub fn set_pool_idle_timeout(timeout: Duration) {
    _ = CONNECTION_POOL_IDLE_TIMEOUT.set(timeout);
}

fn get_pool_idle_timeout() -> Duration {
    *CONNECTION_POOL_IDLE_TIMEOUT
        .get()
        .unwrap_or(&DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT)
}

static EXTRA_CA_CERTS: OnceLock<Vec<CertificateDer<'static>>> = OnceLock::new();

pub fn set_extra_ca_certs(certs: Vec<CertificateDer<'static>>) {
    _ = EXTRA_CA_CERTS.set(certs);
}

fn get_extra_ca_certs() -> Option<Vec<CertificateDer<'static>>> {
    EXTRA_CA_CERTS.get().cloned()
}

static TLS_VERSIONS: OnceLock<Vec<&'static SupportedProtocolVersion>> = OnceLock::new();

pub fn set_tls_versions(versions: Vec<&'static SupportedProtocolVersion>) {
    _ = TLS_VERSIONS.set(versions);
}

fn get_tls_versions() -> Option<Vec<&'static SupportedProtocolVersion>> {
    TLS_VERSIONS.get().cloned()
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

    Ok(match get_tls_versions() {
        Some(versions) => builder.with_protocol_versions(&versions),
        None => builder.with_safe_default_protocol_versions(),
    }
    .expect("TLS configuration failed")
    .with_root_certificates(root_certificates)
    .with_no_client_auth())
});

#[derive(Debug, Clone, Copy)]
pub enum HttpVersion {
    #[cfg(feature = "http1")]
    Http1_1,
    #[cfg(feature = "http2")]
    Http2,
}

static HTTP_VERSION: OnceLock<HttpVersion> = OnceLock::new();

pub fn set_http_version(version: HttpVersion) {
    _ = HTTP_VERSION.set(version);
}

fn get_http_version() -> HttpVersion {
    HTTP_VERSION.get().cloned().unwrap_or({
        #[cfg(feature = "http2")]
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

pub type HyperClient = Client<HttpsConnector<HttpConnector>, BoxBody<Bytes, Infallible>>;
pub static HTTP_CLIENT: Lazy<io::Result<HyperClient>> = Lazy::new(|| {
    let pool_idle_timeout = get_pool_idle_timeout();

    let maybe_tls_config = match &*TLS_CONFIG {
        Ok(tls_config) => io::Result::Ok(tls_config.clone()),
        Err(e) => io::Result::Err(io::Error::new(e.kind(), e.to_string())),
    };

    let builder = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(maybe_tls_config?)
        .https_or_http();

    let https = match get_http_version() {
        #[cfg(feature = "http1")]
        HttpVersion::Http1_1 => builder.enable_http1().build(),
        #[cfg(feature = "http2")]
        HttpVersion::Http2 => builder.enable_all_versions().build(),
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

    blob::init(ctx, &globals)?;

    Class::<File>::define(&globals)?;

    Ok(())
}
