// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::hash::HashAlgorithm;
use crate::provider::{AesMode, CryptoError, CryptoProvider, HmacProvider, SimpleDigest};
use crate::subtle::EllipticCurve;
use md5::{Digest, Md5 as Md5Hasher};
use ring::{digest, hmac};

pub struct RingProvider;

pub enum RingDigestType {
    Sha1(RingDigest),
    Sha256(RingDigest),
    Sha384(RingDigest),
    Sha512(RingDigest),
    Md5(RingMd5),
}

pub enum RingHmacType {
    Sha1(RingHmacSha1),
    Sha256(RingHmacSha256),
    Sha384(RingHmacSha384),
    Sha512(RingHmacSha512),
}

impl SimpleDigest for RingDigestType {
    fn update(&mut self, data: &[u8]) {
        match self {
            RingDigestType::Sha1(d) => d.update(data),
            RingDigestType::Sha256(d) => d.update(data),
            RingDigestType::Sha384(d) => d.update(data),
            RingDigestType::Sha512(d) => d.update(data),
            RingDigestType::Md5(d) => d.update(data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            RingDigestType::Sha1(d) => d.finalize(),
            RingDigestType::Sha256(d) => d.finalize(),
            RingDigestType::Sha384(d) => d.finalize(),
            RingDigestType::Sha512(d) => d.finalize(),
            RingDigestType::Md5(d) => d.finalize(),
        }
    }
}

impl HmacProvider for RingHmacType {
    fn update(&mut self, data: &[u8]) {
        match self {
            RingHmacType::Sha1(h) => h.update(data),
            RingHmacType::Sha256(h) => h.update(data),
            RingHmacType::Sha384(h) => h.update(data),
            RingHmacType::Sha512(h) => h.update(data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            RingHmacType::Sha1(h) => h.finalize(),
            RingHmacType::Sha256(h) => h.finalize(),
            RingHmacType::Sha384(h) => h.finalize(),
            RingHmacType::Sha512(h) => h.finalize(),
        }
    }
}

// Simple wrapper for Ring digest
pub struct RingDigest {
    algorithm: &'static digest::Algorithm,
    data: Vec<u8>,
}

impl RingDigest {
    fn new(algorithm: &'static digest::Algorithm) -> Self {
        Self {
            algorithm,
            data: Vec::new(),
        }
    }
}

impl SimpleDigest for RingDigest {
    fn update(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    fn finalize(self) -> Vec<u8> {
        digest::digest(self.algorithm, &self.data).as_ref().to_vec()
    }
}

// MD5 wrapper
pub struct RingMd5(Md5Hasher);

impl SimpleDigest for RingMd5 {
    fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.0, data);
    }

    fn finalize(self) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}

// HMAC implementations
pub struct RingHmacSha1(hmac::Context);
pub struct RingHmacSha256(hmac::Context);
pub struct RingHmacSha384(hmac::Context);
pub struct RingHmacSha512(hmac::Context);

impl HmacProvider for RingHmacSha1 {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finalize(self) -> Vec<u8> {
        self.0.sign().as_ref().to_vec()
    }
}
impl HmacProvider for RingHmacSha256 {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finalize(self) -> Vec<u8> {
        self.0.sign().as_ref().to_vec()
    }
}
impl HmacProvider for RingHmacSha384 {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finalize(self) -> Vec<u8> {
        self.0.sign().as_ref().to_vec()
    }
}
impl HmacProvider for RingHmacSha512 {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }
    fn finalize(self) -> Vec<u8> {
        self.0.sign().as_ref().to_vec()
    }
}

impl CryptoProvider for RingProvider {
    type Digest = RingDigestType;
    type Hmac = RingHmacType;

    fn digest(&self, algorithm: HashAlgorithm) -> Self::Digest {
        match algorithm {
            HashAlgorithm::Md5 => RingDigestType::Md5(RingMd5(Md5Hasher::new())),
            HashAlgorithm::Sha1 => {
                RingDigestType::Sha1(RingDigest::new(&digest::SHA1_FOR_LEGACY_USE_ONLY))
            },
            HashAlgorithm::Sha256 => RingDigestType::Sha256(RingDigest::new(&digest::SHA256)),
            HashAlgorithm::Sha384 => RingDigestType::Sha384(RingDigest::new(&digest::SHA384)),
            HashAlgorithm::Sha512 => RingDigestType::Sha512(RingDigest::new(&digest::SHA512)),
        }
    }

    fn hmac(&self, algorithm: HashAlgorithm, key: &[u8]) -> Self::Hmac {
        match algorithm {
            HashAlgorithm::Md5 => {
                panic!("HMAC-MD5 not supported by Ring provider");
            },
            HashAlgorithm::Sha1 => RingHmacType::Sha1(RingHmacSha1(hmac::Context::with_key(
                &hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key),
            ))),
            HashAlgorithm::Sha256 => RingHmacType::Sha256(RingHmacSha256(hmac::Context::with_key(
                &hmac::Key::new(hmac::HMAC_SHA256, key),
            ))),
            HashAlgorithm::Sha384 => RingHmacType::Sha384(RingHmacSha384(hmac::Context::with_key(
                &hmac::Key::new(hmac::HMAC_SHA384, key),
            ))),
            HashAlgorithm::Sha512 => RingHmacType::Sha512(RingHmacSha512(hmac::Context::with_key(
                &hmac::Key::new(hmac::HMAC_SHA512, key),
            ))),
        }
    }

    fn ecdsa_sign(
        &self,
        _curve: EllipticCurve,
        _private_key_der: &[u8],
        _digest: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn ecdsa_verify(
        &self,
        _curve: EllipticCurve,
        _public_key_sec1: &[u8],
        _signature: &[u8],
        _digest: &[u8],
    ) -> Result<bool, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn ed25519_sign(&self, _private_key_der: &[u8], _data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn ed25519_verify(
        &self,
        _public_key_bytes: &[u8],
        _signature: &[u8],
        _data: &[u8],
    ) -> Result<bool, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_pss_sign(
        &self,
        _private_key_der: &[u8],
        _digest: &[u8],
        _salt_length: usize,
        _hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_pss_verify(
        &self,
        _public_key_der: &[u8],
        _signature: &[u8],
        _digest: &[u8],
        _salt_length: usize,
        _hash_alg: HashAlgorithm,
    ) -> Result<bool, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_pkcs1v15_sign(
        &self,
        _private_key_der: &[u8],
        _digest: &[u8],
        _hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_pkcs1v15_verify(
        &self,
        _public_key_der: &[u8],
        _signature: &[u8],
        _digest: &[u8],
        _hash_alg: HashAlgorithm,
    ) -> Result<bool, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_oaep_encrypt(
        &self,
        _public_key_der: &[u8],
        _data: &[u8],
        _hash_alg: HashAlgorithm,
        _label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_oaep_decrypt(
        &self,
        _private_key_der: &[u8],
        _data: &[u8],
        _hash_alg: HashAlgorithm,
        _label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn ecdh_derive_bits(
        &self,
        _curve: EllipticCurve,
        _private_key_der: &[u8],
        _public_key_sec1: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn x25519_derive_bits(
        &self,
        _private_key: &[u8],
        _public_key: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn aes_encrypt(
        &self,
        _mode: AesMode,
        _key: &[u8],
        _iv: &[u8],
        _data: &[u8],
        _additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn aes_decrypt(
        &self,
        _mode: AesMode,
        _key: &[u8],
        _iv: &[u8],
        _data: &[u8],
        _additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn aes_kw_wrap(&self, _kek: &[u8], _key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn aes_kw_unwrap(&self, _kek: &[u8], _wrapped_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn hkdf_derive_key(
        &self,
        _key: &[u8],
        _salt: &[u8],
        _info: &[u8],
        _length: usize,
        _hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn pbkdf2_derive_key(
        &self,
        _password: &[u8],
        _salt: &[u8],
        _iterations: u32,
        _length: usize,
        _hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_aes_key(&self, _length_bits: u16) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_hmac_key(
        &self,
        _hash_alg: HashAlgorithm,
        _length_bits: u16,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_ec_key(&self, _curve: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_x25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_rsa_key(
        &self,
        _modulus_length: u32,
        _public_exponent: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn import_rsa_public_key_pkcs1(
        &self,
        _der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_rsa_private_key_pkcs1(
        &self,
        _der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_rsa_public_key_spki(
        &self,
        _der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_rsa_private_key_pkcs8(
        &self,
        _der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_rsa_public_key_pkcs1(&self, _key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_rsa_public_key_spki(&self, _key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_rsa_private_key_pkcs8(&self, _key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_ec_public_key_sec1(
        &self,
        _data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_ec_public_key_spki(&self, _der: &[u8]) -> Result<super::EcImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_ec_private_key_pkcs8(
        &self,
        _der: &[u8],
    ) -> Result<super::EcImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_ec_private_key_sec1(
        &self,
        _data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_ec_public_key_sec1(
        &self,
        _key_data: &[u8],
        _curve: EllipticCurve,
        _is_private: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_ec_public_key_spki(
        &self,
        _key_data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_ec_private_key_pkcs8(
        &self,
        _key_data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_okp_public_key_raw(
        &self,
        _data: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_okp_public_key_spki(
        &self,
        _der: &[u8],
        _expected_oid: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_okp_private_key_pkcs8(
        &self,
        _der: &[u8],
        _expected_oid: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_okp_public_key_raw(
        &self,
        _key_data: &[u8],
        _is_private: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_okp_public_key_spki(
        &self,
        _key_data: &[u8],
        _oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_okp_private_key_pkcs8(
        &self,
        _key_data: &[u8],
        _oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_rsa_jwk(
        &self,
        _jwk: super::RsaJwkImport<'_>,
    ) -> Result<super::RsaImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_rsa_jwk(
        &self,
        _key_data: &[u8],
        _is_private: bool,
    ) -> Result<super::RsaJwkExport, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_ec_jwk(
        &self,
        _jwk: super::EcJwkImport<'_>,
        _curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_ec_jwk(
        &self,
        _key_data: &[u8],
        _curve: EllipticCurve,
        _is_private: bool,
    ) -> Result<super::EcJwkExport, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn import_okp_jwk(
        &self,
        _jwk: super::OkpJwkImport<'_>,
        _is_ed25519: bool,
    ) -> Result<super::OkpImportResult, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
    fn export_okp_jwk(
        &self,
        _key_data: &[u8],
        _is_private: bool,
        _is_ed25519: bool,
    ) -> Result<super::OkpJwkExport, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
}
