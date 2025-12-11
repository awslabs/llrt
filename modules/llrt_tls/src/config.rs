// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, OnceLock};

use once_cell::sync::Lazy;
use rustls::{
    crypto::ring,
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
    ClientConfig, RootCertStore, SupportedProtocolVersion,
};
#[cfg(feature = "webpki-roots")]
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

pub static TLS_CONFIG: Lazy<Result<ClientConfig, Box<dyn std::error::Error + Send + Sync>>> =
    Lazy::new(|| build_client_config(BuildClientConfigOptions::default()));

/// Unified TLS client configuration options.
/// Used by SecureContext, tls.connect(), and HTTP agent.
pub struct BuildClientConfigOptions {
    /// Whether to reject unauthorized certificates (default: true)
    pub reject_unauthorized: bool,
    /// Custom CA certificates in PEM format
    pub ca: Option<Vec<Vec<u8>>>,
    /// Client certificate in PEM format for mTLS
    pub cert: Option<Vec<u8>>,
    /// Client private key in PEM format for mTLS
    pub key: Option<Vec<u8>>,
    /// Key log callback for debugging TLS connections
    pub key_log: Option<Arc<dyn rustls::KeyLog>>,
    /// Cipher suites (OpenSSL format) - currently unused, reserved for future
    pub ciphers: Option<String>,
    /// Minimum TLS version (e.g., "TLSv1.2") - currently unused, reserved for future
    pub min_version: Option<String>,
    /// Maximum TLS version (e.g., "TLSv1.3") - currently unused, reserved for future
    pub max_version: Option<String>,
}

impl Default for BuildClientConfigOptions {
    fn default() -> Self {
        Self {
            reject_unauthorized: true, // Secure by default
            ca: None,
            cert: None,
            key: None,
            key_log: None,
            ciphers: None,
            min_version: None,
            max_version: None,
        }
    }
}

pub fn build_client_config(
    options: BuildClientConfigOptions,
) -> Result<ClientConfig, Box<dyn std::error::Error + Send + Sync>> {
    let provider = Arc::new(ring::default_provider());
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
    } else if let Some(ca) = &options.ca {
        let mut root_certificates = RootCertStore::empty();

        for cert in ca {
            root_certificates.add(CertificateDer::from_pem_slice(cert)?)?;
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

    // Client authentication (mTLS)
    let mut config = if let (Some(cert_pem), Some(key_pem)) = (&options.cert, &options.key) {
        // Parse client certificate chain
        let certs: Vec<CertificateDer<'static>> =
            CertificateDer::pem_slice_iter(cert_pem).collect::<std::result::Result<Vec<_>, _>>()?;

        // Parse private key
        let key = PrivateKeyDer::from_pem_slice(key_pem)?;

        builder.with_client_auth_cert(certs, key)?
    } else {
        builder.with_no_client_auth()
    };

    // Set key log if provided
    if let Some(key_log) = options.key_log {
        config.key_log = key_log;
    }

    Ok(config)
}
