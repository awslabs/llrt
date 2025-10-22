// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    io,
    sync::{Arc, OnceLock},
};

use once_cell::sync::Lazy;
use rustls::{
    crypto::ring, pki_types::CertificateDer, ClientConfig, RootCertStore, SupportedProtocolVersion,
};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::no_verification::NoCertificateVerification;

static EXTRA_CA_CERTS: OnceLock<Vec<CertificateDer<'static>>> = OnceLock::new();

pub fn set_extra_ca_certs(certs: Vec<CertificateDer<'static>>) {
    _ = EXTRA_CA_CERTS.set(certs);
}

pub fn get_extra_ca_certs() -> Option<Vec<CertificateDer<'static>>> {
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

pub fn get_tls_versions() -> Option<Vec<&'static SupportedProtocolVersion>> {
    let versions = TLS_VERSIONS.get_or_init(Vec::new).clone();
    if versions.is_empty() {
        None
    } else {
        Some(versions)
    }
}

pub static TLS_CONFIG: Lazy<io::Result<ClientConfig>> = Lazy::new(|| {
    Ok(build_client_config(BuildClientConfigOptions {
        reject_unauthorized: true,
    }))
});

pub struct BuildClientConfigOptions {
    pub reject_unauthorized: bool,
}

pub fn build_client_config(options: BuildClientConfigOptions) -> ClientConfig {
    let provider = Arc::new(ring::default_provider());
    let builder = ClientConfig::builder_with_provider(provider.clone());

    // TLS versions
    let builder = match get_tls_versions() {
        Some(versions) => builder.with_protocol_versions(&versions),
        None => builder.with_safe_default_protocol_versions(),
    }
    .expect("TLS configuration failed");

    // Certificate verification
    let builder = if options.reject_unauthorized {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new(provider)))
    } else {
        let mut root_certificates = RootCertStore::empty();

        for cert in TLS_SERVER_ROOTS.iter().cloned() {
            root_certificates.roots.push(cert)
        }

        if let Some(extra_ca_certs) = get_extra_ca_certs() {
            root_certificates.add_parsable_certificates(extra_ca_certs);
        }

        builder.with_root_certificates(root_certificates)
    };

    builder.with_no_client_auth()
}
