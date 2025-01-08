// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod crypto_key;
mod decrypt;
mod derive;
mod derive_algorithm;
mod digest;
mod encrypt;
mod encryption_algorithm;
mod export_key;
mod generate_key;
mod import_key;
mod key_algorithm;
mod sign;
mod sign_algorithm;
mod verify;

pub use crypto_key::CryptoKey;
pub use decrypt::subtle_decrypt;
pub use derive::subtle_derive_bits;
pub use derive::subtle_derive_key;
pub use digest::subtle_digest;
pub use encrypt::subtle_encrypt;
pub use export_key::subtle_export_key;
pub use generate_key::subtle_generate_key;
pub use import_key::subtle_import_key;
use key_algorithm::KeyAlgorithm;
use ring::digest::Digest;
use rsa::pkcs1::DecodeRsaPrivateKey;
pub use sign::subtle_sign;
pub use verify::subtle_verify;

use aes::{
    cipher::{
        block_padding::{Pkcs7, UnpadError},
        consts::{U13, U14, U15, U16},
        typenum::U12,
        BlockDecryptMut, BlockEncryptMut, InvalidLength, KeyIvInit, StreamCipher,
        StreamCipherError,
    },
    Aes128, Aes192, Aes256,
};
use aes_gcm::{
    aead::{Aead, Payload},
    AesGcm, KeyInit,
};
use ctr::{Ctr128BE, Ctr32BE, Ctr64BE};
use llrt_utils::{object::ObjectExt, result::ResultExt, str_enum};
use rquickjs::{Ctx, Exception, Object, Result, Value};
use rsa::{Oaep, RsaPrivateKey};

use crate::sha_hash::ShaAlgorithm;

pub enum AesCbcEncVariant {
    Aes128(cbc::Encryptor<aes::Aes128>),
    Aes192(cbc::Encryptor<aes::Aes192>),
    Aes256(cbc::Encryptor<aes::Aes256>),
}

impl AesCbcEncVariant {
    pub fn new(key_len: u16, key: &[u8], iv: &[u8]) -> std::result::Result<Self, InvalidLength> {
        let variant: AesCbcEncVariant = match key_len {
            128 => Self::Aes128(cbc::Encryptor::new_from_slices(key, iv)?),
            192 => Self::Aes192(cbc::Encryptor::new_from_slices(key, iv)?),
            256 => Self::Aes256(cbc::Encryptor::new_from_slices(key, iv)?),
            _ => return Err(InvalidLength),
        };

        Ok(variant)
    }

    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        match self {
            Self::Aes128(v) => v.clone().encrypt_padded_vec_mut::<Pkcs7>(data),
            Self::Aes192(v) => v.clone().encrypt_padded_vec_mut::<Pkcs7>(data),
            Self::Aes256(v) => v.clone().encrypt_padded_vec_mut::<Pkcs7>(data),
        }
    }
}

pub enum AesCbcDecVariant {
    Aes128(cbc::Decryptor<aes::Aes128>),
    Aes192(cbc::Decryptor<aes::Aes192>),
    Aes256(cbc::Decryptor<aes::Aes256>),
}

impl AesCbcDecVariant {
    pub fn new(key_len: u16, key: &[u8], iv: &[u8]) -> std::result::Result<Self, InvalidLength> {
        let variant: AesCbcDecVariant = match key_len {
            128 => Self::Aes128(cbc::Decryptor::new_from_slices(key, iv)?),
            192 => Self::Aes192(cbc::Decryptor::new_from_slices(key, iv)?),
            256 => Self::Aes256(cbc::Decryptor::new_from_slices(key, iv)?),
            _ => return Err(InvalidLength),
        };

        Ok(variant)
    }

    pub fn decrypt(&self, data: &[u8]) -> std::result::Result<Vec<u8>, UnpadError> {
        Ok(match self {
            Self::Aes128(v) => v.clone().decrypt_padded_vec_mut::<Pkcs7>(data)?,
            Self::Aes192(v) => v.clone().decrypt_padded_vec_mut::<Pkcs7>(data)?,
            Self::Aes256(v) => v.clone().decrypt_padded_vec_mut::<Pkcs7>(data)?,
        })
    }
}

pub enum AesCtrVariant {
    Aes128Ctr32(Ctr32BE<aes::Aes128>),
    Aes128Ctr64(Ctr64BE<aes::Aes128>),
    Aes128Ctr128(Ctr128BE<aes::Aes128>),
    Aes192Ctr32(Ctr32BE<aes::Aes192>),
    Aes192Ctr64(Ctr64BE<aes::Aes192>),
    Aes192Ctr128(Ctr128BE<aes::Aes192>),
    Aes256Ctr32(Ctr32BE<aes::Aes256>),
    Aes256Ctr64(Ctr64BE<aes::Aes256>),
    Aes256Ctr128(Ctr128BE<aes::Aes256>),
}

impl AesCtrVariant {
    pub fn new(
        key_len: u16,
        encryption_length: u32,
        key: &[u8],
        counter: &[u8],
    ) -> std::result::Result<Self, InvalidLength> {
        let variant: AesCtrVariant = match (key_len, encryption_length) {
            (128, 32) => Self::Aes128Ctr32(Ctr32BE::new_from_slices(key, counter)?),
            (128, 64) => Self::Aes128Ctr64(Ctr64BE::new_from_slices(key, counter)?),
            (128, 128) => Self::Aes128Ctr128(Ctr128BE::new_from_slices(key, counter)?),
            (192, 32) => Self::Aes192Ctr32(Ctr32BE::new_from_slices(key, counter)?),
            (192, 64) => Self::Aes192Ctr64(Ctr64BE::new_from_slices(key, counter)?),
            (192, 128) => Self::Aes192Ctr128(Ctr128BE::new_from_slices(key, counter)?),
            (256, 32) => Self::Aes256Ctr32(Ctr32BE::new_from_slices(key, counter)?),
            (256, 64) => Self::Aes256Ctr64(Ctr64BE::new_from_slices(key, counter)?),
            (256, 128) => Self::Aes256Ctr128(Ctr128BE::new_from_slices(key, counter)?),
            _ => return Err(InvalidLength),
        };

        Ok(variant)
    }

    pub fn encrypt(&mut self, data: &[u8]) -> std::result::Result<Vec<u8>, StreamCipherError> {
        let mut ciphertext = data.to_vec();
        match self {
            Self::Aes128Ctr32(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes128Ctr64(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes128Ctr128(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes192Ctr32(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes192Ctr64(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes192Ctr128(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes256Ctr32(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes256Ctr64(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes256Ctr128(v) => v.try_apply_keystream(&mut ciphertext)?,
        }
        Ok(ciphertext)
    }

    pub fn decrypt(&mut self, data: &[u8]) -> std::result::Result<Vec<u8>, StreamCipherError> {
        let mut ciphertext = data.to_vec();
        match self {
            Self::Aes128Ctr32(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes128Ctr64(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes128Ctr128(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes192Ctr32(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes192Ctr64(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes192Ctr128(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes256Ctr32(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes256Ctr64(v) => v.try_apply_keystream(&mut ciphertext)?,
            Self::Aes256Ctr128(v) => v.try_apply_keystream(&mut ciphertext)?,
        }
        Ok(ciphertext)
    }
}

pub enum AesGcmVariant {
    Aes128Gcm96(AesGcm<Aes128, U12, U12>),
    Aes192Gcm96(AesGcm<Aes192, U12, U12>),
    Aes256Gcm96(AesGcm<Aes256, U12, U12>),
    Aes128Gcm104(AesGcm<Aes128, U12, U13>),
    Aes192Gcm104(AesGcm<Aes192, U12, U13>),
    Aes256Gcm104(AesGcm<Aes256, U12, U13>),
    Aes128Gcm112(AesGcm<Aes128, U12, U14>),
    Aes192Gcm112(AesGcm<Aes192, U12, U14>),
    Aes256Gcm112(AesGcm<Aes256, U12, U14>),
    Aes128Gcm120(AesGcm<Aes128, U12, U15>),
    Aes192Gcm120(AesGcm<Aes192, U12, U15>),
    Aes256Gcm120(AesGcm<Aes256, U12, U15>),
    Aes128Gcm128(AesGcm<Aes128, U12, U16>),
    Aes192Gcm128(AesGcm<Aes192, U12, U16>),
    Aes256Gcm128(AesGcm<Aes256, U12, U16>),
}

impl AesGcmVariant {
    pub fn new(
        key_len: u16,
        tag_length: u8,
        key: &[u8],
    ) -> std::result::Result<Self, InvalidLength> {
        let variant = match (key_len, tag_length) {
            (128, 96) => Self::Aes128Gcm96(AesGcm::new_from_slice(key)?),
            (192, 96) => Self::Aes192Gcm96(AesGcm::new_from_slice(key)?),
            (256, 96) => Self::Aes256Gcm96(AesGcm::new_from_slice(key)?),
            (128, 104) => Self::Aes128Gcm104(AesGcm::new_from_slice(key)?),
            (192, 104) => Self::Aes192Gcm104(AesGcm::new_from_slice(key)?),
            (256, 104) => Self::Aes256Gcm104(AesGcm::new_from_slice(key)?),
            (128, 112) => Self::Aes128Gcm112(AesGcm::new_from_slice(key)?),
            (192, 112) => Self::Aes192Gcm112(AesGcm::new_from_slice(key)?),
            (256, 112) => Self::Aes256Gcm112(AesGcm::new_from_slice(key)?),
            (128, 120) => Self::Aes128Gcm120(AesGcm::new_from_slice(key)?),
            (192, 120) => Self::Aes192Gcm120(AesGcm::new_from_slice(key)?),
            (256, 120) => Self::Aes256Gcm120(AesGcm::new_from_slice(key)?),
            (128, 128) => Self::Aes128Gcm128(AesGcm::new_from_slice(key)?),
            (192, 128) => Self::Aes192Gcm128(AesGcm::new_from_slice(key)?),
            (256, 128) => Self::Aes256Gcm128(AesGcm::new_from_slice(key)?),
            _ => return Err(InvalidLength),
        };

        Ok(variant)
    }

    pub fn encrypt(
        &self,
        nonce: &[u8],
        msg: &[u8],
        aad: Option<&[u8]>,
    ) -> std::result::Result<Vec<u8>, aes_gcm::Error> {
        let plaintext: Payload = Payload {
            msg,
            aad: aad.unwrap_or_default(),
        };
        match self {
            Self::Aes128Gcm96(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes192Gcm96(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes256Gcm96(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes128Gcm104(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes192Gcm104(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes256Gcm104(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes128Gcm112(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes192Gcm112(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes256Gcm112(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes128Gcm120(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes192Gcm120(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes256Gcm120(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes128Gcm128(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes192Gcm128(v) => v.encrypt(nonce.into(), plaintext),
            Self::Aes256Gcm128(v) => v.encrypt(nonce.into(), plaintext),
        }
    }

    pub fn decrypt(
        &self,
        nonce: &[u8],
        msg: &[u8],
        aad: Option<&[u8]>,
    ) -> std::result::Result<Vec<u8>, aes_gcm::Error> {
        let ciphertext: Payload = Payload {
            msg,
            aad: aad.unwrap_or_default(),
        };
        match self {
            Self::Aes128Gcm96(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes192Gcm96(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes256Gcm96(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes128Gcm104(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes192Gcm104(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes256Gcm104(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes128Gcm112(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes192Gcm112(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes256Gcm112(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes128Gcm120(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes192Gcm120(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes256Gcm120(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes128Gcm128(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes192Gcm128(v) => v.decrypt(nonce.into(), ciphertext),
            Self::Aes256Gcm128(v) => v.decrypt(nonce.into(), ciphertext),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EllipticCurve {
    P256,
    P384,
    P521,
}

str_enum!(EllipticCurve,P256 => "P-256", P384 => "P-384", P521 => "P-521");

pub fn rsa_private_key<'a>(
    ctx: &Ctx<'_>,
    key: &'a CryptoKey,
    data: &'a [u8],
    algorithm_name: &str,
) -> Result<(RsaPrivateKey, &'a ShaAlgorithm, Digest)> {
    let hash = match &key.algorithm {
        KeyAlgorithm::Rsa { hash, .. } => hash,
        _ => return algorithm_mismatch_error(ctx, algorithm_name),
    };
    if !matches!(
        hash,
        ShaAlgorithm::SHA256 | ShaAlgorithm::SHA384 | ShaAlgorithm::SHA512
    ) {
        return Err(Exception::throw_message(
            ctx,
            "Only Sha-256, Sha-384 or Sha-512 is supported for RSA",
        ));
    }
    let private_key = RsaPrivateKey::from_pkcs1_der(&key.handle).or_throw(ctx)?;

    let digest = ring::digest::digest(hash.digest_algorithm(), data);

    Ok((private_key, hash, digest))
}

pub fn rsa_oaep_private_key(
    ctx: &Ctx<'_>,
    handle: &[u8],
    label: &Option<Box<[u8]>>,
    hash: &ShaAlgorithm,
) -> Result<(RsaPrivateKey, Oaep)> {
    let private_key = RsaPrivateKey::from_pkcs1_der(handle).or_throw(ctx)?;
    let mut padding = match hash {
        ShaAlgorithm::SHA1 => {
            return Err(Exception::throw_message(
                ctx,
                "SHA-1 is not supported for RSA-OAEP",
            ));
        },
        ShaAlgorithm::SHA256 => Oaep::new::<rsa::sha2::Sha256>(),
        ShaAlgorithm::SHA384 => Oaep::new::<rsa::sha2::Sha384>(),
        ShaAlgorithm::SHA512 => Oaep::new::<rsa::sha2::Sha512>(),
    };
    if let Some(label) = label {
        if !label.is_empty() {
            padding.label = Some(String::from_utf8_lossy(label).to_string());
        }
    }

    Ok((private_key, padding))
}

pub fn to_name_and_maybe_object<'js, 'a>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
) -> Result<(String, std::result::Result<Object<'js>, &'a str>)> {
    let obj;
    let name = if let Some(string) = value.as_string() {
        obj = Err("Not an object");
        string.to_string()?
    } else if let Some(object) = value.into_object() {
        let name = object.get_required("name", "algorithm")?;
        obj = Ok(object);
        name
    } else {
        return Err(Exception::throw_message(
            ctx,
            "algorithm must be a string or an object",
        ));
    };
    Ok((name, obj))
}

pub fn algorithm_mismatch_error<T>(ctx: &Ctx<'_>, expected_algorithm: &str) -> Result<T> {
    Err(Exception::throw_message(
        ctx,
        &["Key algorithm must be ", expected_algorithm].concat(),
    ))
}

pub fn algorithm_not_supported_error<T>(ctx: &Ctx<'_>) -> Result<T> {
    Err(Exception::throw_message(ctx, "Algorithm not supported"))
}
