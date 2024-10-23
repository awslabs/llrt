use std::{env, fs::File, io, result::Result as StdResult, time::Duration};

use llrt_modules::http::HttpVersion;
use rustls::{pki_types::CertificateDer, version, SupportedProtocolVersion};
use tracing::warn;

use crate::environment;

pub fn init() -> StdResult<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(pool_idle_timeout) = build_pool_idle_timeout() {
        llrt_modules::http::set_pool_idle_timeout(pool_idle_timeout);
    }

    if let Some(extra_ca_certs) = buid_extra_ca_certs()? {
        llrt_modules::http::set_extra_ca_certs(extra_ca_certs);
    }

    llrt_modules::http::set_tls_versions(build_tls_versions());

    llrt_modules::http::set_http_version(build_http_version());

    Ok(())
}

fn build_pool_idle_timeout() -> Option<Duration> {
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
    Some(Duration::from_secs(pool_idle_timeout))
}

fn buid_extra_ca_certs() -> StdResult<Option<Vec<CertificateDer<'static>>>, io::Error> {
    if let Ok(extra_ca_certs) = env::var(environment::ENV_LLRT_EXTRA_CA_CERTS) {
        if !extra_ca_certs.is_empty() {
            let file = File::open(extra_ca_certs) // This can be sync since we do this once when the VM starts
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

fn build_tls_versions() -> Vec<&'static SupportedProtocolVersion> {
    match env::var(environment::ENV_LLRT_TLS_VERSION).as_deref() {
        Ok("1.3") => vec![&version::TLS13, &version::TLS12],
        _ => vec![&version::TLS12], //Use TLS 1.2 by default to increase compat and keep latency low
    }
}

fn build_http_version() -> HttpVersion {
    match env::var(environment::ENV_LLRT_HTTP_VERSION).as_deref() {
        Ok("1.1") => HttpVersion::Http1_1,
        _ => HttpVersion::Http2,
    }
}
