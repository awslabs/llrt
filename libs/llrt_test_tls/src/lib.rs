// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// FIXME this library is only needed until TLS is natively supported in wiremock.
// See https://github.com/LukeMathWalker/wiremock-rs/issues/58

use std::net::{Ipv4Addr, SocketAddr};

use tokio::net::TcpListener;

use self::config::{FileType, MockServerCerts};

mod api;
mod config;
mod server;

pub struct MockServer {
    addr: SocketAddr,
    ca: String,
    shutdown_tx: tokio::sync::watch::Sender<()>,
}

impl MockServer {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

        // Load the certificates for the mock server.
        let certs = MockServerCerts::load_default().await?;
        let ca = String::from_utf8(tokio::fs::read(FileType::RootCert.default_path()).await?)?;

        // Bind to a random port on localhost.
        let incoming = TcpListener::bind(&SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let addr = incoming.local_addr()?;

        tokio::spawn(async move {
            if let Err(e) = server::run(incoming, certs, shutdown_rx).await {
                println!("failed to run mock server: {e:#}");
            }
        });

        Ok(Self {
            addr,
            ca,
            shutdown_tx,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    pub fn ca(&self) -> &str {
        &self.ca
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(());
    }
}
