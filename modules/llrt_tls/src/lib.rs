// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// Ensure only one TLS backend is selected
#[cfg(all(feature = "tls-ring", feature = "tls-aws-lc"))]
compile_error!("Features `tls-ring` and `tls-aws-lc` are mutually exclusive");

#[cfg(all(feature = "tls-ring", feature = "tls-graviola"))]
compile_error!("Features `tls-ring` and `tls-graviola` are mutually exclusive");

#[cfg(all(feature = "tls-ring", feature = "tls-openssl"))]
compile_error!("Features `tls-ring` and `tls-openssl` are mutually exclusive");

#[cfg(all(feature = "tls-aws-lc", feature = "tls-graviola"))]
compile_error!("Features `tls-aws-lc` and `tls-graviola` are mutually exclusive");

#[cfg(all(feature = "tls-aws-lc", feature = "tls-openssl"))]
compile_error!("Features `tls-aws-lc` and `tls-openssl` are mutually exclusive");

#[cfg(all(feature = "tls-graviola", feature = "tls-openssl"))]
compile_error!("Features `tls-graviola` and `tls-openssl` are mutually exclusive");

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
mod rustls_config;

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
pub use rustls_config::*;

#[cfg(any(feature = "tls-ring", feature = "tls-aws-lc", feature = "tls-graviola"))]
mod no_verification;

#[cfg(feature = "tls-openssl")]
mod openssl_config;

#[cfg(feature = "tls-openssl")]
pub use openssl_config::*;

#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-graviola",
    feature = "tls-openssl"
))]
mod client_auth {
    pub const ENCRYPTED_KEY: &str = "Invalid key: encrypted keys are not supported";
    pub const KEY_MISMATCH: &str = "Invalid key: does not match cert";

    /// Whether a PEM private key is encrypted: PKCS#8 `ENCRYPTED PRIVATE KEY`,
    /// or a legacy OpenSSL key with a `Proc-Type: 4,ENCRYPTED` header.
    pub fn is_encrypted_pem_key(pem: &[u8]) -> bool {
        let contains = |needle: &[u8]| pem.windows(needle.len()).any(|w| w == needle);
        contains(b"-----BEGIN ENCRYPTED PRIVATE KEY-----") || contains(b"Proc-Type: 4,ENCRYPTED")
    }
}

// Once we are ready to add the node TLS module, it should be here.
// Right now this module is supporting the https/fetch modules.
