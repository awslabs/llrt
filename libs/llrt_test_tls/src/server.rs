use std::sync::Arc;

use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use rustls::ServerConfig;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::MockServerCerts;

#[cfg(all(
    feature = "tls-ring",
    not(feature = "tls-aws-lc"),
    not(feature = "tls-graviola")
))]
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

#[cfg(all(feature = "tls-aws-lc", not(feature = "tls-graviola")))]
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::aws_lc_rs::default_provider())
}

#[cfg(feature = "tls-graviola")]
fn get_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls_graviola::default_provider())
}

pub(super) async fn run(
    listener: TcpListener,
    certs: MockServerCerts,
    shutdown_rx: tokio::sync::watch::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
