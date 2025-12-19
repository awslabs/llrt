// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

// Ensure only one crypto provider is selected
#[cfg(all(feature = "crypto-rust", feature = "crypto-openssl"))]
compile_error!("Features `crypto-rust` and `crypto-openssl` are mutually exclusive");

#[cfg(all(feature = "crypto-rust", feature = "crypto-ring"))]
compile_error!("Features `crypto-rust` and `crypto-ring` are mutually exclusive");

#[cfg(all(feature = "crypto-rust", feature = "crypto-graviola"))]
compile_error!("Features `crypto-rust` and `crypto-graviola` are mutually exclusive");

#[cfg(all(feature = "crypto-ring", feature = "crypto-openssl"))]
compile_error!("Features `crypto-ring` and `crypto-openssl` are mutually exclusive");

#[cfg(all(feature = "crypto-ring", feature = "crypto-graviola"))]
compile_error!("Features `crypto-ring` and `crypto-graviola` are mutually exclusive");

#[cfg(all(feature = "crypto-openssl", feature = "crypto-graviola"))]
compile_error!("Features `crypto-openssl` and `crypto-graviola` are mutually exclusive");

#[cfg(all(feature = "crypto-ring-rust", feature = "crypto-graviola-rust"))]
compile_error!("Features `crypto-ring-rust` and `crypto-graviola-rust` are mutually exclusive");

#[cfg(any(feature = "crypto-graviola", feature = "crypto-graviola-rust"))]
mod graviola;

#[cfg(feature = "crypto-openssl")]
mod openssl;

#[cfg(any(feature = "crypto-ring", feature = "crypto-ring-rust"))]
mod ring;

#[cfg(feature = "_rustcrypto")]
mod rust;

use crate::sha_hash::ShaAlgorithm;
use crate::subtle::EllipticCurve;

#[derive(Debug)]
#[allow(dead_code)]
pub struct RsaImportResult {
    pub key_data: Vec<u8>,
    pub modulus_length: u32,
    pub public_exponent: Vec<u8>,
    pub is_private: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct EcImportResult {
    pub key_data: Vec<u8>,
    pub is_private: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct OkpImportResult {
    pub key_data: Vec<u8>,
    pub is_private: bool,
}

/// RSA JWK components for import (all values are raw bytes, not base64)
#[derive(Debug)]
#[allow(dead_code)]
pub struct RsaJwkImport<'a> {
    pub n: &'a [u8],          // modulus
    pub e: &'a [u8],          // public exponent
    pub d: Option<&'a [u8]>,  // private exponent
    pub p: Option<&'a [u8]>,  // first prime
    pub q: Option<&'a [u8]>,  // second prime
    pub dp: Option<&'a [u8]>, // first factor CRT exponent
    pub dq: Option<&'a [u8]>, // second factor CRT exponent
    pub qi: Option<&'a [u8]>, // first CRT coefficient
}

/// RSA JWK components for export
#[derive(Debug)]
#[allow(dead_code)]
pub struct RsaJwkExport {
    pub n: Vec<u8>,
    pub e: Vec<u8>,
    pub d: Option<Vec<u8>>,
    pub p: Option<Vec<u8>>,
    pub q: Option<Vec<u8>>,
    pub dp: Option<Vec<u8>>,
    pub dq: Option<Vec<u8>>,
    pub qi: Option<Vec<u8>>,
}

/// EC JWK components for import (all values are raw bytes)
#[derive(Debug)]
#[allow(dead_code)]
pub struct EcJwkImport<'a> {
    pub x: &'a [u8],
    pub y: &'a [u8],
    pub d: Option<&'a [u8]>,
}

/// EC JWK components for export
#[derive(Debug)]
#[allow(dead_code)]
pub struct EcJwkExport {
    pub x: Vec<u8>,
    pub y: Vec<u8>,
    pub d: Option<Vec<u8>>,
}

pub trait SimpleDigest {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> Vec<u8>;
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum AesMode {
    Ctr { counter_length: u32 },
    Cbc,
    Gcm { tag_length: u8 },
}

#[allow(dead_code)]
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

    // RSA key import from DER formats
    fn import_rsa_public_key_pkcs1(&self, der: &[u8]) -> Result<RsaImportResult, CryptoError>;
    fn import_rsa_private_key_pkcs1(&self, der: &[u8]) -> Result<RsaImportResult, CryptoError>;
    fn import_rsa_public_key_spki(&self, der: &[u8]) -> Result<RsaImportResult, CryptoError>;
    fn import_rsa_private_key_pkcs8(&self, der: &[u8]) -> Result<RsaImportResult, CryptoError>;

    // RSA key export to DER formats
    fn export_rsa_public_key_pkcs1(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn export_rsa_public_key_spki(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError>;
    fn export_rsa_private_key_pkcs8(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError>;

    // EC key import from DER formats
    fn import_ec_public_key_sec1(
        &self,
        data: &[u8],
        curve: EllipticCurve,
    ) -> Result<EcImportResult, CryptoError>;
    fn import_ec_public_key_spki(&self, der: &[u8]) -> Result<EcImportResult, CryptoError>;
    fn import_ec_private_key_pkcs8(&self, der: &[u8]) -> Result<EcImportResult, CryptoError>;
    fn import_ec_private_key_sec1(
        &self,
        data: &[u8],
        curve: EllipticCurve,
    ) -> Result<EcImportResult, CryptoError>;

    // EC key export
    fn export_ec_public_key_sec1(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
        is_private: bool,
    ) -> Result<Vec<u8>, CryptoError>;
    fn export_ec_public_key_spki(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError>;
    fn export_ec_private_key_pkcs8(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError>;

    // OKP (Ed25519/X25519) key import
    fn import_okp_public_key_raw(&self, data: &[u8]) -> Result<OkpImportResult, CryptoError>;
    fn import_okp_public_key_spki(
        &self,
        der: &[u8],
        expected_oid: &[u8],
    ) -> Result<OkpImportResult, CryptoError>;
    fn import_okp_private_key_pkcs8(
        &self,
        der: &[u8],
        expected_oid: &[u8],
    ) -> Result<OkpImportResult, CryptoError>;

    // OKP key export
    fn export_okp_public_key_raw(
        &self,
        key_data: &[u8],
        is_private: bool,
    ) -> Result<Vec<u8>, CryptoError>;
    fn export_okp_public_key_spki(
        &self,
        key_data: &[u8],
        oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;
    fn export_okp_private_key_pkcs8(
        &self,
        key_data: &[u8],
        oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;

    // JWK import/export
    fn import_rsa_jwk(&self, jwk: RsaJwkImport<'_>) -> Result<RsaImportResult, CryptoError>;
    fn export_rsa_jwk(
        &self,
        key_data: &[u8],
        is_private: bool,
    ) -> Result<RsaJwkExport, CryptoError>;
    fn import_ec_jwk(
        &self,
        jwk: EcJwkImport<'_>,
        curve: EllipticCurve,
    ) -> Result<EcImportResult, CryptoError>;
    fn export_ec_jwk(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
        is_private: bool,
    ) -> Result<EcJwkExport, CryptoError>;
}

pub trait HmacProvider {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> Vec<u8>;
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum CryptoError {
    InvalidKey(Option<Box<str>>),
    InvalidData(Option<Box<str>>),
    InvalidSignature(Option<Box<str>>),
    InvalidLength,
    SigningFailed(Option<Box<str>>),
    VerificationFailed,
    OperationFailed(Option<Box<str>>),
    UnsupportedAlgorithm,
    DerivationFailed(Option<Box<str>>),
    EncryptionFailed(Option<Box<str>>),
    DecryptionFailed(Option<Box<str>>),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::InvalidKey(None) => write!(f, "Invalid key"),
            CryptoError::InvalidKey(Some(msg)) => write!(f, "Invalid key: {}", msg),
            CryptoError::InvalidData(None) => write!(f, "Invalid data"),
            CryptoError::InvalidData(Some(msg)) => write!(f, "Invalid data: {}", msg),
            CryptoError::InvalidSignature(None) => write!(f, "Invalid signature"),
            CryptoError::InvalidSignature(Some(msg)) => write!(f, "Invalid signature: {}", msg),
            CryptoError::InvalidLength => write!(f, "Invalid length"),
            CryptoError::SigningFailed(None) => write!(f, "Signing failed"),
            CryptoError::SigningFailed(Some(msg)) => write!(f, "Signing failed: {}", msg),
            CryptoError::VerificationFailed => write!(f, "Verification failed"),
            CryptoError::OperationFailed(None) => write!(f, "Operation failed"),
            CryptoError::OperationFailed(Some(msg)) => write!(f, "Operation failed: {}", msg),
            CryptoError::UnsupportedAlgorithm => write!(f, "Unsupported algorithm"),
            CryptoError::DerivationFailed(None) => write!(f, "Derivation failed"),
            CryptoError::DerivationFailed(Some(msg)) => write!(f, "Derivation failed: {}", msg),
            CryptoError::EncryptionFailed(None) => write!(f, "Encryption failed"),
            CryptoError::EncryptionFailed(Some(msg)) => write!(f, "Encryption failed: {}", msg),
            CryptoError::DecryptionFailed(None) => write!(f, "Decryption failed"),
            CryptoError::DecryptionFailed(Some(msg)) => write!(f, "Decryption failed: {}", msg),
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

#[cfg(feature = "crypto-ring-rust")]
pub type DefaultProvider = RingRustProvider;

#[cfg(all(feature = "crypto-graviola", not(feature = "crypto-graviola-rust")))]
pub type DefaultProvider = graviola::GraviolaProvider;

#[cfg(feature = "crypto-graviola-rust")]
pub type DefaultProvider = GraviolaRustProvider;

// Macro to generate hybrid providers that delegate to RustCrypto
#[cfg(any(feature = "crypto-ring-rust", feature = "crypto-graviola-rust"))]
macro_rules! impl_hybrid_provider {
    ($name:ident, $digest:ty, $hmac:ty, $digest_fn:expr, $hmac_fn:expr, $aes_encrypt:expr, $aes_decrypt:expr) => {
        pub struct $name;
        impl CryptoProvider for $name {
            type Digest = $digest;
            type Hmac = $hmac;
            fn digest(&self, alg: ShaAlgorithm) -> Self::Digest {
                $digest_fn(alg)
            }
            fn hmac(&self, alg: ShaAlgorithm, key: &[u8]) -> Self::Hmac {
                $hmac_fn(alg, key)
            }
            fn ecdsa_sign(
                &self,
                c: EllipticCurve,
                k: &[u8],
                d: &[u8],
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.ecdsa_sign(c, k, d)
            }
            fn ecdsa_verify(
                &self,
                c: EllipticCurve,
                k: &[u8],
                s: &[u8],
                d: &[u8],
            ) -> Result<bool, CryptoError> {
                rust::RustCryptoProvider.ecdsa_verify(c, k, s, d)
            }
            fn ed25519_sign(&self, k: &[u8], d: &[u8]) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.ed25519_sign(k, d)
            }
            fn ed25519_verify(&self, k: &[u8], s: &[u8], d: &[u8]) -> Result<bool, CryptoError> {
                rust::RustCryptoProvider.ed25519_verify(k, s, d)
            }
            fn rsa_pss_sign(
                &self,
                k: &[u8],
                d: &[u8],
                s: usize,
                a: ShaAlgorithm,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.rsa_pss_sign(k, d, s, a)
            }
            fn rsa_pss_verify(
                &self,
                k: &[u8],
                s: &[u8],
                d: &[u8],
                sl: usize,
                a: ShaAlgorithm,
            ) -> Result<bool, CryptoError> {
                rust::RustCryptoProvider.rsa_pss_verify(k, s, d, sl, a)
            }
            fn rsa_pkcs1v15_sign(
                &self,
                k: &[u8],
                d: &[u8],
                a: ShaAlgorithm,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.rsa_pkcs1v15_sign(k, d, a)
            }
            fn rsa_pkcs1v15_verify(
                &self,
                k: &[u8],
                s: &[u8],
                d: &[u8],
                a: ShaAlgorithm,
            ) -> Result<bool, CryptoError> {
                rust::RustCryptoProvider.rsa_pkcs1v15_verify(k, s, d, a)
            }
            fn rsa_oaep_encrypt(
                &self,
                k: &[u8],
                d: &[u8],
                a: ShaAlgorithm,
                l: Option<&[u8]>,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.rsa_oaep_encrypt(k, d, a, l)
            }
            fn rsa_oaep_decrypt(
                &self,
                k: &[u8],
                d: &[u8],
                a: ShaAlgorithm,
                l: Option<&[u8]>,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.rsa_oaep_decrypt(k, d, a, l)
            }
            fn ecdh_derive_bits(
                &self,
                c: EllipticCurve,
                pk: &[u8],
                pubk: &[u8],
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.ecdh_derive_bits(c, pk, pubk)
            }
            fn x25519_derive_bits(&self, pk: &[u8], pubk: &[u8]) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.x25519_derive_bits(pk, pubk)
            }
            fn aes_encrypt(
                &self,
                m: AesMode,
                k: &[u8],
                iv: &[u8],
                d: &[u8],
                aad: Option<&[u8]>,
            ) -> Result<Vec<u8>, CryptoError> {
                $aes_encrypt(m, k, iv, d, aad)
            }
            fn aes_decrypt(
                &self,
                m: AesMode,
                k: &[u8],
                iv: &[u8],
                d: &[u8],
                aad: Option<&[u8]>,
            ) -> Result<Vec<u8>, CryptoError> {
                $aes_decrypt(m, k, iv, d, aad)
            }
            fn aes_kw_wrap(&self, kek: &[u8], k: &[u8]) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.aes_kw_wrap(kek, k)
            }
            fn aes_kw_unwrap(&self, kek: &[u8], w: &[u8]) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.aes_kw_unwrap(kek, w)
            }
            fn hkdf_derive_key(
                &self,
                k: &[u8],
                s: &[u8],
                i: &[u8],
                l: usize,
                a: ShaAlgorithm,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.hkdf_derive_key(k, s, i, l, a)
            }
            fn pbkdf2_derive_key(
                &self,
                p: &[u8],
                s: &[u8],
                i: u32,
                l: usize,
                a: ShaAlgorithm,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.pbkdf2_derive_key(p, s, i, l, a)
            }
            fn generate_aes_key(&self, b: u16) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.generate_aes_key(b)
            }
            fn generate_hmac_key(&self, a: ShaAlgorithm, b: u16) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.generate_hmac_key(a, b)
            }
            fn generate_ec_key(&self, c: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
                rust::RustCryptoProvider.generate_ec_key(c)
            }
            fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
                rust::RustCryptoProvider.generate_ed25519_key()
            }
            fn generate_x25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
                rust::RustCryptoProvider.generate_x25519_key()
            }
            fn generate_rsa_key(
                &self,
                b: u32,
                e: &[u8],
            ) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
                rust::RustCryptoProvider.generate_rsa_key(b, e)
            }
            fn import_rsa_public_key_pkcs1(
                &self,
                d: &[u8],
            ) -> Result<RsaImportResult, CryptoError> {
                rust::RustCryptoProvider.import_rsa_public_key_pkcs1(d)
            }
            fn import_rsa_private_key_pkcs1(
                &self,
                d: &[u8],
            ) -> Result<RsaImportResult, CryptoError> {
                rust::RustCryptoProvider.import_rsa_private_key_pkcs1(d)
            }
            fn import_rsa_public_key_spki(&self, d: &[u8]) -> Result<RsaImportResult, CryptoError> {
                rust::RustCryptoProvider.import_rsa_public_key_spki(d)
            }
            fn import_rsa_private_key_pkcs8(
                &self,
                d: &[u8],
            ) -> Result<RsaImportResult, CryptoError> {
                rust::RustCryptoProvider.import_rsa_private_key_pkcs8(d)
            }
            fn export_rsa_public_key_pkcs1(&self, d: &[u8]) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_rsa_public_key_pkcs1(d)
            }
            fn export_rsa_public_key_spki(&self, d: &[u8]) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_rsa_public_key_spki(d)
            }
            fn export_rsa_private_key_pkcs8(&self, d: &[u8]) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_rsa_private_key_pkcs8(d)
            }
            fn import_ec_public_key_sec1(
                &self,
                d: &[u8],
                c: EllipticCurve,
            ) -> Result<EcImportResult, CryptoError> {
                rust::RustCryptoProvider.import_ec_public_key_sec1(d, c)
            }
            fn import_ec_public_key_spki(&self, d: &[u8]) -> Result<EcImportResult, CryptoError> {
                rust::RustCryptoProvider.import_ec_public_key_spki(d)
            }
            fn import_ec_private_key_pkcs8(&self, d: &[u8]) -> Result<EcImportResult, CryptoError> {
                rust::RustCryptoProvider.import_ec_private_key_pkcs8(d)
            }
            fn import_ec_private_key_sec1(
                &self,
                d: &[u8],
                c: EllipticCurve,
            ) -> Result<EcImportResult, CryptoError> {
                rust::RustCryptoProvider.import_ec_private_key_sec1(d, c)
            }
            fn export_ec_public_key_sec1(
                &self,
                d: &[u8],
                c: EllipticCurve,
                p: bool,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_ec_public_key_sec1(d, c, p)
            }
            fn export_ec_public_key_spki(
                &self,
                d: &[u8],
                c: EllipticCurve,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_ec_public_key_spki(d, c)
            }
            fn export_ec_private_key_pkcs8(
                &self,
                d: &[u8],
                c: EllipticCurve,
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_ec_private_key_pkcs8(d, c)
            }
            fn import_okp_public_key_raw(&self, d: &[u8]) -> Result<OkpImportResult, CryptoError> {
                rust::RustCryptoProvider.import_okp_public_key_raw(d)
            }
            fn import_okp_public_key_spki(
                &self,
                d: &[u8],
                o: &[u8],
            ) -> Result<OkpImportResult, CryptoError> {
                rust::RustCryptoProvider.import_okp_public_key_spki(d, o)
            }
            fn import_okp_private_key_pkcs8(
                &self,
                d: &[u8],
                o: &[u8],
            ) -> Result<OkpImportResult, CryptoError> {
                rust::RustCryptoProvider.import_okp_private_key_pkcs8(d, o)
            }
            fn export_okp_public_key_raw(&self, d: &[u8], p: bool) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_okp_public_key_raw(d, p)
            }
            fn export_okp_public_key_spki(
                &self,
                d: &[u8],
                o: &[u8],
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_okp_public_key_spki(d, o)
            }
            fn export_okp_private_key_pkcs8(
                &self,
                d: &[u8],
                o: &[u8],
            ) -> Result<Vec<u8>, CryptoError> {
                rust::RustCryptoProvider.export_okp_private_key_pkcs8(d, o)
            }
            fn import_rsa_jwk(&self, j: RsaJwkImport<'_>) -> Result<RsaImportResult, CryptoError> {
                rust::RustCryptoProvider.import_rsa_jwk(j)
            }
            fn export_rsa_jwk(&self, d: &[u8], p: bool) -> Result<RsaJwkExport, CryptoError> {
                rust::RustCryptoProvider.export_rsa_jwk(d, p)
            }
            fn import_ec_jwk(
                &self,
                j: EcJwkImport<'_>,
                c: EllipticCurve,
            ) -> Result<EcImportResult, CryptoError> {
                rust::RustCryptoProvider.import_ec_jwk(j, c)
            }
            fn export_ec_jwk(
                &self,
                d: &[u8],
                c: EllipticCurve,
                p: bool,
            ) -> Result<EcJwkExport, CryptoError> {
                rust::RustCryptoProvider.export_ec_jwk(d, c, p)
            }
        }
    };
}

#[cfg(feature = "crypto-ring-rust")]
impl_hybrid_provider!(
    RingRustProvider,
    ring::RingDigestType,
    ring::RingHmacType,
    |a| ring::RingProvider.digest(a),
    |a, k| ring::RingProvider.hmac(a, k),
    |m, k, iv, d, aad| rust::RustCryptoProvider.aes_encrypt(m, k, iv, d, aad),
    |m, k, iv, d, aad| rust::RustCryptoProvider.aes_decrypt(m, k, iv, d, aad)
);

#[cfg(feature = "crypto-graviola-rust")]
fn graviola_aes_supported() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        std::arch::is_aarch64_feature_detected!("aes")
    }
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("aes")
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        false
    }
}

#[cfg(feature = "crypto-graviola-rust")]
impl_hybrid_provider!(
    GraviolaRustProvider,
    graviola::GraviolaRustDigest,
    graviola::GraviolaRustHmac,
    graviola::GraviolaRustDigest::new,
    graviola::GraviolaRustHmac::new,
    |m: AesMode, k: &[u8], iv: &[u8], d: &[u8], aad: Option<&[u8]>| {
        if graviola_aes_supported()
            && matches!(m, AesMode::Gcm { .. })
            && matches!(k.len(), 16 | 32)
        {
            graviola::GraviolaProvider.aes_encrypt(m, k, iv, d, aad)
        } else {
            rust::RustCryptoProvider.aes_encrypt(m, k, iv, d, aad)
        }
    },
    |m: AesMode, k: &[u8], iv: &[u8], d: &[u8], aad: Option<&[u8]>| {
        if graviola_aes_supported()
            && matches!(m, AesMode::Gcm { .. })
            && matches!(k.len(), 16 | 32)
        {
            graviola::GraviolaProvider.aes_decrypt(m, k, iv, d, aad)
        } else {
            rust::RustCryptoProvider.aes_decrypt(m, k, iv, d, aad)
        }
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> impl CryptoProvider {
        #[cfg(feature = "crypto-rust")]
        return rust::RustCryptoProvider;
        #[cfg(feature = "crypto-ring-rust")]
        return RingRustProvider;
        #[cfg(feature = "crypto-graviola-rust")]
        return GraviolaRustProvider;
        #[cfg(feature = "crypto-openssl")]
        return openssl::OpenSslProvider;
        #[cfg(feature = "crypto-ring")]
        return ring::RingProvider;
        #[cfg(all(feature = "crypto-graviola", not(feature = "crypto-graviola-rust")))]
        return graviola::GraviolaProvider;
    }

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    // SHA digest tests
    #[test]
    fn test_sha256_digest() {
        let p = provider();
        let mut digest = p.digest(ShaAlgorithm::SHA256);
        digest.update(b"hello world");
        let result = digest.finalize();
        assert_eq!(result.len(), 32);
        assert_eq!(
            to_hex(&result),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha384_digest() {
        let p = provider();
        let mut digest = p.digest(ShaAlgorithm::SHA384);
        digest.update(b"hello world");
        let result = digest.finalize();
        assert_eq!(result.len(), 48);
    }

    #[test]
    fn test_sha512_digest() {
        let p = provider();
        let mut digest = p.digest(ShaAlgorithm::SHA512);
        digest.update(b"hello world");
        let result = digest.finalize();
        assert_eq!(result.len(), 64);
    }

    // HMAC tests
    #[test]
    fn test_hmac_sha256() {
        let p = provider();
        let key = b"secret key";
        let mut hmac = p.hmac(ShaAlgorithm::SHA256, key);
        hmac.update(b"hello world");
        let result = hmac.finalize();
        assert_eq!(result.len(), 32);
    }

    // AES-GCM tests - only for providers that support AES
    #[cfg(any(
        feature = "crypto-rust",
        feature = "crypto-openssl",
        feature = "crypto-ring-rust",
        feature = "crypto-graviola-rust"
    ))]
    #[test]
    fn test_aes_gcm_128_roundtrip() {
        let p = provider();
        let key = [0u8; 16];
        let iv = [0u8; 12];
        let plaintext = b"hello world";
        let aad = b"additional data";

        let ciphertext = p
            .aes_encrypt(
                AesMode::Gcm { tag_length: 128 },
                &key,
                &iv,
                plaintext,
                Some(aad),
            )
            .unwrap();

        assert_eq!(ciphertext.len(), plaintext.len() + 16); // plaintext + tag

        let decrypted = p
            .aes_decrypt(
                AesMode::Gcm { tag_length: 128 },
                &key,
                &iv,
                &ciphertext,
                Some(aad),
            )
            .unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[cfg(any(
        feature = "crypto-rust",
        feature = "crypto-openssl",
        feature = "crypto-ring-rust",
        feature = "crypto-graviola-rust"
    ))]
    #[test]
    fn test_aes_gcm_256_roundtrip() {
        let p = provider();
        let key = [0u8; 32];
        let iv = [0u8; 12];
        let plaintext = b"hello world";

        let ciphertext = p
            .aes_encrypt(AesMode::Gcm { tag_length: 128 }, &key, &iv, plaintext, None)
            .unwrap();

        let decrypted = p
            .aes_decrypt(
                AesMode::Gcm { tag_length: 128 },
                &key,
                &iv,
                &ciphertext,
                None,
            )
            .unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[cfg(any(
        feature = "crypto-rust",
        feature = "crypto-openssl",
        feature = "crypto-ring-rust",
        feature = "crypto-graviola-rust"
    ))]
    #[test]
    fn test_aes_gcm_wrong_key_fails() {
        let p = provider();
        let key = [0u8; 16];
        let wrong_key = [1u8; 16];
        let iv = [0u8; 12];
        let plaintext = b"hello world";

        let ciphertext = p
            .aes_encrypt(AesMode::Gcm { tag_length: 128 }, &key, &iv, plaintext, None)
            .unwrap();

        let result = p.aes_decrypt(
            AesMode::Gcm { tag_length: 128 },
            &wrong_key,
            &iv,
            &ciphertext,
            None,
        );

        assert!(result.is_err());
    }

    // Key generation tests - only for providers that support key generation
    #[cfg(any(
        feature = "crypto-rust",
        feature = "crypto-openssl",
        feature = "crypto-ring-rust",
        feature = "crypto-graviola-rust"
    ))]
    #[test]
    fn test_generate_aes_key_128() {
        let p = provider();
        let key = p.generate_aes_key(128).unwrap();
        assert_eq!(key.len(), 16);
    }

    #[cfg(any(
        feature = "crypto-rust",
        feature = "crypto-openssl",
        feature = "crypto-ring-rust",
        feature = "crypto-graviola-rust"
    ))]
    #[test]
    fn test_generate_aes_key_256() {
        let p = provider();
        let key = p.generate_aes_key(256).unwrap();
        assert_eq!(key.len(), 32);
    }

    #[cfg(any(
        feature = "crypto-rust",
        feature = "crypto-openssl",
        feature = "crypto-ring-rust",
        feature = "crypto-graviola-rust"
    ))]
    #[test]
    fn test_generate_hmac_key() {
        let p = provider();
        let key = p.generate_hmac_key(ShaAlgorithm::SHA256, 256).unwrap();
        assert_eq!(key.len(), 32);
    }

    // Tests that require full crypto support
    #[cfg(any(
        feature = "crypto-rust",
        feature = "crypto-openssl",
        feature = "crypto-ring-rust",
        feature = "crypto-graviola-rust"
    ))]
    mod full_provider_tests {
        use super::*;

        #[test]
        fn test_aes_cbc_roundtrip() {
            let p = provider();
            let key = [0u8; 16];
            let iv = [0u8; 16];
            let plaintext = b"hello world12345"; // 16 bytes for block alignment

            let ciphertext = p
                .aes_encrypt(AesMode::Cbc, &key, &iv, plaintext, None)
                .unwrap();

            let decrypted = p
                .aes_decrypt(AesMode::Cbc, &key, &iv, &ciphertext, None)
                .unwrap();

            assert_eq!(decrypted, plaintext);
        }

        #[test]
        fn test_aes_ctr_roundtrip() {
            let p = provider();
            let key = [0u8; 16];
            let iv = [0u8; 16];
            let plaintext = b"hello world";

            let ciphertext = p
                .aes_encrypt(
                    AesMode::Ctr { counter_length: 64 },
                    &key,
                    &iv,
                    plaintext,
                    None,
                )
                .unwrap();

            let decrypted = p
                .aes_decrypt(
                    AesMode::Ctr { counter_length: 64 },
                    &key,
                    &iv,
                    &ciphertext,
                    None,
                )
                .unwrap();

            assert_eq!(decrypted, plaintext);
        }

        #[test]
        fn test_aes_kw_roundtrip() {
            let p = provider();
            let kek = [0u8; 16];
            let key_to_wrap = [1u8; 16];

            let wrapped = p.aes_kw_wrap(&kek, &key_to_wrap).unwrap();
            let unwrapped = p.aes_kw_unwrap(&kek, &wrapped).unwrap();

            assert_eq!(unwrapped, key_to_wrap);
        }

        #[test]
        fn test_hkdf_derive() {
            let p = provider();
            let ikm = b"input key material";
            let salt = b"salt";
            let info = b"info";

            let derived = p
                .hkdf_derive_key(ikm, salt, info, 32, ShaAlgorithm::SHA256)
                .unwrap();

            assert_eq!(derived.len(), 32);
        }

        #[test]
        fn test_pbkdf2_derive() {
            let p = provider();
            let password = b"password";
            let salt = b"salt";

            let derived = p
                .pbkdf2_derive_key(password, salt, 1000, 32, ShaAlgorithm::SHA256)
                .unwrap();

            assert_eq!(derived.len(), 32);
        }

        #[test]
        fn test_ec_p256_sign_verify() {
            let p = provider();
            let (private_key, public_key) = p.generate_ec_key(EllipticCurve::P256).unwrap();

            // Create a digest to sign
            let mut digest = p.digest(ShaAlgorithm::SHA256);
            digest.update(b"message to sign");
            let hash = digest.finalize();

            let signature = p
                .ecdsa_sign(EllipticCurve::P256, &private_key, &hash)
                .unwrap();

            let valid = p
                .ecdsa_verify(EllipticCurve::P256, &public_key, &signature, &hash)
                .unwrap();

            assert!(valid);
        }

        #[test]
        fn test_ec_p384_sign_verify() {
            let p = provider();
            let (private_key, public_key) = p.generate_ec_key(EllipticCurve::P384).unwrap();

            let mut digest = p.digest(ShaAlgorithm::SHA384);
            digest.update(b"message to sign");
            let hash = digest.finalize();

            let signature = p
                .ecdsa_sign(EllipticCurve::P384, &private_key, &hash)
                .unwrap();

            let valid = p
                .ecdsa_verify(EllipticCurve::P384, &public_key, &signature, &hash)
                .unwrap();

            assert!(valid);
        }

        #[test]
        fn test_ed25519_sign_verify() {
            let p = provider();
            let (private_key, public_key) = p.generate_ed25519_key().unwrap();

            let message = b"message to sign";
            let signature = p.ed25519_sign(&private_key, message).unwrap();

            let valid = p.ed25519_verify(&public_key, &signature, message).unwrap();

            assert!(valid);
        }

        #[test]
        fn test_x25519_key_exchange() {
            let p = provider();
            let (alice_private, alice_public) = p.generate_x25519_key().unwrap();
            let (bob_private, bob_public) = p.generate_x25519_key().unwrap();

            let alice_shared = p.x25519_derive_bits(&alice_private, &bob_public).unwrap();
            let bob_shared = p.x25519_derive_bits(&bob_private, &alice_public).unwrap();

            assert_eq!(alice_shared, bob_shared);
            assert_eq!(alice_shared.len(), 32);
        }

        #[test]
        fn test_ecdh_p256_key_exchange() {
            let p = provider();
            let (alice_private, alice_public) = p.generate_ec_key(EllipticCurve::P256).unwrap();
            let (bob_private, bob_public) = p.generate_ec_key(EllipticCurve::P256).unwrap();

            let alice_shared = p
                .ecdh_derive_bits(EllipticCurve::P256, &alice_private, &bob_public)
                .unwrap();
            let bob_shared = p
                .ecdh_derive_bits(EllipticCurve::P256, &bob_private, &alice_public)
                .unwrap();

            assert_eq!(alice_shared, bob_shared);
        }

        #[test]
        fn test_rsa_pss_sign_verify() {
            let p = provider();
            let (private_key, public_key) = p.generate_rsa_key(2048, &[1, 0, 1]).unwrap();

            let mut digest = p.digest(ShaAlgorithm::SHA256);
            digest.update(b"message to sign");
            let hash = digest.finalize();

            let signature = p
                .rsa_pss_sign(&private_key, &hash, 32, ShaAlgorithm::SHA256)
                .unwrap();

            let valid = p
                .rsa_pss_verify(&public_key, &signature, &hash, 32, ShaAlgorithm::SHA256)
                .unwrap();

            assert!(valid);
        }

        #[test]
        fn test_rsa_pkcs1v15_sign_verify() {
            let p = provider();
            let (private_key, public_key) = p.generate_rsa_key(2048, &[1, 0, 1]).unwrap();

            let mut digest = p.digest(ShaAlgorithm::SHA256);
            digest.update(b"message to sign");
            let hash = digest.finalize();

            let signature = p
                .rsa_pkcs1v15_sign(&private_key, &hash, ShaAlgorithm::SHA256)
                .unwrap();

            let valid = p
                .rsa_pkcs1v15_verify(&public_key, &signature, &hash, ShaAlgorithm::SHA256)
                .unwrap();

            assert!(valid);
        }

        #[test]
        fn test_rsa_oaep_encrypt_decrypt() {
            let p = provider();
            let (private_key, public_key) = p.generate_rsa_key(2048, &[1, 0, 1]).unwrap();

            let plaintext = b"secret message";

            let ciphertext = p
                .rsa_oaep_encrypt(&public_key, plaintext, ShaAlgorithm::SHA256, None)
                .unwrap();

            let decrypted = p
                .rsa_oaep_decrypt(&private_key, &ciphertext, ShaAlgorithm::SHA256, None)
                .unwrap();

            assert_eq!(decrypted, plaintext);
        }
    }
}
