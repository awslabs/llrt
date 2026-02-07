// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

mod aes_variants;

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
use der::Encode;
use ecdsa::signature::hazmat::PrehashVerifier;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use elliptic_curve::{consts::U12, sec1::ToSec1Point, Generate};
use hkdf::Hkdf;
use hmac::{Hmac as HmacImpl, Mac};
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
    ecdsa::{
        Signature as P521Signature, SigningKey as P521SigningKey, VerifyingKey as P521VerifyingKey,
    },
    SecretKey as P521SecretKey,
};
use pbkdf2::pbkdf2;
use pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey,
};
use rsa::signature::hazmat::PrehashSigner;
use rsa::{
    pss::Pss,
    sha2::{Digest, Sha256, Sha384, Sha512},
    BoxedUint, Oaep, Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey,
};
use sha1::Sha1;

use crate::{
    hash::HashAlgorithm,
    provider::{AesMode, CryptoError, CryptoProvider, HmacProvider, SimpleDigest},
    random_byte_array,
    subtle::EllipticCurve,
};

use aes_variants::AesGcmVariant;

impl From<aes::cipher::InvalidLength> for CryptoError {
    fn from(_: aes::cipher::InvalidLength) -> Self {
        CryptoError::InvalidLength
    }
}

impl From<StreamCipherError> for CryptoError {
    fn from(_: StreamCipherError) -> Self {
        CryptoError::OperationFailed(None)
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

impl CryptoProvider for RustCryptoProvider {
    type Digest = RustDigest;
    type Hmac = RustHmac;

    fn digest(&self, algorithm: HashAlgorithm) -> Self::Digest {
        match algorithm {
            HashAlgorithm::Md5 => RustDigest::Md5(md5::Md5::new()),
            HashAlgorithm::Sha1 => RustDigest::Sha1(Sha1::new()),
            HashAlgorithm::Sha256 => RustDigest::Sha256(Sha256::new()),
            HashAlgorithm::Sha384 => RustDigest::Sha384(Sha384::new()),
            HashAlgorithm::Sha512 => RustDigest::Sha512(Sha512::new()),
        }
    }

    fn hmac(&self, algorithm: HashAlgorithm, key: &[u8]) -> Self::Hmac {
        match algorithm {
            HashAlgorithm::Md5 => panic!("HMAC-MD5 not supported"),
            HashAlgorithm::Sha1 => RustHmac::Sha1(HmacImpl::<Sha1>::new_from_slice(key).unwrap()),
            HashAlgorithm::Sha256 => {
                RustHmac::Sha256(HmacImpl::<Sha256>::new_from_slice(key).unwrap())
            },
            HashAlgorithm::Sha384 => {
                RustHmac::Sha384(HmacImpl::<Sha384>::new_from_slice(key).unwrap())
            },
            HashAlgorithm::Sha512 => {
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
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let signing_key = P256SigningKey::from(secret_key);
                let signature: p256::ecdsa::Signature = signing_key
                    .sign_prehash(digest)
                    .map_err(|_| CryptoError::SigningFailed(None))?;
                Ok(signature.to_bytes().to_vec())
            },
            EllipticCurve::P384 => {
                let secret_key = P384SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let signing_key = P384SigningKey::from(secret_key);
                let signature: p384::ecdsa::Signature = signing_key
                    .sign_prehash(digest)
                    .map_err(|_| CryptoError::SigningFailed(None))?;
                Ok(signature.to_bytes().to_vec())
            },
            EllipticCurve::P521 => {
                let secret_key = P521SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let signing_key = P521SigningKey::from(secret_key);
                let signature: p521::ecdsa::Signature = signing_key
                    .sign_prehash(digest)
                    .map_err(|_| CryptoError::SigningFailed(None))?;
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
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let sig = P256Signature::from_slice(signature)
                    .map_err(|_| CryptoError::InvalidSignature(None))?;
                Ok(verifying_key.verify_prehash(digest, &sig).is_ok())
            },
            EllipticCurve::P384 => {
                let verifying_key = P384VerifyingKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let sig = P384Signature::from_slice(signature)
                    .map_err(|_| CryptoError::InvalidSignature(None))?;
                Ok(verifying_key.verify_prehash(digest, &sig).is_ok())
            },
            EllipticCurve::P521 => {
                let verifying_key = P521VerifyingKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let sig = P521Signature::from_slice(signature)
                    .map_err(|_| CryptoError::InvalidSignature(None))?;
                Ok(verifying_key.verify_prehash(digest, &sig).is_ok())
            },
        }
    }

    fn ed25519_sign(&self, private_key_der: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(private_key_der)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let signature = signing_key
            .try_sign(data)
            .map_err(|_| CryptoError::InvalidSignature(None))?;
        Ok(signature.to_bytes().to_vec())
    }

    fn ed25519_verify(
        &self,
        public_key_bytes: &[u8],
        signature: &[u8],
        data: &[u8],
    ) -> Result<bool, CryptoError> {
        let public_key = VerifyingKey::from_bytes(
            public_key_bytes
                .try_into()
                .map_err(|_| CryptoError::InvalidKey(None))?,
        )
        .map_err(|_| CryptoError::InvalidKey(None))?;
        let signature = Signature::from_bytes(
            signature
                .try_into()
                .map_err(|_| CryptoError::InvalidSignature(None))?,
        );
        Ok(public_key.verify(data, &signature).is_ok())
    }

    fn rsa_pss_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut rng = rand::rng();
        let private_key = RsaPrivateKey::from_pkcs1_der(private_key_der)
            .map_err(|_| CryptoError::InvalidKey(None))?;

        match hash_alg {
            HashAlgorithm::Sha256 => private_key
                .sign_with_rng(&mut rng, Pss::<Sha256>::new_with_salt(salt_length), digest)
                .map_err(|_| CryptoError::SigningFailed(None)),
            HashAlgorithm::Sha384 => private_key
                .sign_with_rng(&mut rng, Pss::<Sha384>::new_with_salt(salt_length), digest)
                .map_err(|_| CryptoError::SigningFailed(None)),
            HashAlgorithm::Sha512 => private_key
                .sign_with_rng(&mut rng, Pss::<Sha512>::new_with_salt(salt_length), digest)
                .map_err(|_| CryptoError::SigningFailed(None)),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_pss_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        salt_length: usize,
        hash_alg: HashAlgorithm,
    ) -> Result<bool, CryptoError> {
        let public_key = RsaPublicKey::from_pkcs1_der(public_key_der)
            .map_err(|_| CryptoError::InvalidKey(None))?;

        match hash_alg {
            HashAlgorithm::Sha256 => Ok(public_key
                .verify(Pss::<Sha256>::new_with_salt(salt_length), digest, signature)
                .is_ok()),
            HashAlgorithm::Sha384 => Ok(public_key
                .verify(Pss::<Sha384>::new_with_salt(salt_length), digest, signature)
                .is_ok()),
            HashAlgorithm::Sha512 => Ok(public_key
                .verify(Pss::<Sha512>::new_with_salt(salt_length), digest, signature)
                .is_ok()),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_pkcs1v15_sign(
        &self,
        private_key_der: &[u8],
        digest: &[u8],
        hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut rng = rand::rng();
        let private_key = RsaPrivateKey::from_pkcs1_der(private_key_der)
            .map_err(|_| CryptoError::InvalidKey(None))?;

        match hash_alg {
            HashAlgorithm::Sha256 => private_key
                .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha256>(), digest)
                .map_err(|_| CryptoError::SigningFailed(None)),
            HashAlgorithm::Sha384 => private_key
                .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha384>(), digest)
                .map_err(|_| CryptoError::SigningFailed(None)),
            HashAlgorithm::Sha512 => private_key
                .sign_with_rng(&mut rng, Pkcs1v15Sign::new::<Sha512>(), digest)
                .map_err(|_| CryptoError::SigningFailed(None)),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_pkcs1v15_verify(
        &self,
        public_key_der: &[u8],
        signature: &[u8],
        digest: &[u8],
        hash_alg: HashAlgorithm,
    ) -> Result<bool, CryptoError> {
        let public_key = RsaPublicKey::from_pkcs1_der(public_key_der)
            .map_err(|_| CryptoError::InvalidKey(None))?;

        match hash_alg {
            HashAlgorithm::Sha256 => Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha256>(), digest, signature)
                .is_ok()),
            HashAlgorithm::Sha384 => Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha384>(), digest, signature)
                .is_ok()),
            HashAlgorithm::Sha512 => Ok(public_key
                .verify(Pkcs1v15Sign::new::<Sha512>(), digest, signature)
                .is_ok()),
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_oaep_encrypt(
        &self,
        public_key_der: &[u8],
        data: &[u8],
        hash_alg: HashAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut rng = rand::rng();
        let public_key = RsaPublicKey::from_pkcs1_der(public_key_der)
            .map_err(|_| CryptoError::InvalidKey(None))?;

        match hash_alg {
            HashAlgorithm::Sha1 => {
                let mut padding = Oaep::<Sha1>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed(None))
            },
            HashAlgorithm::Sha256 => {
                let mut padding = Oaep::<Sha256>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed(None))
            },
            HashAlgorithm::Sha384 => {
                let mut padding = Oaep::<Sha384>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed(None))
            },
            HashAlgorithm::Sha512 => {
                let mut padding = Oaep::<Sha512>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                public_key
                    .encrypt(&mut rng, padding, data)
                    .map_err(|_| CryptoError::EncryptionFailed(None))
            },
            _ => Err(CryptoError::UnsupportedAlgorithm),
        }
    }

    fn rsa_oaep_decrypt(
        &self,
        private_key_der: &[u8],
        data: &[u8],
        hash_alg: HashAlgorithm,
        label: Option<&[u8]>,
    ) -> Result<Vec<u8>, CryptoError> {
        let private_key = RsaPrivateKey::from_pkcs1_der(private_key_der)
            .map_err(|_| CryptoError::InvalidKey(None))?;

        match hash_alg {
            HashAlgorithm::Sha1 => {
                let mut padding = Oaep::<Sha1>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed(None))
            },
            HashAlgorithm::Sha256 => {
                let mut padding = Oaep::<Sha256>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed(None))
            },
            HashAlgorithm::Sha384 => {
                let mut padding = Oaep::<Sha384>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed(None))
            },
            HashAlgorithm::Sha512 => {
                let mut padding = Oaep::<Sha512>::new();
                if let Some(l) = label {
                    if !l.is_empty() {
                        padding.label = Some(l.into());
                    }
                }
                private_key
                    .decrypt(padding, data)
                    .map_err(|_| CryptoError::DecryptionFailed(None))
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
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let public_key = p256::PublicKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let shared_secret = p256::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );
                Ok(shared_secret.raw_secret_bytes().to_vec())
            },
            EllipticCurve::P384 => {
                let secret_key = P384SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let public_key = p384::PublicKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let shared_secret = p384::elliptic_curve::ecdh::diffie_hellman(
                    secret_key.to_nonzero_scalar(),
                    public_key.as_affine(),
                );
                Ok(shared_secret.raw_secret_bytes().to_vec())
            },
            EllipticCurve::P521 => {
                let secret_key = P521SecretKey::from_pkcs8_der(private_key_der)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let public_key = p521::PublicKey::from_sec1_bytes(public_key_sec1)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
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
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let public_array: [u8; 32] = public_key
            .try_into()
            .map_err(|_| CryptoError::InvalidKey(None))?;

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
                _ => Err(CryptoError::InvalidKey(None)),
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
                    _ => return Err(CryptoError::InvalidKey(None)),
                }
                Ok(ciphertext)
            },
            AesMode::Gcm { tag_length } => {
                let variant = AesGcmVariant::new((key.len() * 8) as u16, tag_length, key)?;
                let nonce: &Array<_, _> =
                    &Nonce::<U12>::try_from(iv).map_err(|_| CryptoError::InvalidData(None))?;

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
                .map_err(|_| CryptoError::EncryptionFailed(None))
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
                        .map_err(|_| CryptoError::DecryptionFailed(None))
                },
                24 => {
                    let decryptor = Decryptor::<aes::Aes192>::new_from_slices(key, iv)?;
                    decryptor
                        .decrypt_padded_vec::<Pkcs7>(data)
                        .map_err(|_| CryptoError::DecryptionFailed(None))
                },
                32 => {
                    let decryptor = Decryptor::<aes::Aes256>::new_from_slices(key, iv)?;
                    decryptor
                        .decrypt_padded_vec::<Pkcs7>(data)
                        .map_err(|_| CryptoError::DecryptionFailed(None))
                },
                _ => Err(CryptoError::InvalidKey(None)),
            },
            AesMode::Ctr { .. } => {
                // CTR decryption is the same as encryption
                self.aes_encrypt(mode, key, iv, data, additional_data)
            },
            AesMode::Gcm { tag_length } => {
                let variant = AesGcmVariant::new((key.len() * 8) as u16, tag_length, key)?;
                let nonce: &Array<_, _> =
                    &Nonce::<U12>::try_from(iv).map_err(|_| CryptoError::InvalidData(None))?;

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
                .map_err(|_| CryptoError::DecryptionFailed(None))
            },
        }
    }

    fn aes_kw_wrap(&self, kek: &[u8], key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match kek.len() {
            16 => {
                let kw =
                    KwAes128::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey(None))?;
                let mut buf = vec![0u8; key.len() + 8];
                let result = kw
                    .wrap_key(key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                Ok(result.to_vec())
            },
            24 => {
                let kw =
                    KwAes192::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey(None))?;
                let mut buf = vec![0u8; key.len() + 8];
                let result = kw
                    .wrap_key(key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                Ok(result.to_vec())
            },
            32 => {
                let kw =
                    KwAes256::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey(None))?;
                let mut buf = vec![0u8; key.len() + 8];
                let result = kw
                    .wrap_key(key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                Ok(result.to_vec())
            },
            _ => Err(CryptoError::InvalidKey(None)),
        }
    }

    fn aes_kw_unwrap(&self, kek: &[u8], wrapped_key: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match kek.len() {
            16 => {
                let kw =
                    KwAes128::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey(None))?;
                let mut buf = vec![0u8; wrapped_key.len()];
                let result = kw
                    .unwrap_key(wrapped_key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                Ok(result.to_vec())
            },
            24 => {
                let kw =
                    KwAes192::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey(None))?;
                let mut buf = vec![0u8; wrapped_key.len()];
                let result = kw
                    .unwrap_key(wrapped_key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                Ok(result.to_vec())
            },
            32 => {
                let kw =
                    KwAes256::new_from_slice(kek).map_err(|_| CryptoError::InvalidKey(None))?;
                let mut buf = vec![0u8; wrapped_key.len()];
                let result = kw
                    .unwrap_key(wrapped_key, &mut buf)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                Ok(result.to_vec())
            },
            _ => Err(CryptoError::InvalidKey(None)),
        }
    }

    fn hkdf_derive_key(
        &self,
        key: &[u8],
        salt: &[u8],
        info: &[u8],
        length: usize,
        hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut out = vec![0u8; length];

        match hash_alg {
            HashAlgorithm::Sha1 => {
                let prk = Hkdf::<Sha1>::new(Some(salt), key);
                prk.expand(info, &mut out)
            },
            HashAlgorithm::Sha256 => {
                let prk = Hkdf::<Sha256>::new(Some(salt), key);
                prk.expand(info, &mut out)
            },
            HashAlgorithm::Sha384 => {
                let prk = Hkdf::<Sha384>::new(Some(salt), key);
                prk.expand(info, &mut out)
            },
            HashAlgorithm::Sha512 => {
                let prk = Hkdf::<Sha512>::new(Some(salt), key);
                prk.expand(info, &mut out)
            },
            _ => return Err(CryptoError::UnsupportedAlgorithm),
        }
        .map_err(|_| CryptoError::DerivationFailed(None))?;
        Ok(out)
    }

    fn pbkdf2_derive_key(
        &self,
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        length: usize,
        hash_alg: HashAlgorithm,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut out = vec![0; length];
        let iterations = NonZeroU32::new(iterations).ok_or(CryptoError::InvalidData(None))?;
        match hash_alg {
            HashAlgorithm::Sha1 => {
                pbkdf2::<HmacImpl<Sha1>>(password, salt, iterations.get(), &mut out)
            },
            HashAlgorithm::Sha256 => {
                pbkdf2::<HmacImpl<Sha256>>(password, salt, iterations.get(), &mut out)
            },
            HashAlgorithm::Sha384 => {
                pbkdf2::<HmacImpl<Sha384>>(password, salt, iterations.get(), &mut out)
            },
            HashAlgorithm::Sha512 => {
                pbkdf2::<HmacImpl<Sha512>>(password, salt, iterations.get(), &mut out)
            },
            _ => return Err(CryptoError::UnsupportedAlgorithm),
        }
        .map_err(|_| CryptoError::InvalidLength)?;
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
        hash_alg: HashAlgorithm,
        length_bits: u16,
    ) -> Result<Vec<u8>, CryptoError> {
        let length_bytes = if length_bits == 0 {
            hash_alg.block_len()
        } else {
            (length_bits / 8) as usize
        };

        if length_bytes > 128 {
            return Err(CryptoError::InvalidLength);
        }

        Ok(random_byte_array(length_bytes))
    }

    fn generate_ec_key(&self, curve: EllipticCurve) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let mut rng = rand::rng();

        match curve {
            EllipticCurve::P256 => {
                let key = P256SecretKey::try_generate_from_rng(&mut rng)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                let pkcs8 = key
                    .to_pkcs8_der()
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                let private_key = pkcs8.as_bytes().to_vec();
                let public_key = key.public_key().to_sec1_bytes().to_vec();
                Ok((private_key, public_key))
            },
            EllipticCurve::P384 => {
                let key = P384SecretKey::try_generate_from_rng(&mut rng)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                let pkcs8 = key
                    .to_pkcs8_der()
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                let private_key = pkcs8.as_bytes().to_vec();
                let public_key = key.public_key().to_sec1_bytes().to_vec();
                Ok((private_key, public_key))
            },
            EllipticCurve::P521 => {
                let key = P521SecretKey::try_generate_from_rng(&mut rng)
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                let pkcs8 = key
                    .to_pkcs8_der()
                    .map_err(|_| CryptoError::OperationFailed(None))?;
                let private_key = pkcs8.as_bytes().to_vec();
                let public_key = key.public_key().to_sec1_bytes().to_vec();
                Ok((private_key, public_key))
            },
        }
    }

    fn generate_ed25519_key(&self) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        let mut rng = rand::rng();
        let private_key = ed25519_dalek::SigningKey::generate(&mut rng)
            .to_pkcs8_der()
            .map_err(|_| CryptoError::OperationFailed(None))?
            .as_bytes()
            .to_vec();
        let signing_key = ed25519_dalek::SigningKey::from_pkcs8_der(&private_key)
            .map_err(|_| CryptoError::OperationFailed(None))?;
        let public_key = signing_key.verifying_key().to_bytes().to_vec();
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
            _ => return Err(CryptoError::InvalidData(None)),
        };

        let exp = BoxedUint::from(exponent);
        let mut rng = rand::rng();
        let rsa_private_key = RsaPrivateKey::new_with_exp(&mut rng, modulus_length as usize, exp)
            .map_err(|_| CryptoError::OperationFailed(None))?;

        let public_key = rsa_private_key
            .to_public_key()
            .to_pkcs1_der()
            .map_err(|_| CryptoError::OperationFailed(None))?;
        let private_key = rsa_private_key
            .to_pkcs1_der()
            .map_err(|_| CryptoError::OperationFailed(None))?;

        Ok((
            private_key.as_bytes().to_vec(),
            public_key.as_bytes().to_vec(),
        ))
    }

    fn import_rsa_public_key_pkcs1(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        use der::Decode;
        let public_key =
            rsa::pkcs1::RsaPublicKey::from_der(der).map_err(|_| CryptoError::InvalidKey(None))?;
        let modulus_length = public_key.modulus.as_bytes().len() * 8;
        let public_exponent = public_key.public_exponent.as_bytes().to_vec();
        let key_data = public_key
            .to_der()
            .map_err(|_| CryptoError::InvalidKey(None))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length: modulus_length as u32,
            public_exponent,
            is_private: false,
        })
    }

    fn import_rsa_private_key_pkcs1(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        use der::Decode;
        let private_key =
            rsa::pkcs1::RsaPrivateKey::from_der(der).map_err(|_| CryptoError::InvalidKey(None))?;
        let modulus_length = private_key.modulus.as_bytes().len() * 8;
        let public_exponent = private_key.public_exponent.as_bytes().to_vec();
        let key_data = private_key
            .to_der()
            .map_err(|_| CryptoError::InvalidKey(None))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length: modulus_length as u32,
            public_exponent,
            is_private: true,
        })
    }

    fn import_rsa_public_key_spki(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        use der::Decode;
        let spki = spki::SubjectPublicKeyInfoRef::try_from(der)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let public_key = rsa::pkcs1::RsaPublicKey::from_der(spki.subject_public_key.raw_bytes())
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let modulus_length = public_key.modulus.as_bytes().len() * 8;
        let public_exponent = public_key.public_exponent.as_bytes().to_vec();
        let key_data = public_key
            .to_der()
            .map_err(|_| CryptoError::InvalidKey(None))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length: modulus_length as u32,
            public_exponent,
            is_private: false,
        })
    }

    fn import_rsa_private_key_pkcs8(
        &self,
        der: &[u8],
    ) -> Result<super::RsaImportResult, CryptoError> {
        use der::Decode;
        let pk_info =
            pkcs8::PrivateKeyInfoRef::from_der(der).map_err(|_| CryptoError::InvalidKey(None))?;
        let private_key = rsa::pkcs1::RsaPrivateKey::from_der(pk_info.private_key.as_bytes())
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let modulus_length = private_key.modulus.as_bytes().len() * 8;
        let public_exponent = private_key.public_exponent.as_bytes().to_vec();
        let key_data = pk_info
            .private_key
            .to_der()
            .map_err(|_| CryptoError::InvalidKey(None))?;
        Ok(super::RsaImportResult {
            key_data,
            modulus_length: modulus_length as u32,
            public_exponent,
            is_private: true,
        })
    }

    fn export_rsa_public_key_pkcs1(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // key_data is already PKCS1 DER
        Ok(key_data.to_vec())
    }

    fn export_rsa_public_key_spki(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        use der::{Decode, Encode};
        let public_key = rsa::pkcs1::RsaPublicKey::from_der(key_data)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let spki = spki::SubjectPublicKeyInfo {
            algorithm: spki::AlgorithmIdentifier::<der::asn1::Any> {
                oid: const_oid::db::rfc5912::RSA_ENCRYPTION,
                parameters: Some(der::asn1::Null.into()),
            },
            subject_public_key: spki::der::asn1::BitString::from_bytes(
                &public_key
                    .to_der()
                    .map_err(|_| CryptoError::InvalidKey(None))?,
            )
            .map_err(|_| CryptoError::InvalidKey(None))?,
        };
        spki.to_der().map_err(|_| CryptoError::InvalidKey(None))
    }

    fn export_rsa_private_key_pkcs8(&self, key_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let private_key =
            RsaPrivateKey::from_pkcs1_der(key_data).map_err(|_| CryptoError::InvalidKey(None))?;
        private_key
            .to_pkcs8_der()
            .map(|doc| doc.as_bytes().to_vec())
            .map_err(|_| CryptoError::InvalidKey(None))
    }

    fn import_ec_public_key_sec1(
        &self,
        data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        Ok(super::EcImportResult {
            key_data: data.to_vec(),
            is_private: false,
        })
    }

    fn import_ec_public_key_spki(&self, der: &[u8]) -> Result<super::EcImportResult, CryptoError> {
        let spki = spki::SubjectPublicKeyInfoRef::try_from(der)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        Ok(super::EcImportResult {
            key_data: spki.subject_public_key.raw_bytes().to_vec(),
            is_private: false,
        })
    }

    fn import_ec_private_key_pkcs8(
        &self,
        der: &[u8],
    ) -> Result<super::EcImportResult, CryptoError> {
        Ok(super::EcImportResult {
            key_data: der.to_vec(),
            is_private: true,
        })
    }

    fn import_ec_private_key_sec1(
        &self,
        data: &[u8],
        curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        // Convert SEC1 private key to PKCS8
        let pkcs8_der = match curve {
            EllipticCurve::P256 => {
                let key =
                    P256SecretKey::from_slice(data).map_err(|_| CryptoError::InvalidKey(None))?;
                key.to_pkcs8_der()
                    .map_err(|_| CryptoError::InvalidKey(None))?
                    .as_bytes()
                    .to_vec()
            },
            EllipticCurve::P384 => {
                let key =
                    P384SecretKey::from_slice(data).map_err(|_| CryptoError::InvalidKey(None))?;
                key.to_pkcs8_der()
                    .map_err(|_| CryptoError::InvalidKey(None))?
                    .as_bytes()
                    .to_vec()
            },
            EllipticCurve::P521 => {
                let key =
                    P521SecretKey::from_slice(data).map_err(|_| CryptoError::InvalidKey(None))?;
                key.to_pkcs8_der()
                    .map_err(|_| CryptoError::InvalidKey(None))?
                    .as_bytes()
                    .to_vec()
            },
        };
        Ok(super::EcImportResult {
            key_data: pkcs8_der,
            is_private: true,
        })
    }

    fn export_ec_public_key_sec1(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
        is_private: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        if is_private {
            // Extract public key from PKCS8 private key
            match curve {
                EllipticCurve::P256 => {
                    let key = P256SecretKey::from_pkcs8_der(key_data)
                        .map_err(|_| CryptoError::InvalidKey(None))?;
                    Ok(key.public_key().to_sec1_point(false).as_bytes().to_vec())
                },
                EllipticCurve::P384 => {
                    let key = P384SecretKey::from_pkcs8_der(key_data)
                        .map_err(|_| CryptoError::InvalidKey(None))?;
                    Ok(key.public_key().to_sec1_point(false).as_bytes().to_vec())
                },
                EllipticCurve::P521 => {
                    let key = P521SecretKey::from_pkcs8_der(key_data)
                        .map_err(|_| CryptoError::InvalidKey(None))?;
                    Ok(key.public_key().to_sec1_point(false).as_bytes().to_vec())
                },
            }
        } else {
            // key_data is already SEC1 encoded
            Ok(key_data.to_vec())
        }
    }

    fn export_ec_public_key_spki(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError> {
        use der::Encode;
        use elliptic_curve::pkcs8::AssociatedOid;
        let curve_oid = match curve {
            EllipticCurve::P256 => p256::NistP256::OID,
            EllipticCurve::P384 => p384::NistP384::OID,
            EllipticCurve::P521 => p521::NistP521::OID,
        };
        let spki = spki::SubjectPublicKeyInfo {
            algorithm: spki::AlgorithmIdentifier::<der::asn1::ObjectIdentifier> {
                oid: elliptic_curve::ALGORITHM_OID,
                parameters: Some(curve_oid),
            },
            subject_public_key: spki::der::asn1::BitString::from_bytes(key_data)
                .map_err(|_| CryptoError::InvalidKey(None))?,
        };
        spki.to_der().map_err(|_| CryptoError::InvalidKey(None))
    }

    fn export_ec_private_key_pkcs8(
        &self,
        key_data: &[u8],
        _curve: EllipticCurve,
    ) -> Result<Vec<u8>, CryptoError> {
        // key_data is already PKCS8
        Ok(key_data.to_vec())
    }

    fn import_okp_public_key_raw(
        &self,
        data: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        if data.len() != 32 {
            return Err(CryptoError::InvalidKey(None));
        }
        Ok(super::OkpImportResult {
            key_data: data.to_vec(),
            is_private: false,
        })
    }

    fn import_okp_public_key_spki(
        &self,
        der: &[u8],
        _expected_oid: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        let spki = spki::SubjectPublicKeyInfoRef::try_from(der)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        Ok(super::OkpImportResult {
            key_data: spki.subject_public_key.raw_bytes().to_vec(),
            is_private: false,
        })
    }

    fn import_okp_private_key_pkcs8(
        &self,
        der: &[u8],
        _expected_oid: &[u8],
    ) -> Result<super::OkpImportResult, CryptoError> {
        Ok(super::OkpImportResult {
            key_data: der.to_vec(),
            is_private: true,
        })
    }

    fn export_okp_public_key_raw(
        &self,
        key_data: &[u8],
        is_private: bool,
    ) -> Result<Vec<u8>, CryptoError> {
        if is_private {
            // Extract public key from PKCS8 - for X25519/Ed25519
            use der::Decode;
            let pk_info = pkcs8::PrivateKeyInfoRef::from_der(key_data)
                .map_err(|_| CryptoError::InvalidKey(None))?;
            // The private key is wrapped in an OCTET STRING, skip the tag+length (2 bytes)
            let private_key_bytes = pk_info.private_key.as_bytes();
            let seed = if private_key_bytes.len() > 2 && private_key_bytes[0] == 0x04 {
                &private_key_bytes[2..]
            } else {
                private_key_bytes
            };
            let bytes: [u8; 32] = seed.try_into().map_err(|_| CryptoError::InvalidKey(None))?;
            let secret = x25519_dalek::StaticSecret::from(bytes);
            let public = x25519_dalek::PublicKey::from(&secret);
            Ok(public.as_bytes().to_vec())
        } else {
            Ok(key_data.to_vec())
        }
    }

    fn export_okp_public_key_spki(
        &self,
        key_data: &[u8],
        oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        use der::Encode;
        let oid = const_oid::ObjectIdentifier::from_bytes(oid)
            .map_err(|_| CryptoError::InvalidKey(None))?;
        let spki = spki::SubjectPublicKeyInfo {
            algorithm: spki::AlgorithmIdentifierOwned {
                oid,
                parameters: None,
            },
            subject_public_key: spki::der::asn1::BitString::from_bytes(key_data)
                .map_err(|_| CryptoError::InvalidKey(None))?,
        };
        spki.to_der().map_err(|_| CryptoError::InvalidKey(None))
    }

    fn export_okp_private_key_pkcs8(
        &self,
        key_data: &[u8],
        _oid: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        // key_data is already PKCS8
        Ok(key_data.to_vec())
    }

    fn import_rsa_jwk(
        &self,
        jwk: super::RsaJwkImport<'_>,
    ) -> Result<super::RsaImportResult, CryptoError> {
        use der::{asn1::UintRef, Encode};
        let modulus = UintRef::new(jwk.n).map_err(|_| CryptoError::InvalidKey(None))?;
        let public_exponent = UintRef::new(jwk.e).map_err(|_| CryptoError::InvalidKey(None))?;
        let modulus_length = (modulus.as_bytes().len() * 8) as u32;
        let pub_exp_bytes = public_exponent.as_bytes().to_vec();

        if let (Some(d), Some(p), Some(q), Some(dp), Some(dq), Some(qi)) =
            (jwk.d, jwk.p, jwk.q, jwk.dp, jwk.dq, jwk.qi)
        {
            let private_key = rsa::pkcs1::RsaPrivateKey {
                modulus,
                public_exponent,
                private_exponent: UintRef::new(d).map_err(|_| CryptoError::InvalidKey(None))?,
                prime1: UintRef::new(p).map_err(|_| CryptoError::InvalidKey(None))?,
                prime2: UintRef::new(q).map_err(|_| CryptoError::InvalidKey(None))?,
                exponent1: UintRef::new(dp).map_err(|_| CryptoError::InvalidKey(None))?,
                exponent2: UintRef::new(dq).map_err(|_| CryptoError::InvalidKey(None))?,
                coefficient: UintRef::new(qi).map_err(|_| CryptoError::InvalidKey(None))?,
                other_prime_infos: None,
            };
            Ok(super::RsaImportResult {
                key_data: private_key
                    .to_der()
                    .map_err(|_| CryptoError::InvalidKey(None))?,
                modulus_length,
                public_exponent: pub_exp_bytes,
                is_private: true,
            })
        } else {
            let public_key = rsa::pkcs1::RsaPublicKey {
                modulus,
                public_exponent,
            };
            Ok(super::RsaImportResult {
                key_data: public_key
                    .to_der()
                    .map_err(|_| CryptoError::InvalidKey(None))?,
                modulus_length,
                public_exponent: pub_exp_bytes,
                is_private: false,
            })
        }
    }

    fn export_rsa_jwk(
        &self,
        key_data: &[u8],
        is_private: bool,
    ) -> Result<super::RsaJwkExport, CryptoError> {
        use der::Decode;
        if is_private {
            let key = rsa::pkcs1::RsaPrivateKey::from_der(key_data)
                .map_err(|_| CryptoError::InvalidKey(None))?;
            Ok(super::RsaJwkExport {
                n: key.modulus.as_bytes().to_vec(),
                e: key.public_exponent.as_bytes().to_vec(),
                d: Some(key.private_exponent.as_bytes().to_vec()),
                p: Some(key.prime1.as_bytes().to_vec()),
                q: Some(key.prime2.as_bytes().to_vec()),
                dp: Some(key.exponent1.as_bytes().to_vec()),
                dq: Some(key.exponent2.as_bytes().to_vec()),
                qi: Some(key.coefficient.as_bytes().to_vec()),
            })
        } else {
            let key = rsa::pkcs1::RsaPublicKey::from_der(key_data)
                .map_err(|_| CryptoError::InvalidKey(None))?;
            Ok(super::RsaJwkExport {
                n: key.modulus.as_bytes().to_vec(),
                e: key.public_exponent.as_bytes().to_vec(),
                d: None,
                p: None,
                q: None,
                dp: None,
                dq: None,
                qi: None,
            })
        }
    }

    fn import_ec_jwk(
        &self,
        jwk: super::EcJwkImport<'_>,
        curve: EllipticCurve,
    ) -> Result<super::EcImportResult, CryptoError> {
        if let Some(d) = jwk.d {
            // Private key - convert to PKCS8
            let pkcs8_der = match curve {
                EllipticCurve::P256 => {
                    let key =
                        P256SecretKey::from_slice(d).map_err(|_| CryptoError::InvalidKey(None))?;
                    key.to_pkcs8_der()
                        .map_err(|_| CryptoError::InvalidKey(None))?
                        .as_bytes()
                        .to_vec()
                },
                EllipticCurve::P384 => {
                    let key =
                        P384SecretKey::from_slice(d).map_err(|_| CryptoError::InvalidKey(None))?;
                    key.to_pkcs8_der()
                        .map_err(|_| CryptoError::InvalidKey(None))?
                        .as_bytes()
                        .to_vec()
                },
                EllipticCurve::P521 => {
                    let key =
                        P521SecretKey::from_slice(d).map_err(|_| CryptoError::InvalidKey(None))?;
                    key.to_pkcs8_der()
                        .map_err(|_| CryptoError::InvalidKey(None))?
                        .as_bytes()
                        .to_vec()
                },
            };
            Ok(super::EcImportResult {
                key_data: pkcs8_der,
                is_private: true,
            })
        } else {
            // Public key - encode as SEC1 uncompressed point
            let mut point = Vec::with_capacity(1 + jwk.x.len() + jwk.y.len());
            point.push(0x04); // uncompressed
            point.extend_from_slice(jwk.x);
            point.extend_from_slice(jwk.y);
            Ok(super::EcImportResult {
                key_data: point,
                is_private: false,
            })
        }
    }

    fn export_ec_jwk(
        &self,
        key_data: &[u8],
        curve: EllipticCurve,
        is_private: bool,
    ) -> Result<super::EcJwkExport, CryptoError> {
        let coord_len = match curve {
            EllipticCurve::P256 => 32,
            EllipticCurve::P384 => 48,
            EllipticCurve::P521 => 66,
        };
        if is_private {
            // key_data is PKCS8 - use elliptic_curve's SecretKey to parse it
            let (x, y, d) = match curve {
                EllipticCurve::P256 => {
                    let sk = P256SecretKey::from_pkcs8_der(key_data)
                        .map_err(|_| CryptoError::InvalidKey(None))?;
                    let pk = sk.public_key();
                    let pt = pk.to_sec1_point(false);
                    (
                        pt.x().unwrap().to_vec(),
                        pt.y().unwrap().to_vec(),
                        sk.to_bytes().to_vec(),
                    )
                },
                EllipticCurve::P384 => {
                    let sk = P384SecretKey::from_pkcs8_der(key_data)
                        .map_err(|_| CryptoError::InvalidKey(None))?;
                    let pk = sk.public_key();
                    let pt = pk.to_sec1_point(false);
                    (
                        pt.x().unwrap().to_vec(),
                        pt.y().unwrap().to_vec(),
                        sk.to_bytes().to_vec(),
                    )
                },
                EllipticCurve::P521 => {
                    let sk = P521SecretKey::from_pkcs8_der(key_data)
                        .map_err(|_| CryptoError::InvalidKey(None))?;
                    let pk = sk.public_key();
                    let pt = pk.to_sec1_point(false);
                    (
                        pt.x().unwrap().to_vec(),
                        pt.y().unwrap().to_vec(),
                        sk.to_bytes().to_vec(),
                    )
                },
            };
            Ok(super::EcJwkExport { x, y, d: Some(d) })
        } else {
            // key_data is SEC1 uncompressed point (0x04 || x || y)
            if key_data.len() != 1 + 2 * coord_len || key_data[0] != 0x04 {
                return Err(CryptoError::InvalidKey(None));
            }
            let x = key_data[1..1 + coord_len].to_vec();
            let y = key_data[1 + coord_len..].to_vec();
            Ok(super::EcJwkExport { x, y, d: None })
        }
    }

    fn import_okp_jwk(
        &self,
        jwk: super::OkpJwkImport<'_>,
        is_ed25519: bool,
    ) -> Result<super::OkpImportResult, CryptoError> {
        if let Some(d) = jwk.d {
            // Private key - for Ed25519 we need PKCS8, for X25519 we store raw
            if is_ed25519 {
                // Ed25519: construct PKCS8 from raw private key
                use der::{
                    asn1::{BitStringRef, OctetStringRef},
                    Encode,
                };
                let pk_info = pkcs8::PrivateKeyInfoRef {
                    algorithm: spki::AlgorithmIdentifier {
                        oid: const_oid::db::rfc8410::ID_ED_25519,
                        parameters: None,
                    },
                    private_key: OctetStringRef::new(d)
                        .map_err(|_| CryptoError::InvalidKey(None))?,
                    public_key: Some(
                        BitStringRef::from_bytes(jwk.x)
                            .map_err(|_| CryptoError::InvalidKey(None))?,
                    ),
                };
                let der = pk_info
                    .to_der()
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                Ok(super::OkpImportResult {
                    key_data: der,
                    is_private: true,
                })
            } else {
                // X25519: store raw 32-byte secret
                Ok(super::OkpImportResult {
                    key_data: d.to_vec(),
                    is_private: true,
                })
            }
        } else {
            // Public key - store raw bytes
            Ok(super::OkpImportResult {
                key_data: jwk.x.to_vec(),
                is_private: false,
            })
        }
    }

    fn export_okp_jwk(
        &self,
        key_data: &[u8],
        is_private: bool,
        is_ed25519: bool,
    ) -> Result<super::OkpJwkExport, CryptoError> {
        if is_private {
            if is_ed25519 {
                // Ed25519: key_data is PKCS8
                use der::Decode;
                let pk_info = pkcs8::PrivateKeyInfoRef::from_der(key_data)
                    .map_err(|_| CryptoError::InvalidKey(None))?;
                let d = pk_info.private_key.as_bytes();
                let x = pk_info
                    .public_key
                    .ok_or(CryptoError::InvalidKey(None))?
                    .raw_bytes()
                    .to_vec();
                Ok(super::OkpJwkExport {
                    x,
                    d: Some(d.to_vec()),
                })
            } else {
                // X25519: key_data is raw 32-byte secret
                let secret = x25519_dalek::StaticSecret::from(
                    <[u8; 32]>::try_from(key_data).map_err(|_| CryptoError::InvalidKey(None))?,
                );
                let public = x25519_dalek::PublicKey::from(&secret);
                Ok(super::OkpJwkExport {
                    x: public.as_bytes().to_vec(),
                    d: Some(key_data.to_vec()),
                })
            }
        } else {
            // Public key - key_data is raw bytes
            Ok(super::OkpJwkExport {
                x: key_data.to_vec(),
                d: None,
            })
        }
    }
}
