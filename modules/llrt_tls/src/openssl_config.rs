// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::OnceLock;

use once_cell::sync::Lazy;
use openssl::ssl::{SslConnectorBuilder, SslMethod, SslVerifyMode};
use openssl::x509::X509;

use crate::client_auth::KEY_MISMATCH;
use crate::openssl_key::load_private_key;
use crate::BuildClientConfigOptions;

static EXTRA_CA_CERTS: OnceLock<Vec<Vec<u8>>> = OnceLock::new();

pub fn set_extra_ca_certs(certs: Vec<Vec<u8>>) {
    _ = EXTRA_CA_CERTS.set(certs);
}

pub fn get_extra_ca_certs() -> Option<Vec<Vec<u8>>> {
    let certs = EXTRA_CA_CERTS.get_or_init(Vec::new).clone();
    if certs.is_empty() {
        None
    } else {
        Some(certs)
    }
}

static TLS_VERSION: OnceLock<Option<openssl::ssl::SslVersion>> = OnceLock::new();

pub fn set_tls_version(version: Option<openssl::ssl::SslVersion>) {
    _ = TLS_VERSION.set(version);
}

pub fn get_tls_version() -> Option<openssl::ssl::SslVersion> {
    *TLS_VERSION.get_or_init(|| None)
}

pub static TLS_CONFIG: Lazy<Result<SslConnectorBuilder, Box<dyn std::error::Error + Send + Sync>>> =
    Lazy::new(|| build_client_config(BuildClientConfigOptions::default()));

pub fn build_client_config(
    options: BuildClientConfigOptions,
) -> Result<SslConnectorBuilder, Box<dyn std::error::Error + Send + Sync>> {
    let mut builder = openssl::ssl::SslConnector::builder(SslMethod::tls_client())?;

    // TLS version
    if let Some(version) = get_tls_version() {
        builder.set_min_proto_version(Some(version))?;
    }

    // Certificate verification
    if !options.reject_unauthorized {
        builder.set_verify(SslVerifyMode::NONE);
    } else if let Some(ca) = options.ca {
        for cert_pem in ca {
            let cert = X509::from_pem(&cert_pem)?;
            builder.cert_store_mut().add_cert(cert)?;
        }
    } else {
        // Use system default CA certificates
        builder.set_default_verify_paths()?;

        // Add extra CA certs if configured
        if let Some(extra_certs) = get_extra_ca_certs() {
            for cert_der in extra_certs {
                if let Ok(cert) = X509::from_der(&cert_der) {
                    let _ = builder.cert_store_mut().add_cert(cert);
                }
            }
        }
    }

    // Client authentication (mutual TLS)
    match (options.cert, options.key) {
        (Some(cert), Some(key)) => {
            let mut cert_chain = X509::stack_from_pem(&cert)
                .map_err(|e| format!("Invalid cert: {e}"))?
                .into_iter();
            let leaf = cert_chain
                .next()
                .ok_or("Invalid cert: no certificate found")?;

            let key = load_private_key(&key, options.passphrase.as_deref())?;
            if !leaf.public_key()?.public_eq(&key) {
                return Err(KEY_MISMATCH.into());
            }

            builder.set_certificate(&leaf)?;
            for intermediate in cert_chain {
                builder.add_extra_chain_cert(intermediate)?;
            }
            builder.set_private_key(&key)?;
        },
        (None, None) => {},
        _ => return Err("cert and key must be provided together".into()),
    }

    Ok(builder)
}
