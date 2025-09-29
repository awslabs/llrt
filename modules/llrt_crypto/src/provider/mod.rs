// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "crypto-openssl")]
mod openssl;
#[cfg(feature = "crypto-ring")]
mod ring;
#[cfg(feature = "crypto-rust")]
mod rust;

use crate::sha_hash::ShaAlgorithm;
use crate::subtle::EllipticCurve;

pub trait SimpleDigest {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> Vec<u8>;
}

#[derive(Debug, Clone, Copy)]
pub enum AesMode {
    Ctr { counter_length: u32 },
    Cbc,
    Gcm { tag_length: u8 },
}

pub trait CryptoProvider {
    type Digest: SimpleDigest;
    type Hmac: HmacProvider;

    // Digest operations
    fn digest(&self, algorithm: ShaAlgorithm) -> Self::Digest;

    // HMAC operations
    fn hmac(&self, algorithm: ShaAlgorithm, key: &[u8]) -> Self::Hmac;

    // ECDSA operations
    fn ecdsa_sign(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        digest: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;
    fn ecdsa_verify(
        &self,
        curve: EllipticCurve,
        public_key_sec1: &[u8],
        signature: &[u8],
        digest: &[u8],
    ) -> Result<bool, CryptoError>;

    // EdDSA operations
    fn ed25519_sign(&self, private_key_der: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn ed25519_verify(
        &self,
        public_key_bytes: &[u8],
        signature: &[u8],
        data: &[u8],
    ) -> Result<bool, CryptoError>;

    // RSA operations
    fn rsa_pss_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError>;
    fn rsa_pss_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError>;
    fn rsa_pkcs1v15_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError>;
    fn rsa_pkcs1v15_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError>;
    fn rsa_oaep_encrypt(
        &self,
        public_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError>;
    fn rsa_oaep_decrypt(
        &self,
        private_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError>;

    // ECDH operations
    fn ecdh_derive_bits(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        public_key_sec1: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;

    // X25519 operations
    fn x25519_derive_bits(
        &self,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;

    // AES operations
    fn aes_encrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError>;
    fn aes_decrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError>;

    // AES-KW operations
    fn aes_kw_wrap(&self, kek: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn aes_kw_unwrap(&self, kek: &[u8], wrapped_key: &[u8]) -> Result<Vec<u8>, CryptoError>;

    // KDF operations
    fn hkdf_derive_key(
        &self,
        key: &[u8],
        salt: &[u8],
        info: &[u8],
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError>;
    fn pbkdf2_derive_key(
        &self,
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError>;

    fn generate_aes_key(&self, length_bits: u16) -> Result<Vec<u8>, CryptoError>;
    fn generate_hmac_key(
        &self,
        hash_alg: ShaAlgorithm,
        length_bits: u16,
    ) -> Result<Vec<u8>, CryptoError>;
    fn generate_ec_key(&self, curve: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError>; // (private, public)
    fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError>;
    fn generate_x25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError>;
    fn generate_rsa_key(
        &self,
        modulus_length: u32,
        public_exponent: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), CryptoError>;
}

pub trait HmacProvider {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> Vec<u8>;
}

#[derive(Debug)]
pub enum CryptoError {
    InvalidKey,
    InvalidData,
    InvalidSignature,
    InvalidLength,
    SigningFailed,
    VerificationFailed,
    OperationFailed,
    UnsupportedAlgorithm,
    DerivationFailed,
    EncryptionFailed,
    DecryptionFailed,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::InvalidKey => write!(f, "Invalid key"),
            CryptoError::InvalidData => write!(f, "Invalid data"),
            CryptoError::InvalidSignature => write!(f, "Invalid signature"),
            CryptoError::InvalidLength => write!(f, "Invalid length"),
            CryptoError::SigningFailed => write!(f, "Signing failed"),
            CryptoError::VerificationFailed => write!(f, "Verification failed"),
            CryptoError::OperationFailed => write!(f, "Operation failed"),
            CryptoError::UnsupportedAlgorithm => write!(f, "Unsupported algorithm"),
            CryptoError::DerivationFailed => write!(f, "Derivation failed"),
            CryptoError::EncryptionFailed => write!(f, "Encryption failed"),
            CryptoError::DecryptionFailed => write!(f, "Decryption failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

#[cfg(feature = "crypto-openssl")]
pub type DefaultProvider = openssl::OpenSslProvider;

#[cfg(feature = "crypto-rust")]
pub type DefaultProvider = rust::RustCryptoProvider;

#[cfg(feature = "crypto-ring")]
pub type DefaultProvider = ring::RingProvider;
