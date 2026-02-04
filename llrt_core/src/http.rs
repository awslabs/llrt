// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{env, result::Result as StdResult};

use tracing::warn;

use crate::environment;
use crate::modules::https::{set_http_version, set_pool_idle_timeout_seconds, HttpVersion};

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
use std::{fs::File, io};

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
use rustls::{pki_types::CertificateDer, version, SupportedProtocolVersion};

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
use crate::modules::tls::{set_extra_ca_certs, set_tls_versions};

#[cfg(feature = "tls-openssl")]
use crate::modules::tls::set_tls_version;

pub fn init() -> StdResult<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(pool_idle_timeout) = build_pool_idle_timeout() {
        set_pool_idle_timeout_seconds(pool_idle_timeout);
    }

    #[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
    {
        if let Some(extra_ca_certs) = build_extra_ca_certs()? {
            set_extra_ca_certs(extra_ca_certs);
        }
        set_tls_versions(build_tls_versions());
    }

    #[cfg(feature = "tls-openssl")]
    {
        set_tls_version(build_tls_version_openssl());
    }

    set_http_version(build_http_version());

    Ok(())
}

fn build_pool_idle_timeout() -> Option<u64> {
    let Ok(env_value) = env::var(environment::ENV_LLRT_NET_POOL_IDLE_TIMEOUT) else {
        return None;
    };
    let Ok(pool_idle_timeout) = env_value.parse::<u64>() else {
        return None;
    };

    if pool_idle_timeout > 300 {
        warn!(
            r#""{}" is exceeds 300s (5min), risking errors due to possible server connection closures."#,
            environment::ENV_LLRT_NET_POOL_IDLE_TIMEOUT
        )
    }
    Some(pool_idle_timeout)
}

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
fn build_extra_ca_certs() -> StdResult<Option<Vec<CertificateDer<'static>>>, io::Error> {
    if let Ok(extra_ca_certs) = env::var(environment::ENV_LLRT_EXTRA_CA_CERTS) {
        if !extra_ca_certs.is_empty() {
            let file = File::open(extra_ca_certs)
                .map_err(|_| io::Error::other("Failed to open extra CA certificates file"))?;
            let mut reader = io::BufReader::new(file);
            return Ok(Some(
                rustls_pemfile::certs(&mut reader)
                    .filter_map(io::Result::ok)
                    .collect(),
            ));
        }
    }
    Ok(None)
}

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
fn build_tls_versions() -> Vec<&'static SupportedProtocolVersion> {
    match env::var(environment::ENV_LLRT_TLS_VERSION).as_deref() {
        Ok("1.3") => vec![&version::TLS13, &version::TLS12],
        _ => vec![&version::TLS12],
    }
}

#[cfg(feature = "tls-openssl")]
fn build_tls_version_openssl() -> Option<openssl::ssl::SslVersion> {
    match env::var(environment::ENV_LLRT_TLS_VERSION).as_deref() {
        Ok("1.3") => Some(openssl::ssl::SslVersion::TLS1_3),
        _ => Some(openssl::ssl::SslVersion::TLS1_2),
    }
}

fn build_http_version() -> HttpVersion {
    match env::var(environment::ENV_LLRT_HTTP_VERSION).as_deref() {
        Ok("2") => HttpVersion::Http2,
        _ => HttpVersion::Http1_1,
    }
}
