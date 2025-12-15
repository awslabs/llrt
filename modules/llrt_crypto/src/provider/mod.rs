// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(feature = "crypto-graviola", feature = "crypto-graviola-rust"))]
mod graviola;
#[cfg(feature = "crypto-openssl")]
mod openssl;
#[cfg(any(feature = "crypto-ring", feature = "crypto-ring-rust"))]
mod ring;
#[cfg(any(
    feature = "crypto-rust",
    feature = "crypto-ring-rust",
    feature = "crypto-graviola-rust",
    feature = "crypto-openssl"
))]
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

#[cfg(feature = "crypto-ring-rust")]
pub type DefaultProvider = RingRustProvider;

#[cfg(feature = "crypto-graviola")]
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
impl_hybrid_provider!(
    GraviolaRustProvider,
    graviola::GraviolaRustDigest,
    graviola::GraviolaRustHmac,
    graviola::GraviolaRustDigest::new,
    graviola::GraviolaRustHmac::new,
    |m: AesMode, k: &[u8], iv: &[u8], d: &[u8], aad: Option<&[u8]>| match m {
        // Graviola only supports AES-128 and AES-256, fall back to RustCrypto for AES-192
        AesMode::Gcm { .. } if matches!(k.len(), 16 | 32) =>
            graviola::GraviolaProvider.aes_encrypt(m, k, iv, d, aad),
        _ => rust::RustCryptoProvider.aes_encrypt(m, k, iv, d, aad),
    },
    |m: AesMode, k: &[u8], iv: &[u8], d: &[u8], aad: Option<&[u8]>| match m {
        // Graviola only supports AES-128 and AES-256, fall back to RustCrypto for AES-192
        AesMode::Gcm { .. } if matches!(k.len(), 16 | 32) =>
            graviola::GraviolaProvider.aes_decrypt(m, k, iv, d, aad),
        _ => rust::RustCryptoProvider.aes_decrypt(m, k, iv, d, aad),
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
        #[cfg(feature = "crypto-graviola")]
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

    // AES-GCM tests
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

    // Key generation tests
    #[test]
    fn test_generate_aes_key_128() {
        let p = provider();
        let key = p.generate_aes_key(128).unwrap();
        assert_eq!(key.len(), 16);
    }

    #[test]
    fn test_generate_aes_key_256() {
        let p = provider();
        let key = p.generate_aes_key(256).unwrap();
        assert_eq!(key.len(), 32);
    }

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
