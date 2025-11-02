use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use std::path::PathBuf;

pub enum FileType {
    RootCert,
    ServerCert,
    ServerKey,
}

impl FileType {
    pub fn default_path(&self) -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let data_dir = PathBuf::from(manifest_dir).join("data");
        match self {
            Self::RootCert => data_dir.join("root.pem"),
            Self::ServerCert => data_dir.join("server.pem"),
            Self::ServerKey => data_dir.join("server.key"),
        }
    }
}

pub struct MockServerCerts {
    pub root_cert: CertificateDer<'static>,
    pub server_cert: CertificateDer<'static>,
    pub server_key: PrivateKeyDer<'static>,
}

impl MockServerCerts {
    pub async fn load_default() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let certfile = tokio::fs::read(FileType::RootCert.default_path()).await?;
        let root_cert = CertificateDer::from_pem_slice(&certfile)?;

        let certfile = tokio::fs::read(FileType::ServerCert.default_path()).await?;
        let server_cert = CertificateDer::from_pem_slice(&certfile)?;

        let keyfile = tokio::fs::read(FileType::ServerKey.default_path()).await?;
        let server_key = PrivateKeyDer::from_pem_slice(&keyfile)?;

        Ok(Self {
            root_cert,
            server_cert,
            server_key,
        })
    }
}
