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

// Enforce that exactly one TLS crypto-provider feature is active at compile time.
// Enabling two simultaneously (e.g. tls-ring + tls-aws-lc) would make
// get_crypto_provider() silently pick whichever cfg arm comes first.
#[cfg(all(feature = "tls-ring", feature = "tls-aws-lc"))]
compile_error!("Features `tls-ring` and `tls-aws-lc` are mutually exclusive — enable only one.");
#[cfg(all(feature = "tls-ring", feature = "tls-graviola"))]
compile_error!("Features `tls-ring` and `tls-graviola` are mutually exclusive — enable only one.");
#[cfg(all(feature = "tls-aws-lc", feature = "tls-graviola"))]
compile_error!(
    "Features `tls-aws-lc` and `tls-graviola` are mutually exclusive — enable only one."
);

// rustls-platform-verifier requires a process-wide default CryptoProvider to be
// installed before any TLS handshake. We install it once via a OnceLock.
#[cfg(feature = "platform-verifier")]
static PROVIDER_INSTALLED: OnceLock<()> = OnceLock::new();

// Select the crypto provider based on feature flags. Only one tls-* feature
// should be active at a time, enforced by the workspace default features.
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    #[cfg(feature = "tls-ring")]
    {
        return Arc::new(rustls::crypto::ring::default_provider());
    }
    #[cfg(feature = "tls-aws-lc")]
    {
        return Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    }
    #[cfg(feature = "tls-graviola")]
    {
        return Arc::new(rustls_graviola::default_provider());
    }
    #[allow(unreachable_code)]
    panic!("No TLS crypto provider feature enabled (tls-ring / tls-aws-lc / tls-graviola)");
}

// Call early in process startup when platform-verifier is active.
// `install_default()` returns Err if a provider is already installed; that is
// harmless and expected when multiple TLS configs are built in the same process.
// Any other error (e.g. a broken provider) is logged as a warning so it is not
// silently swallowed while PROVIDER_INSTALLED is still marked as initialised.
#[cfg(feature = "platform-verifier")]
fn install_default_provider_once() {
    PROVIDER_INSTALLED.get_or_init(|| {
        #[cfg(feature = "tls-ring")]
        if let Err(existing) = rustls::crypto::ring::default_provider().install_default() {
            tracing::debug!(
                "rustls default provider already installed ({:?}); using existing",
                existing.name()
            );
        }
        #[cfg(feature = "tls-aws-lc")]
        if let Err(existing) = rustls::crypto::aws_lc_rs::default_provider().install_default() {
            tracing::debug!(
                "rustls default provider already installed ({:?}); using existing",
                existing.name()
            );
        }
        #[cfg(feature = "tls-graviola")]
        if let Err(existing) = rustls_graviola::default_provider().install_default() {
            tracing::debug!(
                "rustls default provider already installed ({:?}); using existing",
                existing.name()
            );
        }
    });
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
    #[cfg(feature = "platform-verifier")]
    install_default_provider_once();

    let provider = get_crypto_provider();

    let builder = ClientConfig::builder_with_provider(Arc::clone(&provider));

    // TLS versions
    let builder = match get_tls_versions() {
        Some(versions) => builder.with_protocol_versions(&versions),
        None => builder.with_safe_default_protocol_versions(),
    }?;

    // Certificate verification
    let builder = if !options.reject_unauthorized {
        // SAFETY: dangerous() bypasses certificate verification. This is intentional
        // and explicitly requested by the caller via `rejectUnauthorized: false`.
        // The NoCertificateVerification verifier is only used when the JS caller
        // opts out of verification — it must never be used as a default.
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
        #[cfg(feature = "platform-verifier")]
        // Only use the platform verifier when no extra CA certs are registered.
        // rustls_platform_verifier::Verifier delegates entirely to the OS trust
        // store and has no API for appending additional anchors. If extra certs
        // are present (e.g. a private PKI) we fall through to the standard
        // RootCertStore path below so those certs are honoured.
        if get_extra_ca_certs().is_none() {
            // SAFETY: dangerous() is required by rustls-platform-verifier because its
            // Verifier implements ServerCertVerifier using the OS trust store rather than
            // a bundled root set. rustls exposes this path through dangerous() to signal
            // that the caller is responsible for ensuring the verifier is trustworthy.
            // rustls_platform_verifier::Verifier delegates to SecTrustEvaluateWithError
            // (macOS/iOS), NSS (Linux), or SChannel (Windows) — it is not bypassing
            // verification, it is replacing it with the platform-native mechanism.
            return Ok(builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(
                    rustls_platform_verifier::Verifier::new(),
                ))
                .with_no_client_auth());
        }

        {
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
        }
    };

    Ok(builder.with_no_client_auth())
}
