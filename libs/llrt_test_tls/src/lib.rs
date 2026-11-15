// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// FIXME this library is only needed until TLS is natively supported in wiremock.
// See https://github.com/LukeMathWalker/wiremock-rs/issues/58

#[cfg(all(feature = "tls-ring", feature = "tls-aws-lc"))]
compile_error!("Features 'tls-ring' and 'tls-aws-lc' are mutually exclusive");

#[cfg(all(feature = "tls-ring", feature = "tls-graviola"))]
compile_error!("Features 'tls-ring' and 'tls-graviola' are mutually exclusive");

#[cfg(all(feature = "tls-aws-lc", feature = "tls-graviola"))]
compile_error!("Features 'tls-aws-lc' and 'tls-graviola' are mutually exclusive");

use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::TcpListener;

pub use self::config::FileType;
use self::config::MockServerCerts;

mod api;
mod config;
mod server;

/// TLS handshake errors seen by the server, as `Display` strings.
type HandshakeErrors = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

pub struct MockServer {
    addr: SocketAddr,
    ca: String,
    handshake_errors: HandshakeErrors,
    shutdown_tx: tokio::sync::watch::Sender<()>,
}

impl MockServer {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::start_inner(false).await
    }

    /// Starts a server that requires a client certificate signed by
    /// [`FileType::ClientCa`] (mutual TLS), such as [`FileType::ClientCert`].
    pub async fn start_with_client_auth() -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    {
        Self::start_inner(true).await
    }

    async fn start_inner(
        client_auth: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

        // Load the certificates for the mock server.
        let certs = MockServerCerts::load_default().await?;
        let ca = String::from_utf8(tokio::fs::read(FileType::RootCert.default_path()).await?)?;

        // Bind to a random port on localhost.
        let incoming = TcpListener::bind(&SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let addr = incoming.local_addr()?;

        let handshake_errors = HandshakeErrors::default();
        let server_handshake_errors = handshake_errors.clone();

        tokio::spawn(async move {
            if let Err(e) = server::run(
                incoming,
                certs,
                client_auth,
                server_handshake_errors,
                shutdown_rx,
            )
            .await
            {
                println!("failed to run mock server: {e:#}");
            }
        });

        Ok(Self {
            addr,
            ca,
            handshake_errors,
            shutdown_tx,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    pub fn ca(&self) -> &str {
        &self.ca
    }

    /// TLS handshake errors the server has seen so far, such as a client that
    /// sent no certificate to a server started with [`Self::start_with_client_auth`].
    pub fn handshake_errors(&self) -> Vec<String> {
        self.handshake_errors.lock().unwrap().clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(());
    }
}
