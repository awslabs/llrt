// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod crypto_key;
mod decrypt;
mod derive_bits;
mod digest;
mod encrypt;
mod export_key;
mod generate_key;
mod sign;
mod verify;

pub use crypto_key::CryptoKey;
pub use decrypt::subtle_decrypt;
pub use derive_bits::subtle_derive_bits;
pub use digest::subtle_digest;
pub use encrypt::subtle_encrypt;
pub use export_key::subtle_export_key;
pub use generate_key::subtle_generate_key;
pub use sign::subtle_sign;
pub use verify::subtle_verify;

use aes::{cipher::typenum::U16, Aes256};
use aes_gcm::AesGcm;
use hmac::Hmac;
use llrt_utils::{object::ObjectExt, result::ResultExt};
use rquickjs::{Array, Ctx, Exception, Result, Value};
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;
pub type Aes256Gcm = AesGcm<Aes256, U16>;

#[derive(Debug)]
pub enum Sha {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl TryFrom<&str> for Sha {
    type Error = String;

    fn try_from(hash: &str) -> std::result::Result<Self, Self::Error> {
        match hash.to_ascii_uppercase().as_str() {
            "SHA-1" => Ok(Sha::Sha1),
            "SHA-256" => Ok(Sha::Sha256),
            "SHA-384" => Ok(Sha::Sha384),
            "SHA-512" => Ok(Sha::Sha512),
            _ => Err(["'", hash, "' not available"].concat()),
        }
    }
}

#[derive(Debug)]
pub enum CryptoNamedCurve {
    P256,
    P384,
}

impl TryFrom<&str> for CryptoNamedCurve {
    type Error = String;

    fn try_from(curve: &str) -> std::result::Result<Self, Self::Error> {
        match curve.to_ascii_uppercase().as_str() {
            "P-256" => Ok(CryptoNamedCurve::P256),
            "P-384" => Ok(CryptoNamedCurve::P384),
            _ => Err(["'", curve, "' not available"].concat()),
        }
    }
}

#[derive(Debug)]
pub enum Algorithm {
    Hmac,
    AesGcm(Vec<u8>),
    AesCbc(Vec<u8>),
    AesCtr(Vec<u8>, u32),
    RsaPss(u32),
    RsassaPkcs1v15,
    Ecdsa(Sha),
    RsaOaep(Option<Vec<u8>>),
}

#[derive(Debug)]
pub enum DeriveAlgorithm {
    Edch {
        curve: CryptoNamedCurve,
        public: Vec<u8>,
    },
    Hkdf {
        hash: Sha,
        salt: Vec<u8>,
        info: Vec<u8>,
    },
    Pbkdf2 {
        hash: Sha,
        salt: Vec<u8>,
        iterations: u32,
    },
}

#[derive(Debug)]
pub enum KeyGenAlgorithm {
    Rsa {
        modulus_length: u32,
        public_exponent: Vec<u8>,
    },
    Ec {
        curve: CryptoNamedCurve,
    },
    Aes {
        length: u32,
    },
    Hmac {
        hash: Sha,
        length: Option<u32>,
    },
}

fn extract_algorithm_object(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Algorithm> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'name' property required"))?;

    match name.as_str() {
        "HMAC" => Ok(Algorithm::Hmac),
        "AES-GCM" => {
            let iv = algorithm
                .get_optional("iv")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'iv' property required"))?;

            Ok(Algorithm::AesGcm(iv))
        },
        "AES-CBC" => {
            let iv = algorithm
                .get_optional("iv")?
                .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'iv' property required"))?;

            Ok(Algorithm::AesCbc(iv))
        },
        "AES-CTR" => {
            let counter = algorithm.get_optional("counter")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'counter' property required")
            })?;

            let length = algorithm.get_optional("length")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'length' property required")
            })?;

            Ok(Algorithm::AesCtr(counter, length))
        },
        "RSA-OAEP" => {
            let label = algorithm.get_optional("label")?;

            Ok(Algorithm::RsaOaep(label))
        },
        _ => Err(Exception::throw_message(
            ctx,
            "Algorithm 'name' must be HMAC | AES-GCM | AES-CBC | AES-CTR | RSA-OAEP",
        )),
    }
}

fn extract_sign_verify_algorithm(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Algorithm> {
    if algorithm.is_string() {
        let algorithm_name = algorithm.as_string().unwrap().to_string()?;

        return match algorithm_name.as_str() {
            "RSASSA-PKCS1-v1_5" => Ok(Algorithm::RsassaPkcs1v15),
            "HMAC" => Ok(Algorithm::Hmac),
            _ => Err(Exception::throw_message(ctx, "Algorithm not supported")),
        };
    }

    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'name' property required"))?;

    match name.as_str() {
        "RSASSA-PKCS1-v1_5" => Ok(Algorithm::RsassaPkcs1v15),
        "HMAC" => Ok(Algorithm::Hmac),
        "RSA-PSS" => {
            let salt_length = algorithm.get_optional("saltLength")?.ok_or_else(|| {
                Exception::throw_type(ctx, "algorithm 'saltLength' property required")
            })?;

            Ok(Algorithm::RsaPss(salt_length))
        },
        "ECDSA" => {
            let sha = extract_sha_hash(ctx, algorithm)?;

            Ok(Algorithm::Ecdsa(sha))
        },
        _ => Err(Exception::throw_message(
            ctx,
            "Algorithm 'name' must be RSASSA-PKCS1-v1_5 | HMAC | RSA-PSS | ECDSA",
        )),
    }
}

fn extract_sha_hash(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Sha> {
    let hash = algorithm
        .get_optional::<_, String>("hash")?
        .ok_or_else(|| Exception::throw_type(ctx, "algorithm 'hash' property required"))?;

    Sha::try_from(hash.as_str()).or_throw(ctx)
}

fn check_supported_usage(ctx: &Ctx<'_>, key_usages: &Array, name: &str) -> Result<()> {
    if !key_usages.contains_key(name)? {
        return Err(Exception::throw_type(
            ctx,
            &["CryptoKey doesn't support '", name, "'"].concat(),
        ));
    }
    Ok(())
}
