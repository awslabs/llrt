// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::num::NonZeroU32;

use aes::cipher::{
    block_padding::Pkcs7, BlockModeDecrypt, BlockModeEncrypt, KeyIvInit, StreamCipher,
    StreamCipherError,
};
use aes_gcm::{
    aead::{Aead, Payload},
    KeyInit, Nonce,
};
use aes_kw::{KwAes128, KwAes192, KwAes256};
use cbc::{Decryptor, Encryptor};
use ctr::{cipher::Array, Ctr128BE, Ctr32BE, Ctr64BE};
use ecdsa::signature::hazmat::PrehashVerifier;
use elliptic_curve::consts::U12;
use hmac::{Hmac as HmacImpl, Mac};
use once_cell::sync::Lazy;
use p256::{
    ecdsa::{
        Signature as P256Signature, SigningKey as P256SigningKey, VerifyingKey as P256VerifyingKey,
    },
    SecretKey as P256SecretKey,
};
use p384::{
    ecdsa::{
        Signature as P384Signature, SigningKey as P384SigningKey, VerifyingKey as P384VerifyingKey,
    },
    SecretKey as P384SecretKey,
};
use p521::{
    ecdsa::{Signature as P521Signature, VerifyingKey as P521VerifyingKey},
    SecretKey as P521SecretKey,
};
use pkcs8::EncodePrivateKey;
use ring::{
    pbkdf2,
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey},
};
use rsa::pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey,
};
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::hazmat::PrehashSigner;
use rsa::{
    pss::Pss,
    sha2::{Digest, Sha256, Sha384, Sha512},
    BoxedUint, Oaep, Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey,
};
use sha1::Sha1;

use crate::{
    provider::{AesMode, CryptoError, CryptoProvider, HmacProvider, SimpleDigest},
    random_byte_array,
    sha_hash::ShaAlgorithm,
    subtle::{AesGcmVariant, EllipticCurve},
};

impl From<aes::cipher::InvalidLength> for CryptoError {
    fn from(_: aes::cipher::InvalidLength) -> Self {
        CryptoError::InvalidLength
    }
}

impl From<StreamCipherError> for CryptoError {
    fn from(_: StreamCipherError) -> Self {
        CryptoError::OperationFailed
    }
}

// Digest implementation using sha2/md5 crates
pub enum RustDigest {
    Md5(md5::Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

impl SimpleDigest for RustDigest {
    fn update(&mut self, data: &[u8]) {
        match self {
            RustDigest::Md5(h) => Digest::update(h, data),
            RustDigest::Sha1(h) => Digest::update(h, data),
            RustDigest::Sha256(h) => Digest::update(h, data),
            RustDigest::Sha384(h) => Digest::update(h, data),
            RustDigest::Sha512(h) => Digest::update(h, data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            RustDigest::Md5(h) => h.finalize().to_vec(),
            RustDigest::Sha1(h) => h.finalize().to_vec(),
            RustDigest::Sha256(h) => h.finalize().to_vec(),
            RustDigest::Sha384(h) => h.finalize().to_vec(),
            RustDigest::Sha512(h) => h.finalize().to_vec(),
        }
    }
}

// HMAC implementation using hmac crate
pub enum RustHmac {
    Sha1(HmacImpl<Sha1>),
    Sha256(HmacImpl<Sha256>),
    Sha384(HmacImpl<Sha384>),
    Sha512(HmacImpl<Sha512>),
}

impl HmacProvider for RustHmac {
    fn update(&mut self, data: &[u8]) {
        match self {
            RustHmac::Sha1(h) => Mac::update(h, data),
            RustHmac::Sha256(h) => Mac::update(h, data),
            RustHmac::Sha384(h) => Mac::update(h, data),
            RustHmac::Sha512(h) => Mac::update(h, data),
        }
    }

    fn finalize(self) -> Vec<u8> {
        match self {
            RustHmac::Sha1(h) => h.finalize().into_bytes().to_vec(),
            RustHmac::Sha256(h) => h.finalize().into_bytes().to_vec(),
            RustHmac::Sha384(h) => h.finalize().into_bytes().to_vec(),
            RustHmac::Sha512(h) => h.finalize().into_bytes().to_vec(),
        }
    }
}

// Main Crypto Provider
#[derive(Default)]
pub struct RustCryptoProvider;

pub static SYSTEM_RANDOM: Lazy<SystemRandom> = Lazy::new(SystemRandom::new);

impl CryptoProvider for RustCryptoProvider {
    type Digest = RustDigest;
    type Hmac = RustHmac;

    fn digest(&self, algorithm: ShaAlgorithm) -> Self::Digest {
        match algorithm {
            ShaAlgorithm::MD5 => RustDigest::Md5(md5::Md5::new()),
            ShaAlgorithm::SHA1 => RustDigest::Sha1(Sha1::new()),
            ShaAlgorithm::SHA256 => RustDigest::Sha256(Sha256::new()),
            ShaAlgorithm::SHA384 => RustDigest::Sha384(Sha384::new()),
            ShaAlgorithm::SHA512 => RustDigest::Sha512(Sha512::new()),
        }
    }

    fn hmac(&self, algorithm: ShaAlgorithm, key: &[u8]) -> Self::Hmac {
        match algorithm {
            ShaAlgorithm::MD5 => panic!("HMAC-MD5 not supported"),
            ShaAlgorithm::SHA1 => RustHmac::Sha1(HmacImpl::<Sha1>::new_from_slice(key).unwrap()),
            ShaAlgorithm::SHA256 => {
                RustHmac::Sha256(HmacImpl::<Sha256>::new_from_slice(key).unwrap())
            },
            ShaAlgorithm::SHA384 => {
                RustHmac::Sha384(HmacImpl::<Sha384>::new_from_slice(key).unwrap())
            },
            ShaAlgorithm::SHA512 => {
                RustHmac::Sha512(HmacImpl::<Sha512>::new_from_slice(key).unwrap())
            },
        }
    }

    fn ecdsa_sign(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        digest: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        match curve {
            EllipticCurve::P256 => {
                let secret_key = P256SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let signing_key = P256SigningKey::from(secret_key);
                let signature: p256::ecdsa::Signature = signing_key
                    .sign_prehash(digest)
                    .map_err(|_| CryptoError::SigningFailed)?;
                Ok(signature.to_bytes().to_vec())
            },
            EllipticCurve::P384 => {
                let secret_key = P384SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let signing_key = P384SigningKey::from(secret_key);
                let signature: p384::ecdsa::Signature = signing_key
                    .sign_prehash(digest)
                    .map_err(|_| CryptoError::SigningFailed)?;
                Ok(signature.to_bytes().to_vec())
            },
            EllipticCurve::P521 => {
                let secret_key = P521SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let signing_key = p521::ecdsa::SigningKey::from(secret_key);
                let signature: p521::ecdsa::Signature = signing_key
                    .sign_prehash(digest)
                    .map_err(|_| CryptoError::SigningFailed)?;
                Ok(signature.to_bytes().to_vec())
            },
        }
    }

    fn ecdsa_verify(
        &self,
        curve: EllipticCurve,
        public_key_sec1: &[u8],
        signature: &[u8],
        digest: &[u8],
    ) -> Result<bool, CryptoError> {
        match curve {
            EllipticCurve::P256 => {
                let verifying_key = P256VerifyingKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let sig = P256Signature::from_slice(signature)
                    .map_err(|_| CryptoError::InvalidSignature)?;
                Ok(verifying_key.verify_prehash(digest, &sig).is_ok())
            },
            EllipticCurve::P384 => {
                let verifying_key = P384VerifyingKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let sig = P384Signature::from_slice(signature)
                    .map_err(|_| CryptoError::InvalidSignature)?;
                Ok(verifying_key.verify_prehash(digest, &sig).is_ok())
            },
            EllipticCurve::P521 => {
                let verifying_key = P521VerifyingKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let sig = P521Signature::from_slice(signature)
                    .map_err(|_| CryptoError::InvalidSignature)?;
                Ok(verifying_key.verify_prehash(digest, &sig).is_ok())
            },
        }
    }

    fn ed25519_sign(&self, private_key_der: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let key_pair =
            Ed25519KeyPair::from_pkcs8(private_key_der).map_err(|_| CryptoError::InvalidKey)?;
        let signature = key_pair.sign(data);
        Ok(signature.as_ref().to_vec())
    }

    fn ed25519_verify(
        &self,
        public_key_bytes: &[u8],
        signature: &[u8],
        data: &[u8],
    ) -> Result<bool, CryptoError> {
        let public_key = UnparsedPublicKey::new(&ring::signature::ED25519, public_key_bytes);
        Ok(public_key.verify(data, signature).is_ok())
    }

    fn rsa_pss_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut rng = rand::rng();
        let private_key =
            RsaPrivateKey::from_pkcs1_der(private_key_der).map_err(|_| CryptoError::InvalidKey)?;

        match hash_alg {
            ShaAlgorithm::SHA256 => private_key
                .sign_with_rng(&mut rng, Pss::<Sha256>::new_with_salt(salt_length), digest)
                .map_err(|_| CryptoError::SigningFailed),
            ShaAlgorithm::SHA384 => private_key
                .sign_with_rng(&mut rng, Pss::<Sha384>::new_with_salt(salt_length), digest)
                .map_err(|_| CryptoError::SigningFailed),
            ShaAlgorithm::SHA512 => private_key
                .sign_with_rng(&mut rng, Pss::<Sha512>::new_with_salt(salt_length), digest)
                .map_err(|_| CryptoError::SigningFailed),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_pss_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        let public_key =
            RsaPublicKey::from_pkcs1_der(public_key_der).map_err(|_| CryptoError::InvalidKey)?;

        match hash_alg {
            ShaAlgorithm::SHA256 => Ok(public_key
                .verify(Pss::<Sha256>::new_with_salt(salt_length), digest, signature)
                .is_ok()),
            ShaAlgorithm::SHA384 => Ok(public_key
                .verify(Pss::<Sha384>::new_with_salt(salt_length), digest, signature)
                .is_ok()),
            ShaAlgorithm::SHA512 => Ok(public_key
                .verify(Pss::<Sha512>::new_with_salt(salt_length), digest, signature)
                .is_ok()),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_pkcs1v15_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut rng = rand::rng();
        let private_key =
            RsaPrivateKey::from_pkcs1_der(private_key_der).map_err(|_| CryptoError::InvalidKey)?;

        match hash_alg {
            ShaAlgorithm::SHA256 => private_key
                .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha256>(), digest)
                .map_err(|_| CryptoError::SigningFailed),
            ShaAlgorithm::SHA384 => private_key
                .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha384>(), digest)
                .map_err(|_| CryptoError::SigningFailed),
            ShaAlgorithm::SHA512 => private_key
                .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha512>(), digest)
                .map_err(|_| CryptoError::SigningFailed),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_pkcs1v15_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        hash_alg: ShaAlgorithm,
    ) -> Result<bool, CryptoError> {
        let public_key =
            RsaPublicKey::from_pkcs1_der(public_key_der).map_err(|_| CryptoError::InvalidKey)?;

        match hash_alg {
            ShaAlgorithm::SHA256 => Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha256>(), digest, signature)
                .is_ok()),
            ShaAlgorithm::SHA384 => Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha384>(), digest, signature)
                .is_ok()),
            ShaAlgorithm::SHA512 => Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha512>(), digest, signature)
                .is_ok()),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_oaep_encrypt(
        &self,
        public_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut rng = rand::rng();
        let public_key =
            RsaPublicKey::from_pkcs1_der(public_key_der).map_err(|_| CryptoError::InvalidKey)?;

        match hash_alg {
            ShaAlgorithm::SHA1 => {
                let mut padding = Oaep::<Sha1>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed)
            },
            ShaAlgorithm::SHA256 => {
                let mut padding = Oaep::<Sha256>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed)
            },
            ShaAlgorithm::SHA384 => {
                let mut padding = Oaep::<Sha384>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed)
            },
            ShaAlgorithm::SHA512 => {
                let mut padding = Oaep::<Sha512>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed)
            },
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_oaep_decrypt(
        &self,
        private_key_der: &[u8],
        data: &[u8],
        hash_alg: ShaAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        let private_key =
            RsaPrivateKey::from_pkcs1_der(private_key_der).map_err(|_| CryptoError::InvalidKey)?;

        match hash_alg {
            ShaAlgorithm::SHA1 => {
                let mut padding = Oaep::<Sha1>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed)
            },
            ShaAlgorithm::SHA256 => {
                let mut padding = Oaep::<Sha256>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed)
            },
            ShaAlgorithm::SHA384 => {
                let mut padding = Oaep::<Sha384>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed)
            },
            ShaAlgorithm::SHA512 => {
                let mut padding = Oaep::<Sha512>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed)
            },
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn ecdh_derive_bits(
        &self,
        curve: EllipticCurve,
        private_key_der: &[u8],
        public_key_sec1: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        match curve {
            EllipticCurve::P256 => {
                let secret_key = P256SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let public_key = p256::PublicKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let shared_secret = p256::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );
                Ok(shared_secret.raw_secret_bytes().to_vec())
            },
            EllipticCurve::P384 => {
                let secret_key = P384SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let public_key = p384::PublicKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let shared_secret = p384::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );
                Ok(shared_secret.raw_secret_bytes().to_vec())
            },
            EllipticCurve::P521 => {
                let secret_key = P521SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let public_key = p521::PublicKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey)?;
                let shared_secret = p521::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );
                Ok(shared_secret.raw_secret_bytes().to_vec())
            },
        }
    }

    fn x25519_derive_bits(
        &self,
        private_key: &[u8],
        public_key: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let private_array: [u8; 32] = private_key
            .try_into()
            .map_err(|_| CryptoError::InvalidKey)?;
        let public_array: [u8; 32] = public_key.try_into().map_err(|_| CryptoError::InvalidKey)?;

        let secret_key = x25519_dalek::StaticSecret::from(private_array);
        let public_key = x25519_dalek::PublicKey::from(public_array);
        let shared_secret = secret_key.diffie_hellman(&public_key);

        Ok(shared_secret.as_bytes().to_vec())
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
            AesMode::Cbc => match key.len() {
                16 => {
                    let encryptor = Encryptor::<aes::Aes128>::new_from_slices(key, iv)?;
                    Ok(encryptor.encrypt_padded_vec::<Pkcs7>(data))
                },
                24 => {
                    let encryptor = Encryptor::<aes::Aes192>::new_from_slices(key, iv)?;
                    Ok(encryptor.encrypt_padded_vec::<Pkcs7>(data))
                },
                32 => {
                    let encryptor = Encryptor::<aes::Aes256>::new_from_slices(key, iv)?;
                    Ok(encryptor.encrypt_padded_vec::<Pkcs7>(data))
                },
                _ => Err(CryptoError::InvalidKey),
            },
            AesMode::Ctr { counter_length } => {
                let mut ciphertext = data.to_vec();
                match (key.len(), counter_length) {
                    (16, 32) => {
                        let mut cipher = Ctr32BE::<aes::Aes128>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (16, 64) => {
                        let mut cipher = Ctr64BE::<aes::Aes128>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (16, 128) => {
                        let mut cipher = Ctr128BE::<aes::Aes128>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (24, 32) => {
                        let mut cipher = Ctr32BE::<aes::Aes192>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (24, 64) => {
                        let mut cipher = Ctr64BE::<aes::Aes192>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (24, 128) => {
                        let mut cipher = Ctr128BE::<aes::Aes192>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (32, 32) => {
                        let mut cipher = Ctr32BE::<aes::Aes256>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (32, 64) => {
                        let mut cipher = Ctr64BE::<aes::Aes256>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    (32, 128) => {
                        let mut cipher = Ctr128BE::<aes::Aes256>::new_from_slices(key, iv)?;
                        cipher.try_apply_keystream(&mut ciphertext)?;
                    },
                    _ => return Err(CryptoError::InvalidKey),
                }
                Ok(ciphertext)
            },
            AesMode::Gcm { tag_length } => {
                let variant = AesGcmVariant::new((key.len() * 8) as u16, tag_length, key)?;
                let nonce: &Array<_, _> =
                    &Nonce::<U12>::try_from(iv).map_err(|_| CryptoError::InvalidData)?;

                let plaintext = Payload {
                    msg: data,
                    aad: additional_data.unwrap_or_default(),
                };

                match variant {
                    AesGcmVariant::Aes128Gcm96(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes192Gcm96(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes256Gcm96(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes128Gcm104(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes192Gcm104(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes256Gcm104(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes128Gcm112(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes192Gcm112(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes256Gcm112(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes128Gcm120(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes192Gcm120(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes256Gcm120(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes128Gcm128(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes192Gcm128(v) => v.encrypt(nonce, plaintext),
                    AesGcmVariant::Aes256Gcm128(v) => v.encrypt(nonce, plaintext),
                }
                .map_err(|_| CryptoError::EncryptionFailed)
            },
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
            AesMode::Cbc => match key.len() {
                16 => {
                    let decryptor = Decryptor::<aes::Aes128>::new_from_slices(key, iv)?;
                    decryptor
                        .decrypt_padded_vec::<Pkcs7>(data)
                        .map_err(|_| CryptoError::DecryptionFailed)
                },
                24 => {
                    let decryptor = Decryptor::<aes::Aes192>::new_from_slices(key, iv)?;
                    decryptor
                        .decrypt_padded_vec::<Pkcs7>(data)
                        .map_err(|_| CryptoError::DecryptionFailed)
                },
                32 => {
                    let decryptor = Decryptor::<aes::Aes256>::new_from_slices(key, iv)?;
                    decryptor
                        .decrypt_padded_vec::<Pkcs7>(data)
                        .map_err(|_| CryptoError::DecryptionFailed)
                },
                _ => Err(CryptoError::InvalidKey),
            },
            AesMode::Ctr { .. } => {
                // CTR decryption is the same as encryption
                self.aes_encrypt(mode, key, iv, data, additional_data)
            },
            AesMode::Gcm { tag_length } => {
                let variant = AesGcmVariant::new((key.len() * 8) as u16, tag_length, key)?;
                let nonce: &Array<_, _> =
                    &Nonce::<U12>::try_from(iv).map_err(|_| CryptoError::InvalidData)?;

                let ciphertext = Payload {
                    msg: data,
                    aad: additional_data.unwrap_or_default(),
                };

                match variant {
                    AesGcmVariant::Aes128Gcm96(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes192Gcm96(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes256Gcm96(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes128Gcm104(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes192Gcm104(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes256Gcm104(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes128Gcm112(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes192Gcm112(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes256Gcm112(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes128Gcm120(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes192Gcm120(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes256Gcm120(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes128Gcm128(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes192Gcm128(v) => v.decrypt(nonce, ciphertext),
                    AesGcmVariant::Aes256Gcm128(v) => v.decrypt(nonce, ciphertext),
                }
                .map_err(|_| CryptoError::DecryptionFailed)
            },
        }
    }

    fn aes_kw_wrap(&self, kek: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match kek.len() {
            16 => {
                let kw = KwAes128::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey)?;
                let mut buf = vec![0u8; key.len() + 8];
                let result = kw
                    .wrap_key(key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed)?;
                Ok(result.to_vec())
            },
            24 => {
                let kw = KwAes192::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey)?;
                let mut buf = vec![0u8; key.len() + 8];
                let result = kw
                    .wrap_key(key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed)?;
                Ok(result.to_vec())
            },
            32 => {
                let kw = KwAes256::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey)?;
                let mut buf = vec![0u8; key.len() + 8];
                let result = kw
                    .wrap_key(key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed)?;
                Ok(result.to_vec())
            },
            _ => Err(CryptoError::InvalidKey),
        }
    }

    fn aes_kw_unwrap(&self, kek: &[u8], wrapped_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match kek.len() {
            16 => {
                let kw = KwAes128::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey)?;
                let mut buf = vec![0u8; wrapped_key.len()];
                let result = kw
                    .unwrap_key(wrapped_key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed)?;
                Ok(result.to_vec())
            },
            24 => {
                let kw = KwAes192::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey)?;
                let mut buf = vec![0u8; wrapped_key.len()];
                let result = kw
                    .unwrap_key(wrapped_key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed)?;
                Ok(result.to_vec())
            },
            32 => {
                let kw = KwAes256::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey)?;
                let mut buf = vec![0u8; wrapped_key.len()];
                let result = kw
                    .unwrap_key(wrapped_key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed)?;
                Ok(result.to_vec())
            },
            _ => Err(CryptoError::InvalidKey),
        }
    }

    fn hkdf_derive_key(
        &self,
        key: &[u8],
        salt: &[u8],
        info: &[u8],
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        use ring::hkdf;

        let algorithm = match hash_alg {
            ShaAlgorithm::SHA1 => hkdf::HKDF_SHA1_FOR_LEGACY_USE_ONLY,
            ShaAlgorithm::SHA256 => hkdf::HKDF_SHA256,
            ShaAlgorithm::SHA384 => hkdf::HKDF_SHA384,
            ShaAlgorithm::SHA512 => hkdf::HKDF_SHA512,
            _ => return Err(CryptoError::UnsupportedAlgorithm),
        };

        let salt = hkdf::Salt::new(algorithm, salt);
        let prk = salt.extract(key);
        let info = &[info];
        let okm = prk
            .expand(info, HkdfOutput(length))
            .map_err(|_| CryptoError::DerivationFailed)?;

        let mut out = vec![0u8; length];
        okm.fill(&mut out)
            .map_err(|_| CryptoError::DerivationFailed)?;
        Ok(out)
    }

    fn pbkdf2_derive_key(
        &self,
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        length: usize,
        hash_alg: ShaAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let algorithm = match hash_alg {
            ShaAlgorithm::SHA1 => pbkdf2::PBKDF2_HMAC_SHA1,
            ShaAlgorithm::SHA256 => pbkdf2::PBKDF2_HMAC_SHA256,
            ShaAlgorithm::SHA384 => pbkdf2::PBKDF2_HMAC_SHA384,
            ShaAlgorithm::SHA512 => pbkdf2::PBKDF2_HMAC_SHA512,
            _ => return Err(CryptoError::UnsupportedAlgorithm),
        };

        let mut out = vec![0; length];
        let iterations = NonZeroU32::new(iterations).ok_or(CryptoError::InvalidData)?;
        pbkdf2::derive(algorithm, iterations, salt, password, &mut out);
        Ok(out)
    }

    fn generate_aes_key(&self, length_bits: u16) -> Result<Vec<u8>, CryptoError> {
        let length_bytes = (length_bits / 8) as usize;
        if !matches!(length_bits, 128 | 192 | 256) {
            return Err(CryptoError::InvalidLength);
        }
        Ok(random_byte_array(length_bytes))
    }

    fn generate_hmac_key(
        &self,
        hash_alg: ShaAlgorithm,
        length_bits: u16,
    ) -> Result<Vec<u8>, CryptoError> {
        let length_bytes = if length_bits == 0 {
            hash_alg.hmac_algorithm().digest_algorithm().block_len()
        } else {
            (length_bits / 8) as usize
        };

        if length_bytes > ring::digest::MAX_BLOCK_LEN {
            return Err(CryptoError::InvalidLength);
        }

        Ok(random_byte_array(length_bytes))
    }

    fn generate_ec_key(&self, curve: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let mut rng = rand::rng();

        match curve {
            EllipticCurve::P256 => {
                let key = P256SecretKey::try_from_rng(&mut rng)
                    .map_err(|_| CryptoError::OperationFailed)?;
                let pkcs8 = key
                    .to_pkcs8_der()
                    .map_err(|_| CryptoError::OperationFailed)?;
                let private_key = pkcs8.as_bytes().to_vec();
                let public_key = key.public_key().to_sec1_bytes().to_vec();
                Ok((private_key, public_key))
            },
            EllipticCurve::P384 => {
                let key = P384SecretKey::try_from_rng(&mut rng)
                    .map_err(|_| CryptoError::OperationFailed)?;
                let pkcs8 = key
                    .to_pkcs8_der()
                    .map_err(|_| CryptoError::OperationFailed)?;
                let private_key = pkcs8.as_bytes().to_vec();
                let public_key = key.public_key().to_sec1_bytes().to_vec();
                Ok((private_key, public_key))
            },
            EllipticCurve::P521 => {
                let key = P521SecretKey::try_from_rng(&mut rng)
                    .map_err(|_| CryptoError::OperationFailed)?;
                let pkcs8 = key
                    .to_pkcs8_der()
                    .map_err(|_| CryptoError::OperationFailed)?;
                let private_key = pkcs8.as_bytes().to_vec();
                let public_key = key.public_key().to_sec1_bytes().to_vec();
                Ok((private_key, public_key))
            },
        }
    }

    fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let rng = &(*SYSTEM_RANDOM);
        let pkcs8 =
            Ed25519KeyPair::generate_pkcs8(rng).map_err(|_| CryptoError::OperationFailed)?;
        let private_key = pkcs8.as_ref().to_vec();
        let key_pair =
            Ed25519KeyPair::from_pkcs8(&private_key).map_err(|_| CryptoError::OperationFailed)?;
        let public_key = key_pair.public_key().as_ref().to_vec();
        Ok((private_key, public_key))
    }

    fn generate_x25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let mut rng = rand::rng();
        let secret_key = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let private_key = secret_key.as_bytes().to_vec();
        let public_key = x25519_dalek::PublicKey::from(&secret_key)
            .as_bytes()
            .to_vec();
        Ok((private_key, public_key))
    }

    fn generate_rsa_key(
        &self,
        modulus_length: u32,
        public_exponent: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let exponent: u64 = match public_exponent {
            [0x01, 0x00, 0x01] => 65537,
            [0x03] => 3,
            bytes
                if bytes.ends_with(&[0x03]) && bytes[..bytes.len() - 1].iter().all(|&b| b == 0) =>
            {
                3
            },
            _ => return Err(CryptoError::InvalidData),
        };

        let exp = BoxedUint::from(exponent);
        let mut rng = rand::rng();
        let rsa_private_key = RsaPrivateKey::new_with_exp(&mut rng, modulus_length as usize, exp)
            .map_err(|_| CryptoError::OperationFailed)?;

        let public_key = rsa_private_key
            .to_public_key()
            .to_pkcs1_der()
            .map_err(|_| CryptoError::OperationFailed)?;
        let private_key = rsa_private_key
            .to_pkcs1_der()
            .map_err(|_| CryptoError::OperationFailed)?;

        Ok((
            private_key.as_bytes().to_vec(),
            public_key.as_bytes().to_vec(),
        ))
    }
}

// Helper struct for HKDF output length
struct HkdfOutput(usize);

impl ring::hkdf::KeyType for HkdfOutput {
    fn len(&self) -> usize {
        self.0
    }
}
