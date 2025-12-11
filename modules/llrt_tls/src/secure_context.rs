// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! SecureContext implementation for TLS
//!
//! This module provides the `createSecureContext` function which creates
//! a reusable TLS context that can be shared between multiple connections.

use std::sync::Arc;

use llrt_utils::object::ObjectExt;
use rquickjs::{class::Trace, Class, Ctx, Exception, JsLifetime, Object, Result, Value};
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::BuildClientConfigOptions;

/// Parse BuildClientConfigOptions from a JavaScript options object
pub fn options_from_js<'js>(
    ctx: &Ctx<'js>,
    opts: &Object<'js>,
) -> Result<BuildClientConfigOptions> {
    let mut options = BuildClientConfigOptions::default();

    // Handle certificate
    if let Some(cert_value) = opts.get_optional::<_, Value>("cert")? {
        if let Some(s) = cert_value.as_string() {
            options.cert = Some(s.to_string()?.into_bytes());
        } else if let Some(bytes) = get_bytes_from_value(ctx, &cert_value)? {
            options.cert = Some(bytes);
        }
    }

    // Handle private key
    if let Some(key_value) = opts.get_optional::<_, Value>("key")? {
        if let Some(s) = key_value.as_string() {
            options.key = Some(s.to_string()?.into_bytes());
        } else if let Some(bytes) = get_bytes_from_value(ctx, &key_value)? {
            options.key = Some(bytes);
        }
    }

    // Handle CA certificates
    if let Some(ca_value) = opts.get_optional::<_, Value>("ca")? {
        let mut ca_certs = Vec::new();
        if let Some(ca_array) = ca_value.as_array() {
            for item in ca_array.iter::<Value>() {
                let item = item?;
                if let Some(s) = item.as_string() {
                    ca_certs.push(s.to_string()?.into_bytes());
                } else if let Some(bytes) = get_bytes_from_value(ctx, &item)? {
                    ca_certs.push(bytes);
                }
            }
        } else if let Some(s) = ca_value.as_string() {
            ca_certs.push(s.to_string()?.into_bytes());
        } else if let Some(bytes) = get_bytes_from_value(ctx, &ca_value)? {
            ca_certs.push(bytes);
        }
        if !ca_certs.is_empty() {
            options.ca = Some(ca_certs);
        }
    }

    // Handle rejectUnauthorized
    if let Some(reject_unauthorized) = opts.get_optional::<_, bool>("rejectUnauthorized")? {
        options.reject_unauthorized = reject_unauthorized;
    }

    if let Some(ciphers) = opts.get_optional::<_, String>("ciphers")? {
        options.ciphers = Some(ciphers);
    }

    if let Some(min_version) = opts.get_optional::<_, String>("minVersion")? {
        options.min_version = Some(min_version);
    }

    if let Some(max_version) = opts.get_optional::<_, String>("maxVersion")? {
        options.max_version = Some(max_version);
    }

    Ok(options)
}

fn get_bytes_from_value<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> Result<Option<Vec<u8>>> {
    if let Ok(bytes) = llrt_utils::bytes::ObjectBytes::from(ctx, value) {
        if let Ok(vec) = TryInto::<Vec<u8>>::try_into(bytes) {
            return Ok(Some(vec));
        }
    }
    Ok(None)
}

/// A secure context that can be used to configure TLS connections
#[rquickjs::class]
pub struct SecureContext {
    /// The client configuration (for outbound connections)
    pub(crate) client_config: Option<Arc<ClientConfig>>,
    /// The server configuration (for inbound connections)
    pub(crate) server_config: Option<Arc<ServerConfig>>,
}

impl<'js> Trace<'js> for SecureContext {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {
        // No JS values to trace
    }
}

unsafe impl<'js> JsLifetime<'js> for SecureContext {
    type Changed<'to> = SecureContext;
}

impl Default for SecureContext {
    fn default() -> Self {
        Self::new()
    }
}

#[rquickjs::methods]
impl SecureContext {
    #[qjs(constructor)]
    pub fn new() -> Self {
        Self {
            client_config: None,
            server_config: None,
        }
    }
}

impl SecureContext {
    /// Create a new SecureContext from options
    pub fn from_options(ctx: &Ctx<'_>, options: BuildClientConfigOptions) -> Result<Self> {
        let mut secure_context = Self::new();

        // Build client config if we have CA certs or need default trust
        let provider = Arc::new(rustls::crypto::ring::default_provider());

        // Build root certificate store
        let mut root_store = RootCertStore::empty();

        if let Some(ca_certs) = &options.ca {
            for ca in ca_certs {
                let cert = CertificateDer::from_pem_slice(ca).map_err(|e| {
                    Exception::throw_message(ctx, &format!("Invalid CA certificate: {}", e))
                })?;
                root_store.add(cert).map_err(|e| {
                    Exception::throw_message(ctx, &format!("Failed to add CA certificate: {}", e))
                })?;
            }
        } else {
            // Use default root certificates
            for cert in TLS_SERVER_ROOTS.iter().cloned() {
                root_store.roots.push(cert);
            }
        }

        // Build client config
        let client_builder = ClientConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .map_err(|e| {
                Exception::throw_message(ctx, &format!("Failed to set protocol versions: {}", e))
            })?
            .with_root_certificates(root_store);

        let mut client_config =
            if let (Some(cert_pem), Some(key_pem)) = (&options.cert, &options.key) {
                // Client certificate authentication
                let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(cert_pem)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| {
                        Exception::throw_message(ctx, &format!("Invalid certificate: {}", e))
                    })?;

                let key = PrivateKeyDer::from_pem_slice(key_pem).map_err(|e| {
                    Exception::throw_message(ctx, &format!("Invalid private key: {}", e))
                })?;

                client_builder
                    .with_client_auth_cert(certs, key)
                    .map_err(|e| {
                        Exception::throw_message(ctx, &format!("Failed to set client auth: {}", e))
                    })?
            } else {
                client_builder.with_no_client_auth()
            };

        // Set key log if provided
        if let Some(key_log) = options.key_log {
            client_config.key_log = key_log;
        }

        secure_context.client_config = Some(Arc::new(client_config));

        // Build server config if we have cert and key
        if let (Some(cert_pem), Some(key_pem)) = (&options.cert, &options.key) {
            let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(cert_pem)
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| {
                    Exception::throw_message(ctx, &format!("Invalid certificate: {}", e))
                })?;

            let key = PrivateKeyDer::from_pem_slice(key_pem).map_err(|e| {
                Exception::throw_message(ctx, &format!("Invalid private key: {}", e))
            })?;

            let server_config = ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map_err(|e| {
                    Exception::throw_message(ctx, &format!("Failed to build server config: {}", e))
                })?;

            secure_context.server_config = Some(Arc::new(server_config));
        }

        Ok(secure_context)
    }
}

/// Create a secure context from options
pub fn create_secure_context<'js>(
    ctx: Ctx<'js>,
    options: Option<Object<'js>>,
) -> Result<Class<'js, SecureContext>> {
    let opts = if let Some(opts) = options {
        options_from_js(&ctx, &opts)?
    } else {
        BuildClientConfigOptions::default()
    };

    let secure_context = SecureContext::from_options(&ctx, opts)?;
    Class::instance(ctx, secure_context)
}
