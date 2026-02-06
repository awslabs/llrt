// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, OnceLock};

use once_cell::sync::Lazy;
use rustls::{
    pki_types::{pem::PemObject, CertificateDer},
    ClientConfig, RootCertStore, SupportedProtocolVersion,
};
#[cfg(feature = "webpki-roots")]
use webpki_roots::TLS_SERVER_ROOTS;

use crate::no_verification::NoCertificateVerification;

// Select the crypto provider based on feature flags
#[cfg(feature = "tls-rust")]
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls_rustcrypto::provider())
}

#[cfg(feature = "tls-ring")]
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

#[cfg(feature = "tls-aws-lc")]
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::aws_lc_rs::default_provider())
}

#[cfg(feature = "tls-graviola")]
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls_graviola::default_provider())
}

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

pub static TLS_CONFIG: Lazy<Result<ClientConfig, Box<dyn std::error::Error + Send + Sync>>> =
    Lazy::new(|| {
        build_client_config(BuildClientConfigOptions {
            reject_unauthorized: true,
            ca: None,
        })
    });

pub struct BuildClientConfigOptions {
    pub reject_unauthorized: bool,
    pub ca: Option<Vec<Vec<u8>>>,
}

pub fn build_client_config(
    options: BuildClientConfigOptions,
) -> Result<ClientConfig, Box<dyn std::error::Error + Send + Sync>> {
    let provider = get_crypto_provider();
    let builder = ClientConfig::builder_with_provider(provider.clone());

    // TLS versions
    let builder = match get_tls_versions() {
        Some(versions) => builder.with_protocol_versions(&versions),
        None => builder.with_safe_default_protocol_versions(),
    }?;

    // Certificate verification
    let builder = if !options.reject_unauthorized {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new(provider)))
    } else if let Some(ca) = options.ca {
        let mut root_certificates = RootCertStore::empty();

        for cert in ca {
            root_certificates.add(CertificateDer::from_pem_slice(&cert)?)?;
        }
        builder.with_root_certificates(root_certificates)
    } else {
        let mut root_certificates = RootCertStore::empty();

        #[cfg(feature = "webpki-roots")]
        {
            for cert in TLS_SERVER_ROOTS.iter().cloned() {
                root_certificates.roots.push(cert)
            }
        }
        #[cfg(feature = "native-roots")]
        {
            let load_results = rustls_native_certs::load_native_certs();
            for cert in load_results.certs {
                // Continue on parsing errors, as native stores often include ancient or syntactically
                // invalid certificates, like root certificates without any X509 extensions.
                // Inspiration: https://github.com/rustls/rustls/blob/633bf4ba9d9521a95f68766d04c22e2b01e68318/rustls/src/anchors.rs#L105-L112
                if let Err(err) = root_certificates.add(cert) {
                    tracing::debug!("rustls failed to parse DER certificate: {err:?}");
                }
            }
        }

        if let Some(extra_ca_certs) = get_extra_ca_certs() {
            root_certificates.add_parsable_certificates(extra_ca_certs);
        }

        builder.with_root_certificates(root_certificates)
    };

    Ok(builder.with_no_client_auth())
}
