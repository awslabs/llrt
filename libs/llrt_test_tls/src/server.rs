use std::sync::Arc;

use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use tokio::net::TcpListener;

use crate::MockServerCerts;

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

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
pub(super) async fn run(
    listener: TcpListener,
    certs: MockServerCerts,
    shutdown_rx: tokio::sync::watch::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use rustls::ServerConfig;
    use tokio_rustls::TlsAcceptor;

    let cert_chain = vec![certs.server_cert, certs.root_cert];
    let mut server_config = ServerConfig::builder_with_provider(get_crypto_provider())
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(cert_chain, certs.server_key)?;
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    let service = service_fn(crate::api::echo);

    loop {
        let (tcp_stream, _remote_addr) = listener.accept().await?;

        let mut shutdown_signal = shutdown_rx.clone();
        let tls_acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            let tls_stream = match tls_acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => tls_stream,
                Err(err) => {
                    eprintln!("failed to perform tls handshake: {err:#}");
                    return;
                },
            };

            let http_server = Builder::new(TokioExecutor::new());
            let conn = http_server.serve_connection(TokioIo::new(tls_stream), service);
            tokio::pin!(conn);

            loop {
                tokio::select! {
                    _ = conn.as_mut() => break,
                    _ = shutdown_signal.changed() => conn.as_mut().graceful_shutdown(),
                }
            }
        });
    }
}

#[cfg(feature = "tls-openssl")]
pub(super) async fn run(
    listener: TcpListener,
    certs: MockServerCerts,
    shutdown_rx: tokio::sync::watch::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use openssl::ssl::{SslAcceptor, SslMethod};

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;

    // Convert rustls certs to OpenSSL format
    let cert_der = certs.server_cert.as_ref();
    let key_der = match certs.server_key {
        rustls::pki_types::PrivateKeyDer::Pkcs1(ref key) => key.secret_pkcs1_der().to_vec(),
        rustls::pki_types::PrivateKeyDer::Pkcs8(ref key) => key.secret_pkcs8_der().to_vec(),
        rustls::pki_types::PrivateKeyDer::Sec1(ref key) => key.secret_sec1_der().to_vec(),
        _ => return Err("Unsupported key format".into()),
    };

    let cert = openssl::x509::X509::from_der(cert_der)?;
    let pkey = openssl::pkey::PKey::private_key_from_der(&key_der)?;

    builder.set_certificate(&cert)?;
    builder.set_private_key(&pkey)?;

    let root_cert = openssl::x509::X509::from_der(certs.root_cert.as_ref())?;
    builder.add_extra_chain_cert(root_cert)?;

    builder.set_alpn_protos(b"\x02h2\x08http/1.1\x08http/1.0")?;

    let acceptor = builder.build();
    let service = service_fn(crate::api::echo);

    loop {
        let (tcp_stream, _remote_addr) = listener.accept().await?;

        let mut shutdown_signal = shutdown_rx.clone();
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            let ssl = match openssl::ssl::Ssl::new(acceptor.context()) {
                Ok(ssl) => ssl,
                Err(err) => {
                    eprintln!("failed to create ssl: {err:#}");
                    return;
                },
            };

            let tls_stream = match tokio_openssl::SslStream::new(ssl, tcp_stream) {
                Ok(mut stream) => {
                    if let Err(err) = std::pin::Pin::new(&mut stream).accept().await {
                        eprintln!("failed to perform tls handshake: {err:#}");
                        return;
                    }
                    stream
                },
                Err(err) => {
                    eprintln!("failed to create ssl stream: {err:#}");
                    return;
                },
            };

            let http_server = Builder::new(TokioExecutor::new());
            let conn = http_server.serve_connection(TokioIo::new(tls_stream), service);
            tokio::pin!(conn);

            loop {
                tokio::select! {
                    _ = conn.as_mut() => break,
                    _ = shutdown_signal.changed() => conn.as_mut().graceful_shutdown(),
                }
            }
        });
    }
}
