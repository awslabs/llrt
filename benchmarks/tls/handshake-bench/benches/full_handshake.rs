use std::io::Cursor;
use std::sync::Arc;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use llrt_tls::{build_client_config, set_tls_versions, BuildClientConfigOptions};
use rustls::{
    crypto::CryptoProvider,
    pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer, ServerName},
    ClientConfig, ClientConnection, HandshakeKind, ServerConfig, ServerConnection,
};

const CERT_DIR: &str = "../certs";

const CERTIFICATE_IDS: [&str; 7] = [
    "rsa-2048",
    "rsa-3072",
    "rsa-4096",
    "ecdsa-p256",
    "ecdsa-p384",
    "ecdsa-p521",
    "ed25519",
];

#[cfg(feature = "tls-rust")]
fn crypto_provider() -> Arc<CryptoProvider> {
    Arc::new(rustls_rustcrypto::provider())
}

#[cfg(feature = "tls-ring")]
fn crypto_provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

#[cfg(feature = "tls-aws-lc")]
fn crypto_provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::aws_lc_rs::default_provider())
}

#[cfg(feature = "tls-graviola")]
fn crypto_provider() -> Arc<CryptoProvider> {
    Arc::new(rustls_graviola::default_provider())
}

struct HandshakeFixtures {
    client_config: Arc<ClientConfig>,
    server_config: Arc<ServerConfig>,
}

#[derive(Clone, Copy, Debug)]
enum HandshakeKindPattern {
    Full,
    Resumed,
}

impl HandshakeKindPattern {
    fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Resumed => "resumed",
        }
    }
}

impl HandshakeFixtures {
    fn try_new(certificate_id: &str, pattern: HandshakeKindPattern) -> Result<Self, String> {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let cert_dir = manifest_dir.join(CERT_DIR);
        let issuer_dir = cert_dir.join("ca-rsa");
        let issued_dir = cert_dir.join("issued/ca-rsa");

        let ca_pem = std::fs::read(issuer_dir.join("ca.crt"))
            .map_err(|e| format!("reading CA certificate: {e}"))?;
        let server_cert_pem = std::fs::read(issued_dir.join(format!("{certificate_id}.crt")))
            .map_err(|e| format!("reading server certificate: {e}"))?;
        let server_key_pem = std::fs::read(issued_dir.join(format!("{certificate_id}.key")))
            .map_err(|e| format!("reading server key: {e}"))?;

        set_tls_versions(vec![&rustls::version::TLS12]);
        let mut client_config = build_client_config(BuildClientConfigOptions {
            reject_unauthorized: true,
            ca: Some(vec![ca_pem.clone()]),
            cert: None,
            key: None,
        })
        .map_err(|e| format!("building client config: {e}"))?;
        client_config.resumption = match pattern {
            HandshakeKindPattern::Full => rustls::client::Resumption::disabled(),
            HandshakeKindPattern::Resumed => rustls::client::Resumption::in_memory_sessions(256),
        };

        let mut server_certs = CertificateDer::pem_slice_iter(&server_cert_pem)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("parsing server certificate: {e}"))?;
        server_certs.extend(
            CertificateDer::pem_slice_iter(&ca_pem)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("parsing CA certificate: {e}"))?,
        );
        let server_key = PrivateKeyDer::from_pem_slice(&server_key_pem)
            .map_err(|e| format!("parsing server key: {e}"))?;

        let mut server_config = ServerConfig::builder_with_provider(crypto_provider())
            .with_protocol_versions(&[&rustls::version::TLS12])
            .map_err(|e| format!("configuring TLS version: {e}"))?
            .with_no_client_auth()
            .with_single_cert(server_certs, server_key)
            .map_err(|e| format!("building server config: {e}"))?;
        server_config.alpn_protocols.clear();

        let fixtures = Self {
            client_config: Arc::new(client_config),
            server_config: Arc::new(server_config),
        };
        match pattern {
            HandshakeKindPattern::Full => handshake(&fixtures, HandshakeKind::Full)?,
            HandshakeKindPattern::Resumed => {
                handshake(&fixtures, HandshakeKind::Full)?;
                handshake(&fixtures, HandshakeKind::Resumed)?;
            },
        }
        Ok(fixtures)
    }
}

fn transfer_client_to_server(
    client: &mut ClientConnection,
    server: &mut ServerConnection,
) -> Result<(), String> {
    let mut tls = Vec::new();
    client
        .write_tls(&mut tls)
        .map_err(|error| format!("failed to write client TLS data: {error}"))?;
    if !tls.is_empty() {
        let mut input = Cursor::new(tls);
        server
            .read_tls(&mut input)
            .map_err(|error| format!("failed to read client TLS data: {error}"))?;
        server
            .process_new_packets()
            .map_err(|error| format!("failed to process client TLS data: {error}"))?;
    }
    Ok(())
}

fn transfer_server_to_client(
    client: &mut ClientConnection,
    server: &mut ServerConnection,
) -> Result<(), String> {
    let mut tls = Vec::new();
    server
        .write_tls(&mut tls)
        .map_err(|error| format!("failed to write server TLS data: {error}"))?;
    if !tls.is_empty() {
        let mut input = Cursor::new(tls);
        client
            .read_tls(&mut input)
            .map_err(|error| format!("failed to read server TLS data: {error}"))?;
        client
            .process_new_packets()
            .map_err(|error| format!("failed to process server TLS data: {error}"))?;
    }
    Ok(())
}

fn handshake(fixtures: &HandshakeFixtures, expected_kind: HandshakeKind) -> Result<(), String> {
    let server_name = ServerName::try_from("localhost").expect("invalid server name");
    let mut client =
        ClientConnection::new(fixtures.client_config.clone(), server_name).expect("client setup");
    let mut server = ServerConnection::new(fixtures.server_config.clone()).expect("server setup");

    for _ in 0..64 {
        transfer_client_to_server(&mut client, &mut server)?;
        transfer_server_to_client(&mut client, &mut server)?;
        if !client.is_handshaking() && !server.is_handshaking() {
            break;
        }
    }

    if client.is_handshaking() {
        return Err("client handshake did not finish".into());
    }
    if server.is_handshaking() {
        return Err("server handshake did not finish".into());
    }
    if client.handshake_kind() != Some(expected_kind) {
        return Err("client used resumed or incomplete handshake".into());
    }
    if server.handshake_kind() != Some(expected_kind) {
        return Err("server used resumed or incomplete handshake".into());
    }
    if client.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_2) {
        return Err("unexpected negotiated TLS version".into());
    }
    Ok(())
}

fn bench_full_handshake(c: &mut Criterion) {
    let provider_name = tls_handshake_bench::provider_name();
    let mut group = c.benchmark_group(format!("{provider_name}/tls12"));

    for certificate_id in CERTIFICATE_IDS {
        for pattern in [HandshakeKindPattern::Full, HandshakeKindPattern::Resumed] {
            let fixtures = match HandshakeFixtures::try_new(certificate_id, pattern) {
                Ok(fixtures) => fixtures,
                Err(error) => {
                    eprintln!(
                        "TLS_BENCH_SKIP\t{certificate_id}\t{}\t{}",
                        pattern.name(),
                        error.replace(['\t', '\n', '\r'], " ")
                    );
                    continue;
                },
            };

            group.bench_function(BenchmarkId::new(certificate_id, pattern.name()), |b| {
                let expected_kind = match pattern {
                    HandshakeKindPattern::Full => HandshakeKind::Full,
                    HandshakeKindPattern::Resumed => HandshakeKind::Resumed,
                };
                b.iter(|| {
                    handshake(&fixtures, expected_kind).expect("TLS handshake benchmark failed")
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_full_handshake);
criterion_main!(benches);
