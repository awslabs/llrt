// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod crypto_key;
mod decrypt;
mod derive_bits;
mod digest;
mod encrypt;
mod export_key;
mod generate_key;
mod import_key;
mod sign;
mod verify;

pub use crypto_key::CryptoKey;
pub use decrypt::subtle_decrypt;
pub use derive_bits::subtle_derive_bits;
pub use digest::subtle_digest;
pub use encrypt::subtle_encrypt;
pub use export_key::subtle_export_key;
pub use generate_key::subtle_generate_key;
pub use import_key::subtle_import_key;
pub use sign::subtle_sign;
pub use verify::subtle_verify;

use aes::{cipher::typenum::U12, Aes128, Aes192, Aes256};
use aes_gcm::AesGcm;
use ctr::{Ctr128BE, Ctr32BE, Ctr64BE};
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{Array, Ctx, Exception, Result, Value};

type Aes128Ctr32 = Ctr32BE<aes::Aes128>;
type Aes128Ctr64 = Ctr64BE<aes::Aes128>;
type Aes128Ctr128 = Ctr128BE<aes::Aes128>;
type Aes192Ctr32 = Ctr32BE<aes::Aes192>;
type Aes192Ctr64 = Ctr64BE<aes::Aes192>;
type Aes192Ctr128 = Ctr128BE<aes::Aes192>;
type Aes256Ctr32 = Ctr32BE<aes::Aes256>;
type Aes256Ctr64 = Ctr64BE<aes::Aes256>;
type Aes256Ctr128 = Ctr128BE<aes::Aes256>;

type Aes128Gcm = AesGcm<Aes128, U12>;
type Aes192Gcm = AesGcm<Aes192, U12>;
type Aes256Gcm = AesGcm<Aes256, U12>;

#[derive(Debug)]
pub enum Hash {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl TryFrom<&str> for Hash {
    type Error = String;

    fn try_from(hash: &str) -> std::result::Result<Self, Self::Error> {
        match hash.to_ascii_uppercase().as_str() {
            "SHA-1" => Ok(Hash::Sha1),
            "SHA-256" => Ok(Hash::Sha256),
            "SHA-384" => Ok(Hash::Sha384),
            "SHA-512" => Ok(Hash::Sha512),
            _ => Err(["'", hash, "' not available"].concat()),
        }
    }
}

#[derive(Debug)]
pub enum EllipticCurve {
    P256,
    P384,
}

impl TryFrom<&str> for EllipticCurve {
    type Error = String;

    fn try_from(curve: &str) -> std::result::Result<Self, Self::Error> {
        match curve.to_ascii_uppercase().as_str() {
            "P-256" => Ok(EllipticCurve::P256),
            "P-384" => Ok(EllipticCurve::P384),
            _ => Err(["'", curve, "' not available"].concat()),
        }
    }
}

#[derive(Debug)]
pub enum Algorithm {
    AesCbc { iv: Vec<u8> },
    AesCtr { counter: Vec<u8>, length: u32 },
    AesGcm { iv: Vec<u8> },
    Ecdsa { hash: Hash },
    Hmac,
    RsaOaep { label: Option<Vec<u8>> },
    RsaPss { salt_length: u32 },
    RsassaPkcs1v15,
}

#[derive(Debug)]
pub enum DeriveAlgorithm {
    Edch {
        curve: EllipticCurve,
        public: Vec<u8>,
    },
    Hkdf {
        hash: Hash,
        salt: Vec<u8>,
        info: Vec<u8>,
    },
    Pbkdf2 {
        hash: Hash,
        salt: Vec<u8>,
        iterations: u32,
    },
}

#[derive(Debug)]
pub enum KeyGenAlgorithm {
    Aes {
        length: u32,
    },
    Ec {
        curve: EllipticCurve,
    },
    Hmac {
        hash: Hash,
        length: Option<u32>,
    },
    Rsa {
        modulus_length: u32,
        public_exponent: Vec<u8>,
    },
}

fn extract_algorithm_object(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Algorithm> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'name' property required"))?;

    match name.as_str() {
        "AES-CBC" => {
            let iv = algorithm
                .get_optional::<_, ObjectBytes>("iv")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'iv' property required"))?
                .into_bytes();

            if iv.len() != 16 {
                return Err(Exception::throw_message(
                    ctx,
                    "invalid length of iv. Currently supported 16 bytes",
                ));
            }

            Ok(Algorithm::AesCbc { iv })
        },
        "AES-CTR" => {
            let counter = algorithm
                .get_optional::<_, ObjectBytes>("counter")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'counter' property required"))?
                .into_bytes();

            let length = algorithm.get_optional::<_, u32>("length")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'length' property required")
            })?;

            if ![32, 64, 128].contains(&length) {
                return Err(Exception::throw_message(
                    ctx,
                    "invalid counter length. Currently supported 32/64/128 bits",
                ));
            }

            Ok(Algorithm::AesCtr { counter, length })
        },
        "AES-GCM" => {
            let iv = algorithm
                .get_optional::<_, ObjectBytes>("iv")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'iv' property required"))?
                .into_bytes();

            if iv.len() != 12 {
                return Err(Exception::throw_type(
                    ctx,
                    "invalid length of iv. Currently supported 12 bytes",
                ));
            }

            Ok(Algorithm::AesGcm { iv })
        },
        "HMAC" => Ok(Algorithm::Hmac),
        "RSA-OAEP" => {
            let label = algorithm.get_optional::<_, ObjectBytes>("label")?;
            let label = label.map(|lbl| lbl.into_bytes());

            Ok(Algorithm::RsaOaep { label })
        },
        _ => Err(Exception::throw_message(
            ctx,
            "Algorithm 'name' must be AES-CBC | AES-CTR | HMAC | AES-GCM | RSA-OAEP",
        )),
    }
}

fn extract_sign_verify_algorithm(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Algorithm> {
    if algorithm.is_string() {
        let algorithm_name = algorithm.as_string().unwrap().to_string()?;

        return match algorithm_name.as_str() {
            "HMAC" => Ok(Algorithm::Hmac),
            "RSASSA-PKCS1-v1_5" => Ok(Algorithm::RsassaPkcs1v15),
            _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
        };
    }

    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'name' property required"))?;

    match name.as_str() {
        "ECDSA" => {
            let hash = extract_sha_hash(ctx, algorithm)?;

            Ok(Algorithm::Ecdsa { hash })
        },
        "HMAC" => Ok(Algorithm::Hmac),
        "RSA-PSS" => {
            let salt_length = algorithm.get_optional("saltLength")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'saltLength' property required")
            })?;

            Ok(Algorithm::RsaPss { salt_length })
        },
        "RSASSA-PKCS1-v1_5" => Ok(Algorithm::RsassaPkcs1v15),
        _ => Err(Exception::throw_message(
            ctx,
            "Algorithm 'name' must be RSASSA-PKCS1-v1_5 | HMAC | RSA-PSS | ECDSA",
        )),
    }
}

fn extract_sha_hash(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Hash> {
    let hash = algorithm
        .get_optional::<_, String>("hash")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'hash' property required"))?;

    Hash::try_from(hash.as_str()).or_throw(ctx)
}

fn check_supported_usage(ctx: &Ctx<'_>, key_usages: &Array, usage: &str) -> Result<()> {
    for value in key_usages.clone().into_iter() {
        if let Some(key) = value?.as_string() {
            let key = key.to_string()?;
            if key.as_str() == usage {
                return Ok(());
            }
        }
    }
    Err(Exception::throw_type(
        ctx,
        &["CryptoKey doesn't support '", usage, "'"].concat(),
    ))
}
