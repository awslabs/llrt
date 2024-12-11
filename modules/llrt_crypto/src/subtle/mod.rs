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
use llrt_utils::object::ObjectExt;
use ring::signature;
use rquickjs::Object;
use rquickjs::Value;
use rsa::{pkcs1::DecodeRsaPrivateKey, Oaep, RsaPrivateKey};
pub use sign::subtle_sign;
pub use verify::subtle_verify;

use aes::{
    cipher::{
        consts::{U13, U14, U15, U16},
        typenum::U12,
        InvalidLength,
    },
    Aes128, Aes192, Aes256,
};
use aes_gcm::{
    aead::{Aead, Payload},
    AesGcm, KeyInit,
};
use ctr::{Ctr128BE, Ctr32BE, Ctr64BE};
use llrt_utils::{result::ResultExt, str_enum};
use rquickjs::{Ctx, Exception, Result};

use crate::sha_hash::ShaAlgorithm;

type Aes128Ctr32 = Ctr32BE<aes::Aes128>;
type Aes128Ctr64 = Ctr64BE<aes::Aes128>;
type Aes128Ctr128 = Ctr128BE<aes::Aes128>;
type Aes192Ctr32 = Ctr32BE<aes::Aes192>;
type Aes192Ctr64 = Ctr64BE<aes::Aes192>;
type Aes192Ctr128 = Ctr128BE<aes::Aes192>;
type Aes256Ctr32 = Ctr32BE<aes::Aes256>;
type Aes256Ctr64 = Ctr64BE<aes::Aes256>;
type Aes256Ctr128 = Ctr128BE<aes::Aes256>;

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

#[derive(Debug, Clone)]
pub enum EllipticCurve {
    P256,
    P384,
}

impl EllipticCurve {
    fn as_signing_algorithm<'a>(&self) -> &'a signature::EcdsaSigningAlgorithm {
        match self {
            EllipticCurve::P256 => &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            EllipticCurve::P384 => &signature::ECDSA_P384_SHA384_FIXED_SIGNING,
        }
    }
}

str_enum!(EllipticCurve,P256 => "P-256",P384 => "P-384");

pub fn rsa_private_key(
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

pub fn algorithm_missmatch_error(ctx: &Ctx<'_>) -> Result<Vec<u8>> {
    Err(Exception::throw_message(
        ctx,
        "key.algorithm does not match that of operation",
    ))
}

pub fn algorithm_not_supported_error<T>(ctx: &Ctx<'_>) -> Result<T> {
    Err(Exception::throw_message(ctx, "Algorithm not supported"))
}
