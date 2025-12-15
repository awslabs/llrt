// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Graviola crypto provider - a high-performance crypto library using formally verified assembler.
//!
//! Supported: SHA256/384/512, HMAC, AES-GCM
//! Not supported: Most other operations due to API limitations

use graviola::{
    aead::AesGcm,
    hashing::{hmac::Hmac, Hash, HashContext, Sha256, Sha384, Sha512},
};

use crate::provider::{AesMode, CryptoError, CryptoProvider, HmacProvider, SimpleDigest};
use crate::sha_hash::ShaAlgorithm;
use crate::subtle::EllipticCurve;

pub struct GraviolaProvider;

pub enum GraviolaDigest {
    Sha256(<Sha256 as Hash>::Context),
    Sha384(<Sha384 as Hash>::Context),
    Sha512(<Sha512 as Hash>::Context),
}

impl SimpleDigest for GraviolaDigest {
    fn update(&mut self, data: &[u8]) {
        match self {
            GraviolaDigest::Sha256(h) => h.update(data),
            GraviolaDigest::Sha384(h) => h.update(data),
            GraviolaDigest::Sha512(h) => h.update(data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            GraviolaDigest::Sha256(h) => h.finish().as_ref().to_vec(),
            GraviolaDigest::Sha384(h) => h.finish().as_ref().to_vec(),
            GraviolaDigest::Sha512(h) => h.finish().as_ref().to_vec(),
        }
    }
}

pub enum GraviolaHmac {
    Sha256(Hmac<Sha256>),
    Sha384(Hmac<Sha384>),
    Sha512(Hmac<Sha512>),
}

impl HmacProvider for GraviolaHmac {
    fn update(&mut self, data: &[u8]) {
        match self {
            GraviolaHmac::Sha256(h) => h.update(data),
            GraviolaHmac::Sha384(h) => h.update(data),
            GraviolaHmac::Sha512(h) => h.update(data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            GraviolaHmac::Sha256(h) => h.finish().as_ref().to_vec(),
            GraviolaHmac::Sha384(h) => h.finish().as_ref().to_vec(),
            GraviolaHmac::Sha512(h) => h.finish().as_ref().to_vec(),
        }
    }
}

impl CryptoProvider for GraviolaProvider {
    type Digest = GraviolaDigest;
    type Hmac = GraviolaHmac;

    fn digest(&self, algorithm: ShaAlgorithm) -> Self::Digest {
        match algorithm {
            ShaAlgorithm::SHA256 => GraviolaDigest::Sha256(Sha256::new()),
            ShaAlgorithm::SHA384 => GraviolaDigest::Sha384(Sha384::new()),
            ShaAlgorithm::SHA512 => GraviolaDigest::Sha512(Sha512::new()),
            _ => panic!("Unsupported digest algorithm for Graviola"),
        }
    }

    fn hmac(&self, algorithm: ShaAlgorithm, key: &[u8]) -> Self::Hmac {
        match algorithm {
            ShaAlgorithm::SHA256 => GraviolaHmac::Sha256(Hmac::<Sha256>::new(key)),
            ShaAlgorithm::SHA384 => GraviolaHmac::Sha384(Hmac::<Sha384>::new(key)),
            ShaAlgorithm::SHA512 => GraviolaHmac::Sha512(Hmac::<Sha512>::new(key)),
            _ => panic!("Unsupported HMAC algorithm for Graviola"),
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
        _hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_pss_verify(
        &self,
        _public_key_der: &[u8],
        _signature: &[u8],
        _digest: &[u8],
        _salt_length: usize,
        _hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_pkcs1v15_sign(
        &self,
        _private_key_der: &[u8],
        _digest: &[u8],
        _hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_pkcs1v15_verify(
        &self,
        _public_key_der: &[u8],
        _signature: &[u8],
        _digest: &[u8],
        _hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_oaep_encrypt(
        &self,
        _public_key_der: &[u8],
        _data: &[u8],
        _hash_alg: ShaAlgorithm,
        _label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn rsa_oaep_decrypt(
        &self,
        _private_key_der: &[u8],
        _data: &[u8],
        _hash_alg: ShaAlgorithm,
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
        // Graviola doesn't expose from_bytes for X25519 PrivateKey
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn aes_encrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        match mode {
            AesMode::Gcm { .. } => {
                let nonce: [u8; 12] = iv.try_into().map_err(|_| CryptoError::InvalidData)?;
                if !matches!(key.len(), 16 | 32) {
                    return Err(CryptoError::InvalidKey);
                }
                let aead = AesGcm::new(key);
                let aad = additional_data.unwrap_or(&[]);
                let mut ciphertext = data.to_vec();
                let mut tag = [0u8; 16];
                aead.encrypt(&nonce, aad, &mut ciphertext, &mut tag);
                ciphertext.extend_from_slice(&tag);
                Ok(ciphertext)
            },
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn aes_decrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        match mode {
            AesMode::Gcm { .. } => {
                let nonce: [u8; 12] = iv.try_into().map_err(|_| CryptoError::InvalidData)?;
                if !matches!(key.len(), 16 | 32) {
                    return Err(CryptoError::InvalidKey);
                }
                if data.len() < 16 {
                    return Err(CryptoError::InvalidData);
                }
                let aead = AesGcm::new(key);
                let aad = additional_data.unwrap_or(&[]);
                let (ciphertext, tag) = data.split_at(data.len() - 16);
                let tag: [u8; 16] = tag.try_into().unwrap();
                let mut plaintext = ciphertext.to_vec();
                aead.decrypt(&nonce, aad, &mut plaintext, &tag)
                    .map_err(|_| CryptoError::DecryptionFailed)?;
                Ok(plaintext)
            },
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
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
        _hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn pbkdf2_derive_key(
        &self,
        _password: &[u8],
        _salt: &[u8],
        _iterations: u32,
        _length: usize,
        _hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_aes_key(&self, length_bits: u16) -> Result<Vec<u8>, CryptoError> {
        if !matches!(length_bits, 128 | 256) {
            return Err(CryptoError::InvalidLength);
        }
        Ok(crate::random_byte_array((length_bits / 8) as usize))
    }

    fn generate_hmac_key(
        &self,
        hash_alg: ShaAlgorithm,
        length_bits: u16,
    ) -> Result<Vec<u8>, CryptoError> {
        let length_bytes = if length_bits == 0 {
            match hash_alg {
                ShaAlgorithm::SHA256 => 64,
                ShaAlgorithm::SHA384 | ShaAlgorithm::SHA512 => 128,
                _ => return Err(CryptoError::UnsupportedAlgorithm),
            }
        } else {
            (length_bits / 8) as usize
        };
        Ok(crate::random_byte_array(length_bytes))
    }

    fn generate_ec_key(&self, _curve: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_x25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        // Graviola doesn't expose as_bytes for X25519 PrivateKey
        Err(CryptoError::UnsupportedAlgorithm)
    }

    fn generate_rsa_key(
        &self,
        _modulus_length: u32,
        _public_exponent: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        Err(CryptoError::UnsupportedAlgorithm)
    }
}

// Hybrid types for graviola-rust: Graviola for SHA256/384/512, RustCrypto for MD5/SHA1
#[cfg(feature = "crypto-graviola-rust")]
pub enum GraviolaRustDigest {
    Graviola(GraviolaDigest),
    Rust(super::rust::RustDigest),
}

#[cfg(feature = "crypto-graviola-rust")]
impl GraviolaRustDigest {
    pub fn new(algorithm: ShaAlgorithm) -> Self {
        match algorithm {
            ShaAlgorithm::SHA256 | ShaAlgorithm::SHA384 | ShaAlgorithm::SHA512 => {
                Self::Graviola(GraviolaProvider.digest(algorithm))
            },
            _ => Self::Rust(super::rust::RustCryptoProvider.digest(algorithm)),
        }
    }
}

#[cfg(feature = "crypto-graviola-rust")]
impl SimpleDigest for GraviolaRustDigest {
    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Graviola(d) => d.update(data),
            Self::Rust(d) => d.update(data),
        }
    }
    fn finalize(self) -> Vec<u8> {
        match self {
            Self::Graviola(d) => d.finalize(),
            Self::Rust(d) => d.finalize(),
        }
    }
}

#[cfg(feature = "crypto-graviola-rust")]
pub enum GraviolaRustHmac {
    Graviola(GraviolaHmac),
    Rust(super::rust::RustHmac),
}

#[cfg(feature = "crypto-graviola-rust")]
impl GraviolaRustHmac {
    pub fn new(algorithm: ShaAlgorithm, key: &[u8]) -> Self {
        match algorithm {
            ShaAlgorithm::SHA256 | ShaAlgorithm::SHA384 | ShaAlgorithm::SHA512 => {
                Self::Graviola(GraviolaProvider.hmac(algorithm, key))
            },
            _ => Self::Rust(super::rust::RustCryptoProvider.hmac(algorithm, key)),
        }
    }
}

#[cfg(feature = "crypto-graviola-rust")]
impl HmacProvider for GraviolaRustHmac {
    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Graviola(h) => h.update(data),
            Self::Rust(h) => h.update(data),
        }
    }
    fn finalize(self) -> Vec<u8> {
        match self {
            Self::Graviola(h) => h.finalize(),
            Self::Rust(h) => h.finalize(),
        }
    }
}
