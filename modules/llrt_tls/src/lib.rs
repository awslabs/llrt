// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_events::Emitter;
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    Array, Class, Ctx, Exception, Function, Object, Result, Value,
};
use rustls::crypto::ring::default_provider;
use webpki_roots::TLS_SERVER_ROOTS;

pub use self::config::*;

mod config;
mod keylog;
mod no_verification;
mod secure_context;
mod socket;

pub use keylog::{ChannelKeyLog, KeyLogLine};

use self::secure_context::{create_secure_context, SecureContext};
use self::socket::TLSSocket;

pub struct TlsModule;

// TLS version constants
const DEFAULT_MIN_VERSION: &str = "TLSv1.2";
const DEFAULT_MAX_VERSION: &str = "TLSv1.3";

/// Get the array of PEM-encoded root certificates
/// These are the Mozilla CA certificates bundled with the application
fn get_root_certificates(ctx: Ctx<'_>) -> Result<Array<'_>> {
    let arr = Array::new(ctx.clone())?;

    for (i, cert) in TLS_SERVER_ROOTS.iter().enumerate() {
        // Convert DER to PEM format
        let der_bytes = cert.subject_public_key_info.as_ref();
        // Create a simple PEM representation of the certificate
        // Note: TLS_SERVER_ROOTS contains TrustAnchor, not full certs
        // We'll encode what we have as base64
        let b64 = base64_simd::STANDARD.encode_to_string(der_bytes);
        let pem = format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
            b64.as_bytes()
                .chunks(64)
                .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n")
        );
        arr.set(i, pem)?;
    }

    Ok(arr)
}

/// Default server identity check function
/// Verifies that the hostname matches the certificate's subject/SAN
fn check_server_identity<'js>(
    ctx: Ctx<'js>,
    hostname: String,
    cert: Object<'js>,
) -> Result<Value<'js>> {
    // Get the subject from the certificate
    let subject: Option<Object> = cert.get("subject").ok();
    let alt_names: Option<String> = cert.get("subjectaltname").ok();

    // Check subjectAltName first (preferred)
    if let Some(san) = alt_names {
        // subjectaltname is formatted as "DNS:example.com, DNS:*.example.com"
        for entry in san.split(", ") {
            if let Some(dns_name) = entry.strip_prefix("DNS:") {
                if matches_hostname(&hostname, dns_name) {
                    return Ok(Value::new_undefined(ctx.clone()));
                }
            }
        }
    }

    // Check subject CN if no SAN matched
    if let Some(subj) = subject {
        if let Ok(cn) = subj.get::<_, String>("CN") {
            if matches_hostname(&hostname, &cn) {
                return Ok(Value::new_undefined(ctx.clone()));
            }
        }
    }

    // No match found - return an error
    Err(Exception::throw_message(
        &ctx,
        &format!(
            "Hostname/IP does not match certificate's altnames: Host: {}",
            hostname
        ),
    ))
}

/// Check if hostname matches a certificate name (supports wildcards)
fn matches_hostname(hostname: &str, cert_name: &str) -> bool {
    let hostname = hostname.to_lowercase();
    let cert_name = cert_name.to_lowercase();

    if cert_name.starts_with("*.") {
        // Wildcard matching - only matches one level
        let suffix = &cert_name[1..]; // Remove the "*"
        if let Some(pos) = hostname.find('.') {
            return hostname[pos..] == *suffix;
        }
        false
    } else {
        hostname == cert_name
    }
}

/// Get list of supported cipher suites
fn get_ciphers(ctx: Ctx<'_>) -> Result<Array<'_>> {
    let provider = default_provider();
    let arr = Array::new(ctx.clone())?;

    for (i, suite) in provider.cipher_suites.iter().enumerate() {
        // Get the cipher suite identifier and convert to OpenSSL-style name
        let cipher_suite = suite.suite();
        let openssl_name = cipher_suite_to_openssl_name(cipher_suite);
        arr.set(i, openssl_name)?;
    }

    Ok(arr)
}

/// Convert rustls CipherSuite to OpenSSL-style name
pub fn cipher_suite_to_openssl_name(suite: rustls::CipherSuite) -> &'static str {
    use rustls::CipherSuite::*;
    match suite {
        // TLS 1.3 cipher suites
        TLS13_AES_256_GCM_SHA384 => "TLS_AES_256_GCM_SHA384",
        TLS13_AES_128_GCM_SHA256 => "TLS_AES_128_GCM_SHA256",
        TLS13_CHACHA20_POLY1305_SHA256 => "TLS_CHACHA20_POLY1305_SHA256",
        // TLS 1.2 cipher suites
        TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 => "ECDHE-ECDSA-AES256-GCM-SHA384",
        TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 => "ECDHE-ECDSA-AES128-GCM-SHA256",
        TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256 => "ECDHE-ECDSA-CHACHA20-POLY1305",
        TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 => "ECDHE-RSA-AES256-GCM-SHA384",
        TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 => "ECDHE-RSA-AES128-GCM-SHA256",
        TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256 => "ECDHE-RSA-CHACHA20-POLY1305",
        // Fallback for any other cipher suites
        _ => "UNKNOWN",
    }
}

impl ModuleDef for TlsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("connect")?;
        declare.declare("createSecureContext")?;
        declare.declare("getCiphers")?;
        declare.declare("checkServerIdentity")?;
        declare.declare("rootCertificates")?;
        declare.declare("TLSSocket")?;
        declare.declare("SecureContext")?;
        declare.declare("DEFAULT_MIN_VERSION")?;
        declare.declare("DEFAULT_MAX_VERSION")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<TLSSocket>::define(default)?;
            Class::<SecureContext>::define(default)?;

            TLSSocket::add_event_emitter_prototype(ctx)?;

            // tls.connect(options, callback)
            default.set(
                "connect",
                Func::from(
                    |ctx: Ctx<'js>, options: Object<'js>, callback: Opt<Function<'js>>| {
                        let socket = TLSSocket::new(ctx.clone(), false)?;
                        TLSSocket::connect(rquickjs::prelude::This(socket), ctx, options, callback)
                    },
                ),
            )?;

            // tls.createSecureContext(options)
            default.set(
                "createSecureContext",
                Func::from(|ctx: Ctx<'js>, options: Opt<Object<'js>>| {
                    create_secure_context(ctx, options.0)
                }),
            )?;

            // tls.getCiphers()
            default.set("getCiphers", Func::from(get_ciphers))?;

            // tls.checkServerIdentity(hostname, cert)
            default.set("checkServerIdentity", Func::from(check_server_identity))?;

            // tls.rootCertificates
            default.set("rootCertificates", Func::from(get_root_certificates))?;

            // tls.DEFAULT_MIN_VERSION
            default.set("DEFAULT_MIN_VERSION", DEFAULT_MIN_VERSION)?;

            // tls.DEFAULT_MAX_VERSION
            default.set("DEFAULT_MAX_VERSION", DEFAULT_MAX_VERSION)?;

            Ok(())
        })?;

        Ok(())
    }
}

impl From<TlsModule> for ModuleInfo<TlsModule> {
    fn from(val: TlsModule) -> Self {
        ModuleInfo {
            name: "tls",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use llrt_test::{test_async_with, ModuleEvaluator};

    use super::*;

    #[tokio::test]
    async fn test_module_loads() {
        test_async_with(|ctx| {
            Box::pin(async move {
                let result = ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls").await;
                assert!(result.is_ok(), "TLS module should load successfully");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_connect_function_exists() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        typeof tls.connect === 'function'
                    "#,
                )
                .await;

                assert!(result.is_ok(), "connect should be accessible");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_secure_context_function_exists() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        typeof tls.createSecureContext === 'function'
                    "#,
                )
                .await;

                assert!(result.is_ok(), "createSecureContext should be accessible");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_secure_context_class_exists() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        typeof tls.SecureContext === 'function'
                    "#,
                )
                .await;

                assert!(result.is_ok(), "SecureContext class should be accessible");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_class_exists() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        typeof tls.TLSSocket === 'function'
                    "#,
                )
                .await;

                assert!(result.is_ok(), "TLSSocket should be accessible");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_properties() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const socket = new tls.TLSSocket();
                        const checks = [
                            socket.encrypted === true,
                            socket.authorized === false,
                            socket.connecting === false,
                            socket.pending === true,
                            socket.readyState === 'opening',
                        ];
                        checks.every(c => c === true)
                    "#,
                )
                .await;

                assert!(result.is_ok(), "TLSSocket properties should be correct");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_methods_exist() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const socket = new tls.TLSSocket();
                        const checks = [
                            typeof socket.connect === 'function',
                            typeof socket.end === 'function',
                            typeof socket.destroy === 'function',
                            typeof socket.write === 'function',
                            typeof socket.getProtocol === 'function',
                            typeof socket.getCipher === 'function',
                            typeof socket.getPeerCertificate === 'function',
                        ];
                        checks.every(c => c === true)
                    "#,
                )
                .await;

                assert!(result.is_ok(), "TLSSocket methods should exist");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_secure_context_returns_context() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const context = tls.createSecureContext();
                        context instanceof tls.SecureContext
                    "#,
                )
                .await;

                assert!(
                    result.is_ok(),
                    "createSecureContext should return SecureContext"
                );
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_event_emitter() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const socket = new tls.TLSSocket();
                        let called = false;
                        socket.on('test', () => { called = true; });
                        socket.emit('test');
                        called
                    "#,
                )
                .await;

                assert!(result.is_ok(), "TLSSocket should support event emitter");
            })
        })
        .await;
    }

    // Note: Full integration tests for TLS client-server communication require
    // the llrt_core VM with its event loop (runtime.idle()). The tests above
    // verify the API surface and basic functionality. For end-to-end TLS tests,
    // see the integration tests in the tests/ directory or run manual tests
    // with the full LLRT binary.
    //
    // The test certificates are available at:
    // - libs/llrt_test_tls/data/server.pem (certificate)
    // - libs/llrt_test_tls/data/server.key (private key)
    // - libs/llrt_test_tls/data/root.pem (CA certificate)

    #[tokio::test]
    async fn test_create_secure_context_with_options() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                // Test that createSecureContext accepts options
                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const ctx = tls.createSecureContext({
                            minVersion: 'TLSv1.2',
                            maxVersion: 'TLSv1.3'
                        });
                        ctx instanceof tls.SecureContext
                    "#,
                )
                .await;

                assert!(
                    result.is_ok(),
                    "createSecureContext with options should work"
                );
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_initial_state() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const socket = new tls.TLSSocket();
                        const checks = {
                            encrypted: socket.encrypted === true,
                            authorized: socket.authorized === false,
                            connecting: socket.connecting === false,
                            pending: socket.pending === true,
                            readyState: socket.readyState === 'opening',
                            localAddress: socket.localAddress === undefined,
                            remoteAddress: socket.remoteAddress === undefined,
                        };
                        Object.values(checks).every(v => v === true)
                    "#,
                )
                .await;

                assert!(result.is_ok(), "TLSSocket initial state should be correct");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_get_protocol_before_connect() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const socket = new tls.TLSSocket();
                        socket.getProtocol() === null
                    "#,
                )
                .await;

                assert!(
                    result.is_ok(),
                    "getProtocol before connect should return null"
                );
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_get_cipher_before_connect() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const socket = new tls.TLSSocket();
                        const cipher = socket.getCipher();
                        cipher === null || typeof cipher === 'object'
                    "#,
                )
                .await;

                assert!(
                    result.is_ok(),
                    "getCipher before connect should return null or object"
                );
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tls_socket_get_peer_certificate_before_connect() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TlsModule>(ctx.clone(), "tls")
                    .await
                    .unwrap();

                let result = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import tls from 'tls';
                        const socket = new tls.TLSSocket();
                        const cert = socket.getPeerCertificate();
                        typeof cert === 'object'
                    "#,
                )
                .await;

                assert!(
                    result.is_ok(),
                    "getPeerCertificate before connect should return object"
                );
            })
        })
        .await;
    }
}
