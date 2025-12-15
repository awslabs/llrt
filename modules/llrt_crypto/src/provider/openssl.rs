// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! OpenSSL crypto provider - uses OpenSSL for cryptographic operations.

use openssl::hash::{Hasher, MessageDigest};
use openssl::pkey::PKey;
use openssl::sign::Signer;

use crate::provider::{AesMode, CryptoError, CryptoProvider, HmacProvider, SimpleDigest};
use crate::sha_hash::ShaAlgorithm;
use crate::subtle::EllipticCurve;

pub struct OpenSslProvider;

pub enum OpenSslDigest {
    Md5(Hasher),
    Sha1(Hasher),
    Sha256(Hasher),
    Sha384(Hasher),
    Sha512(Hasher),
}

impl SimpleDigest for OpenSslDigest {
    fn update(&mut self, data: &[u8]) {
        match self {
            OpenSslDigest::Md5(h)
            | OpenSslDigest::Sha1(h)
            | OpenSslDigest::Sha256(h)
            | OpenSslDigest::Sha384(h)
            | OpenSslDigest::Sha512(h) => {
                let _ = h.update(data);
            },
        }
    }

    fn finalize(mut self) -> Vec<u8> {
        match self {
            OpenSslDigest::Md5(ref mut h)
            | OpenSslDigest::Sha1(ref mut h)
            | OpenSslDigest::Sha256(ref mut h)
            | OpenSslDigest::Sha384(ref mut h)
            | OpenSslDigest::Sha512(ref mut h) => {
                h.finish().map(|d| d.to_vec()).unwrap_or_default()
            },
        }
    }
}

pub struct OpenSslHmac {
    signer: Signer<'static>,
}

impl HmacProvider for OpenSslHmac {
    fn update(&mut self, data: &[u8]) {
        let _ = self.signer.update(data);
    }

    fn finalize(self) -> Vec<u8> {
        self.signer.sign_to_vec().unwrap_or_default()
    }
}

fn get_message_digest(alg: ShaAlgorithm) -> MessageDigest {
    match alg {
        ShaAlgorithm::MD5 => MessageDigest::md5(),
        ShaAlgorithm::SHA1 => MessageDigest::sha1(),
        ShaAlgorithm::SHA256 => MessageDigest::sha256(),
        ShaAlgorithm::SHA384 => MessageDigest::sha384(),
        ShaAlgorithm::SHA512 => MessageDigest::sha512(),
    }
}

impl CryptoProvider for OpenSslProvider {
    type Digest = OpenSslDigest;
    type Hmac = OpenSslHmac;

    fn digest(&self, algorithm: ShaAlgorithm) -> Self::Digest {
        let md = get_message_digest(algorithm);
        let hasher = Hasher::new(md).expect("Failed to create hasher");
        match algorithm {
            ShaAlgorithm::MD5 => OpenSslDigest::Md5(hasher),
            ShaAlgorithm::SHA1 => OpenSslDigest::Sha1(hasher),
            ShaAlgorithm::SHA256 => OpenSslDigest::Sha256(hasher),
            ShaAlgorithm::SHA384 => OpenSslDigest::Sha384(hasher),
            ShaAlgorithm::SHA512 => OpenSslDigest::Sha512(hasher),
        }
    }

    fn hmac(&self, algorithm: ShaAlgorithm, key: &[u8]) -> Self::Hmac {
        let md = get_message_digest(algorithm);
        let pkey = PKey::hmac(key).expect("Failed to create HMAC key");
        // SAFETY: We're creating a static lifetime signer, but we own the key
        let signer = unsafe {
            std::mem::transmute::<Signer<'_>, Signer<'static>>(
                Signer::new(md, &pkey).expect("Failed to create signer"),
            )
        };
        OpenSslHmac { signer }
    }

    // Delegate to RustCrypto for complex operations
    fn ecdsa_sign(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        digest: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.ecdsa_sign(curve, private_key_der, digest)
    }

    fn ecdsa_verify(
        &self,
        curve: EllipticCurve,
        public_key_sec1: &[u8],
        signature: &[u8],
        digest: &[u8],
    ) -> Result<bool, CryptoError> {
        super::rust::RustCryptoProvider.ecdsa_verify(curve, public_key_sec1, signature, digest)
    }

    fn ed25519_sign(&self, private_key_der: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.ed25519_sign(private_key_der, data)
    }

    fn ed25519_verify(
        &self,
        public_key_bytes: &[u8],
        signature: &[u8],
        data: &[u8],
    ) -> Result<bool, CryptoError> {
        super::rust::RustCryptoProvider.ed25519_verify(public_key_bytes, signature, data)
    }

    fn rsa_pss_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.rsa_pss_sign(private_key_der, digest, salt_length, hash_alg)
    }

    fn rsa_pss_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        super::rust::RustCryptoProvider.rsa_pss_verify(
            public_key_der,
            signature,
            digest,
            salt_length,
            hash_alg,
        )
    }

    fn rsa_pkcs1v15_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.rsa_pkcs1v15_sign(private_key_der, digest, hash_alg)
    }

    fn rsa_pkcs1v15_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        super::rust::RustCryptoProvider.rsa_pkcs1v15_verify(
            public_key_der,
            signature,
            digest,
            hash_alg,
        )
    }

    fn rsa_oaep_encrypt(
        &self,
        public_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.rsa_oaep_encrypt(public_key_der, data, hash_alg, label)
    }

    fn rsa_oaep_decrypt(
        &self,
        private_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.rsa_oaep_decrypt(private_key_der, data, hash_alg, label)
    }

    fn ecdh_derive_bits(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        public_key_sec1: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.ecdh_derive_bits(curve, private_key_der, public_key_sec1)
    }

    fn x25519_derive_bits(
        &self,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.x25519_derive_bits(private_key, public_key)
    }

    fn aes_encrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.aes_encrypt(mode, key, iv, data, additional_data)
    }

    fn aes_decrypt(
        &self,
        mode: AesMode,
        key: &[u8],
        iv: &[u8],
        data: &[u8],
        additional_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.aes_decrypt(mode, key, iv, data, additional_data)
    }

    fn aes_kw_wrap(&self, kek: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.aes_kw_wrap(kek, key)
    }

    fn aes_kw_unwrap(&self, kek: &[u8], wrapped_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.aes_kw_unwrap(kek, wrapped_key)
    }

    fn hkdf_derive_key(
        &self,
        key: &[u8],
        salt: &[u8],
        info: &[u8],
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.hkdf_derive_key(key, salt, info, length, hash_alg)
    }

    fn pbkdf2_derive_key(
        &self,
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider
            .pbkdf2_derive_key(password, salt, iterations, length, hash_alg)
    }

    fn generate_aes_key(&self, length_bits: u16) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.generate_aes_key(length_bits)
    }

    fn generate_hmac_key(
        &self,
        hash_alg: ShaAlgorithm,
        length_bits: u16,
    ) -> Result<Vec<u8>, CryptoError> {
        super::rust::RustCryptoProvider.generate_hmac_key(hash_alg, length_bits)
    }

    fn generate_ec_key(&self, curve: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        super::rust::RustCryptoProvider.generate_ec_key(curve)
    }

    fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        super::rust::RustCryptoProvider.generate_ed25519_key()
    }

    fn generate_x25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        super::rust::RustCryptoProvider.generate_x25519_key()
    }

    fn generate_rsa_key(
        &self,
        modulus_length: u32,
        public_exponent: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        super::rust::RustCryptoProvider.generate_rsa_key(modulus_length, public_exponent)
    }
}
