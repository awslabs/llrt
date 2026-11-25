// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Password-encrypted private keys: PEM, PKCS#8 and PKCS#5 PBES2.

mod pbes2;
pub mod pem;
pub mod pkcs8;

/// Why decryption failed, with OpenSSL's distinctions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The `EncryptedPrivateKeyInfo`, or the key it decrypts to, isn't valid DER.
    InvalidKey,
    UnknownScheme,
    /// A scheme, cipher or PRF that OpenSSL's default provider doesn't have.
    Unsupported,
    /// An algorithm OpenSSL supports and this crate doesn't, by OpenSSL's name.
    Unimplemented(&'static str),
    InvalidParameters,
    UnsupportedKdf,
    UnsupportedCipher,
    InvalidCipherParameters,
    InvalidIv,
    UnsupportedKeyLength,
    UnsupportedPrf,
    UnsupportedSaltType,
    ZeroIterations,
    IterationCountOutOfRange,
    WrongFinalBlockLength,
    BadDecrypt,
}

#[cfg(test)]
mod fixtures {
    use std::path::Path;

    pub fn read(name: &str) -> String {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../llrt_test_tls/data");
        std::fs::read_to_string(data.join(name)).unwrap()
    }

    pub fn der(pem: &str) -> Vec<u8> {
        crate::pem::first_private_key_block(pem.as_bytes())
            .unwrap()
            .decode()
            .unwrap()
    }
}
