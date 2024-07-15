// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod socket;

use std::{env, fs::File, io, time::Duration};

use bytes::Bytes;
use http_body_util::Full;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioTimer},
};
use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};
use rustls::{crypto::ring, version, ClientConfig, RootCertStore};
use tracing::warn;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{environment, module_builder::ModuleInfo};

pub const DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT_SECONDS: u64 = 15;

pub fn get_pool_idle_timeout() -> u64 {
    let pool_idle_timeout: u64 = env::var(environment::ENV_LLRT_NET_POOL_IDLE_TIMEOUT)
        .map(|timeout| {
            timeout
                .parse()
                .unwrap_or(DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT_SECONDS)
        })
        .unwrap_or(DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT_SECONDS);
    if pool_idle_timeout > 300 {
        warn!(
            r#""{}" is exceeds 300s (5min), risking errors due to possible server connection closures."#,
            environment::ENV_LLRT_NET_POOL_IDLE_TIMEOUT
        )
    }
    pool_idle_timeout
}

pub static HTTP_CLIENT: Lazy<io::Result<Client<HttpsConnector<HttpConnector>, Full<Bytes>>>> =
    Lazy::new(|| {
        let pool_idle_timeout: u64 = get_pool_idle_timeout();

        let maybe_tls_config = match &*TLS_CONFIG {
            Ok(tls_config) => io::Result::Ok(tls_config.clone()),
            Err(e) => io::Result::Err(io::Error::new(e.kind(), e.to_string())),
        };

        let builder = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(maybe_tls_config?)
            .https_or_http();

        let https = match env::var(environment::ENV_LLRT_HTTP_VERSION).as_deref() {
            Ok("1.1") => builder.enable_http1().build(),
            _ => builder.enable_all_versions().build(),
        };

        Ok(Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(pool_idle_timeout))
            .pool_timer(TokioTimer::new())
            .build(https))
    });

pub static TLS_CONFIG: Lazy<io::Result<ClientConfig>> = Lazy::new(|| {
    let mut root_certificates = RootCertStore::empty();

    for cert in TLS_SERVER_ROOTS.iter().cloned() {
        root_certificates.roots.push(cert)
    }

    if let Ok(extra_ca_certs) = env::var(environment::ENV_LLRT_EXTRA_CA_CERTS) {
        if !extra_ca_certs.is_empty() {
            let file = File::open(extra_ca_certs)
                .map_err(|_| io::Error::other("Failed to open extra CA certificates file"))?;
            let mut reader = io::BufReader::new(file);
            root_certificates.add_parsable_certificates(
                rustls_pemfile::certs(&mut reader).filter_map(io::Result::ok),
            );
        }
    }

    let builder = ClientConfig::builder_with_provider(ring::default_provider().into());

    Ok(
        match env::var(environment::ENV_LLRT_TLS_VERSION).as_deref() {
            Ok("1.3") => builder.with_safe_default_protocol_versions(),
            _ => builder.with_protocol_versions(&[&version::TLS12]), //Use TLS 1.2 by default to increase compat and keep latency low
        }
        .unwrap()
        .with_root_certificates(root_certificates)
        .with_no_client_auth(),
    )
});

pub struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declare: &Declarations) -> Result<()> {
        socket::declare(declare)?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        socket::init(ctx.clone(), exports)?;
        Ok(())
    }
}

impl From<NetModule> for ModuleInfo<NetModule> {
    fn from(val: NetModule) -> Self {
        ModuleInfo {
            name: "net",
            module: val,
        }
    }
}
