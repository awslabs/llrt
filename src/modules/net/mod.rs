// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod socket;

use std::{env, time::Duration};

use bytes::Bytes;
use http_body_util::Full;
use hyper_rustls::HttpsConnector;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::{TokioExecutor, TokioTimer},
};
use once_cell::sync::Lazy;
use rquickjs::{
    cstr,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};
use rustls::{crypto::ring, ClientConfig, RootCertStore};
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

pub static HTTP_CLIENT: Lazy<Client<HttpsConnector<HttpConnector>, Full<Bytes>>> =
    Lazy::new(|| {
        let pool_idle_timeout: u64 = get_pool_idle_timeout();

        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(TLS_CONFIG.clone())
            .https_or_http()
            .enable_http1()
            .build();

        Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(pool_idle_timeout))
            .pool_timer(TokioTimer::new())
            .build(https)
    });

pub static TLS_CONFIG: Lazy<ClientConfig> = Lazy::new(|| {
    let mut root_certificates = RootCertStore::empty();

    for cert in TLS_SERVER_ROOTS.iter().cloned() {
        root_certificates.roots.push(cert)
    }

    ClientConfig::builder_with_provider(ring::default_provider().into())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_certificates)
        .with_no_client_auth()
});

pub struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        socket::declare(declare)?;
        declare.declare_static(cstr!("default"))?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
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
