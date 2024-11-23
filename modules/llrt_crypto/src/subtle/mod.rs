// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod decrypt;
mod derive_bits;
mod digest;
mod encrypt;
mod generate_key;
mod sign;
mod verify;

use decrypt::decrypt;
use derive_bits::derive_bits;
use digest::digest;
use encrypt::encrypt;
use generate_key::generate_key;
use sign::sign;
use verify::verify;

use aes::{cipher::typenum::U16, Aes256};
use aes_gcm::AesGcm;
use hmac::Hmac;
use llrt_utils::object::ObjectExt;
use rquickjs::{Ctx, Exception, Result, Value};
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

fn get_sha(ctx: &Ctx<'_>, hash: &str) -> Result<Sha> {
    match hash {
        "SHA-1" => Ok(Sha::Sha1),
        "SHA-256" => Ok(Sha::Sha256),
        "SHA-384" => Ok(Sha::Sha384),
        "SHA-512" => Ok(Sha::Sha512),
        _ => Err(Exception::throw_message(ctx, "hash not found")),
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
pub enum CryptoNamedCurve {
    P256,
    P384,
}

fn get_named_curve(ctx: &Ctx<'_>, curve: &str) -> Result<CryptoNamedCurve> {
    match curve {
        "P-256" => Ok(CryptoNamedCurve::P256),
        "P-384" => Ok(CryptoNamedCurve::P384),
        _ => Err(Exception::throw_message(ctx, "named_curve not found")),
    }
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

pub fn subtle_decrypt(
    ctx: Ctx<'_>,
    algorithm: Value,
    key_value: Vec<u8>,
    data: Vec<u8>,
) -> Result<Vec<u8>> {
    let algorithm = extract_algorithm_object(&ctx, &algorithm)?;
    decrypt(&ctx, &algorithm, key_value, data)
}

pub fn subtle_derive_bits(
    ctx: Ctx<'_>,
    algorithm: Value,
    key_value: Vec<u8>,
    length: u32,
) -> Result<Vec<u8>> {
    let derive_algorithm = extract_derive_algorithm(&ctx, &algorithm)?;
    derive_bits(&ctx, &derive_algorithm, key_value, length)
}

pub fn subtle_digest(ctx: Ctx<'_>, name: String, data: Vec<u8>) -> Result<Vec<u8>> {
    digest(ctx, &name, data)
}

pub fn subtle_encrypt(
    ctx: Ctx<'_>,
    algorithm: Value,
    key_value: Vec<u8>,
    data: Vec<u8>,
) -> Result<Vec<u8>> {
    let algorithm = extract_algorithm_object(&ctx, &algorithm)?;
    encrypt(&ctx, &algorithm, key_value, data)
}

pub fn subtle_generate_key(ctx: Ctx<'_>, algorithm: Value) -> Result<Vec<u8>> {
    let key_gen_algorithm = extract_generate_key_algorithm(&ctx, &algorithm)?;
    generate_key(&ctx, &key_gen_algorithm)
}

pub fn subtle_sign(
    ctx: Ctx<'_>,
    algorithm: Value,
    key_value: Vec<u8>,
    data: Vec<u8>,
) -> Result<Vec<u8>> {
    let algorithm = extract_sign_verify_algorithm(&ctx, &algorithm)?;
    sign(&ctx, &algorithm, key_value, data)
}

pub fn subtle_verify(
    ctx: Ctx<'_>,
    algorithm: Value,
    key_value: Vec<u8>,
    signature: Vec<u8>,
    data: Vec<u8>,
) -> Result<bool> {
    let algorithm = extract_sign_verify_algorithm(&ctx, &algorithm)?;
    verify(&ctx, &algorithm, key_value, signature, data)
}

fn extract_algorithm_object(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Algorithm> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_message(ctx, "Algorithm name not found"))?;

    if name == "HMAC" {
        return Ok(Algorithm::Hmac);
    }

    if name == "AES-GCM" {
        let iv = algorithm
            .get_optional("iv")?
            .ok_or_else(|| Exception::throw_message(ctx, "Algorithm iv not found"))?;

        return Ok(Algorithm::AesGcm(iv));
    }

    if name == "AES-CBC" {
        let iv = algorithm
            .get_optional("iv")?
            .ok_or_else(|| Exception::throw_message(ctx, "Algorithm iv not found"))?;

        return Ok(Algorithm::AesCbc(iv));
    }

    if name == "AES-CTR" {
        let counter = algorithm
            .get_optional("counter")?
            .ok_or_else(|| Exception::throw_message(ctx, "AES-CTR counter not found"))?;

        let length = algorithm
            .get_optional("length")?
            .ok_or_else(|| Exception::throw_message(ctx, "AES-CTR length not found"))?;

        return Ok(Algorithm::AesCtr(counter, length));
    }

    if name == "RSA-OAEP" {
        let label = algorithm.get_optional("label")?;

        return Ok(Algorithm::RsaOaep(label));
    }

    Err(Exception::throw_message(ctx, "Algorithm not supported"))
}

fn extract_sha_hash(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Sha> {
    let hash = algorithm
        .get_optional::<_, String>("hash")?
        .ok_or_else(|| Exception::throw_message(ctx, "hash not found"))?;

    get_sha(ctx, &hash)
}

fn extract_sign_verify_algorithm(ctx: &Ctx<'_>, algorithm: &Value) -> Result<Algorithm> {
    if algorithm.is_string() {
        let algorithm_name = algorithm.as_string().unwrap().to_string()?;
        if algorithm_name == "RSASSA-PKCS1-v1_5" {
            return Ok(Algorithm::RsassaPkcs1v15);
        }

        if algorithm_name == "HMAC" {
            return Ok(Algorithm::Hmac);
        }

        return Err(Exception::throw_message(ctx, "Algorithm not supported"));
    }

    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_message(ctx, "Algorithm name not found"))?;

    if name == "RSASSA-PKCS1-v1_5" {
        return Ok(Algorithm::RsassaPkcs1v15);
    }

    if name == "HMAC" {
        return Ok(Algorithm::Hmac);
    }

    if name == "RSA-PSS" {
        let salt_length = algorithm
            .get_optional("saltLength")?
            .ok_or_else(|| Exception::throw_message(ctx, "RSA-PSS saltLength not found"))?;

        return Ok(Algorithm::RsaPss(salt_length));
    }

    if name == "ECDSA" {
        let sha = extract_sha_hash(ctx, algorithm)?;

        return Ok(Algorithm::Ecdsa(sha));
    }

    Err(Exception::throw_message(ctx, "Algorithm not supported"))
}

fn extract_derive_algorithm(ctx: &Ctx<'_>, algorithm: &Value) -> Result<DeriveAlgorithm> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_message(ctx, "Algorithm name not found"))?;

    if name == "ECDH" {
        let namedcurve = algorithm
            .get_optional::<_, String>("namedcurve")?
            .ok_or_else(|| {
                Exception::throw_message(ctx, "ECDH namedCurve must be one of: P-256 or P-384")
            })?;

        let curve = get_named_curve(ctx, &namedcurve)?;

        let public = algorithm
            .get_optional("public")?
            .ok_or_else(|| Exception::throw_message(ctx, "ECDH must have CryptoKey"))?;

        return Ok(DeriveAlgorithm::Edch { curve, public });
    }

    if name == "HKDF" {
        let hash = algorithm
            .get_optional::<_, String>("hash")?
            .ok_or_else(|| Exception::throw_message(ctx, "HKDF must have hash"))?;

        let hash = get_sha(ctx, &hash)?;

        let salt = algorithm
            .get_optional("salt")?
            .ok_or_else(|| Exception::throw_message(ctx, "HKDF must have salt"))?;

        let info = algorithm
            .get_optional("info")?
            .ok_or_else(|| Exception::throw_message(ctx, "HKDF must have info"))?;

        return Ok(DeriveAlgorithm::Hkdf { hash, salt, info });
    }

    if name == "PBKDF2" {
        let hash = algorithm
            .get_optional::<_, String>("hash")?
            .ok_or_else(|| Exception::throw_message(ctx, "PBKDF2 must have hash"))?;

        let hash = get_sha(ctx, &hash)?;

        let salt = algorithm
            .get_optional("salt")?
            .ok_or_else(|| Exception::throw_message(ctx, "PBKDF2 must have salt"))?;

        let iterations = algorithm
            .get_optional("iterations")?
            .ok_or_else(|| Exception::throw_message(ctx, "PBKDF2 must have iterations"))?;

        return Ok(DeriveAlgorithm::Pbkdf2 {
            hash,
            salt,
            iterations,
        });
    }

    Err(Exception::throw_message(ctx, "Algorithm not supported"))
}

fn extract_generate_key_algorithm(ctx: &Ctx<'_>, algorithm: &Value) -> Result<KeyGenAlgorithm> {
    let name = algorithm
        .get_optional::<_, String>("name")?
        .ok_or_else(|| Exception::throw_message(ctx, "Algorithm name not found"))?;

    if name == "RSASSA-PKCS1-v1_5" || name == "RSA-PSS" || name == "RSA-OAEP" {
        let modulus_length = algorithm
            .get_optional("modulusLength")?
            .ok_or_else(|| Exception::throw_message(ctx, "Algorithm modulusLength not found"))?;

        let public_exponent = algorithm
            .get_optional("publicExponent")?
            .ok_or_else(|| Exception::throw_message(ctx, "Algorithm publicExponent not found"))?;

        return Ok(KeyGenAlgorithm::Rsa {
            modulus_length,
            public_exponent,
        });
    }

    if name == "ECDSA" || name == "ECDH" {
        let namedcurve = algorithm
            .get_optional::<_, String>("namedCurve")?
            .ok_or_else(|| Exception::throw_message(ctx, "Algorithm namedCurve not found"))?;
        let curve = get_named_curve(ctx, &namedcurve)?;

        return Ok(KeyGenAlgorithm::Ec { curve });
    }

    if name == "HMAC" {
        let hash = extract_sha_hash(ctx, algorithm)?;

        let length = algorithm.get_optional::<_, u32>("length")?;
        return Ok(KeyGenAlgorithm::Hmac { hash, length });
    }

    if name == "AES-CTR" || name == "AES-CBC" || name == "AES-GCM" || name == "AES-KW" {
        let length = algorithm
            .get_optional("length")?
            .ok_or_else(|| Exception::throw_message(ctx, "Algorithm length not found"))?;

        if length != 128 && length != 192 && length != 256 {
            return Err(Exception::throw_message(
                ctx,
                "Algorithm length must be one of: 128, 192, or 256.",
            ));
        }

        return Ok(KeyGenAlgorithm::Aes { length });
    }

    Err(Exception::throw_message(
                ctx,
        "Algorithm must be RsaHashedKeyGenParams | EcKeyGenParams | HmacKeyGenParams | AesKeyGenParams"
    ))
}
