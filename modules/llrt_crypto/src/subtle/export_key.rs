// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use der::{
    asn1::{self, BitString},
    Encode, SecretDocument,
};
use llrt_encoding::bytes_to_b64_url_safe_string;
use llrt_utils::result::ResultExt;
use p256::elliptic_curve;
use pkcs8::{AssociatedOid, PrivateKeyInfo};
use ring::signature::{EcdsaKeyPair, Ed25519KeyPair, KeyPair, RsaKeyPair};

use pkcs8::EncodePrivateKey;
use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Object, Result};
use rsa::{pkcs1::DecodeRsaPrivateKey, RsaPrivateKey};
use spki::{AlgorithmIdentifier, AlgorithmIdentifierOwned, SubjectPublicKeyInfo};

use crate::{subtle::CryptoKey, SYSTEM_RANDOM};

use super::{
    algorithm_not_supported_error,
    crypto_key::KeyKind,
    key_algorithm::{EcAlgorithm, KeyAlgorithm},
    EllipticCurve,
};

pub async fn subtle_export_key<'js>(
    ctx: Ctx<'js>,
    format: String,
    key: Class<'js, CryptoKey>,
) -> Result<Object<'js>> {
    let key = key.borrow();

    if !key.extractable {
        return Err(Exception::throw_type(
            &ctx,
            "The CryptoKey is non extractable",
        ));
    };

    match format.as_str() {
        "raw" => export_raw(ctx, &key),
        "pkcs8" => export_pkcs8(ctx, &key),
        "spki" => export_spki(ctx, &key),
        "jwk" => export_jwk(ctx, &key),
        _ => Err(Exception::throw_type(
            &ctx,
            &["Format '", &format, "' is not implemented"].concat(),
        )),
    }
}

fn export_raw<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
    if matches!(key.kind, KeyKind::Private) {
        return Err(Exception::throw_type(
            &ctx,
            "Private Crypto keys can't be exported as raw format",
        ));
    };
    let handle = key.handle.as_ref();
    let bytes: Vec<u8> = match &key.algorithm {
        KeyAlgorithm::Aes { .. } | KeyAlgorithm::Hmac { .. } => handle.into(),
        KeyAlgorithm::Ec { curve, .. } => {
            let alg = curve.as_signing_algorithm();
            let rng = &(*SYSTEM_RANDOM);
            let key_pair = EcdsaKeyPair::from_pkcs8(alg, &key.handle, rng).or_throw(&ctx)?;
            key_pair.public_key().as_ref().into()
        },
        KeyAlgorithm::X25519 => handle[32..].into(), //public key last 32 bytes
        KeyAlgorithm::Ed25519 => {
            let key_pair = Ed25519KeyPair::from_pkcs8(handle).or_throw(&ctx)?;
            key_pair.public_key().as_ref().into()
        },
        KeyAlgorithm::Rsa { .. } => {
            let key_pair = ring::signature::RsaKeyPair::from_pkcs8(handle).or_throw(&ctx)?;
            key_pair.public_key().as_ref().into()
        },
        _ => return algorithm_not_supported_error(&ctx),
    };

    Ok(ArrayBuffer::new(ctx, bytes)?.into_object())
}

fn export_pkcs8<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
    let handle = key.handle.as_ref();

    if !matches!(key.kind, KeyKind::Private) {
        return Err(Exception::throw_type(
            &ctx,
            "Public or Secret Crypto keys can't be exported as pkcs8 format",
        ));
    }

    let bytes: Vec<u8> = match &key.algorithm {
        KeyAlgorithm::Ec { .. } | KeyAlgorithm::Ed25519 => handle.into(),
        KeyAlgorithm::X25519 => PrivateKeyInfo::new(
            AlgorithmIdentifier {
                oid: const_oid::db::rfc8410::ID_X_25519,
                parameters: None,
            },
            &handle[0..32], //private key lengths
        )
        .to_der()
        .or_throw(&ctx)?,
        KeyAlgorithm::Rsa { .. } => rsa_der_pkcs1_to_pkcs8(&ctx, handle)?.as_bytes().to_vec(),
        _ => return algorithm_not_supported_error(&ctx),
    };

    Ok(ArrayBuffer::new(ctx, bytes)?.into_object())
}

fn rsa_der_pkcs1_to_pkcs8(ctx: &Ctx, handle: &[u8]) -> Result<SecretDocument> {
    let private_key = RsaPrivateKey::from_pkcs1_der(handle).or_throw(ctx)?;
    private_key.to_pkcs8_der().or_throw(ctx)
}

fn export_spki<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
    if !matches!(key.kind, KeyKind::Public) {
        return Err(Exception::throw_type(
            &ctx,
            "Private or Secret Crypto keys can't be exported as spki format",
        ));
    }

    let handle = key.handle.as_ref();
    let bytes: Vec<u8> = match &key.algorithm {
        KeyAlgorithm::X25519 => {
            let public_key = &handle[32..]; //public key last 32 bytes

            let key_info = spki::SubjectPublicKeyInfo {
                algorithm: spki::AlgorithmIdentifierRef {
                    oid: const_oid::db::rfc8410::ID_X_25519,
                    parameters: None,
                },
                subject_public_key: BitString::from_bytes(public_key).unwrap(),
            };

            key_info.to_der().unwrap()
        },
        KeyAlgorithm::Ec { curve, algorithm } => {
            let alg = curve.as_signing_algorithm();
            let rng = &(*SYSTEM_RANDOM);
            let key_pair = EcdsaKeyPair::from_pkcs8(alg, &key.handle, rng).or_throw(&ctx)?;
            let public_key_bytes = key_pair.public_key().as_ref().to_vec();

            let alg_id = match curve {
                EllipticCurve::P256 => AlgorithmIdentifierOwned {
                    oid: elliptic_curve::ALGORITHM_OID,
                    parameters: Some((&p256::NistP256::OID).into()),
                },
                EllipticCurve::P384 => AlgorithmIdentifierOwned {
                    oid: elliptic_curve::ALGORITHM_OID,
                    parameters: Some((&p384::NistP384::OID).into()),
                },
            };

            let alg_id = match algorithm {
                EcAlgorithm::Ecdh { .. } => AlgorithmIdentifier {
                    oid: const_oid::db::rfc5912::ID_EC_PUBLIC_KEY,
                    parameters: alg_id.parameters,
                },
                _ => alg_id,
            };

            //unwrap ok, key is always valid after this stage
            let key_info = SubjectPublicKeyInfo {
                algorithm: alg_id,

                subject_public_key: BitString::from_bytes(&public_key_bytes).unwrap(),
            };

            key_info.to_der().unwrap()
        },
        KeyAlgorithm::Ed25519 => {
            let key_pair = Ed25519KeyPair::from_pkcs8(handle).or_throw(&ctx)?;

            let key_info = spki::SubjectPublicKeyInfo {
                algorithm: spki::AlgorithmIdentifierOwned {
                    oid: const_oid::db::rfc8410::ID_ED_25519,
                    parameters: None,
                },
                subject_public_key: BitString::from_bytes(key_pair.public_key().as_ref()).unwrap(),
            };
            key_info.to_der().unwrap()
        },

        KeyAlgorithm::Rsa { .. } => {
            let pkcs8 = rsa_der_pkcs1_to_pkcs8(&ctx, handle)?;
            let pkcs8 = pkcs8.as_bytes();
            let key_pair = RsaKeyPair::from_pkcs8(pkcs8).or_throw(&ctx)?;
            let public_key = key_pair.public().as_ref();

            //unwrap ok, key is always valid after this stage
            let key_info = spki::SubjectPublicKeyInfo {
                algorithm: spki::AlgorithmIdentifier {
                    oid: const_oid::db::rfc5912::RSA_ENCRYPTION,
                    parameters: Some(asn1::AnyRef::from(asn1::Null)),
                },
                subject_public_key: BitString::from_bytes(public_key).unwrap(),
            };

            key_info.to_der().unwrap()
        },
        _ => return algorithm_not_supported_error(&ctx),
    };

    Ok(ArrayBuffer::new(ctx, bytes)?.into_object())
}

fn export_jwk<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
    let _name = key.name.as_ref();
    let handle = key.handle.as_ref();
    let obj = Object::new(ctx.clone())?;
    obj.set("key_ops", key.usages())?;
    obj.set("ext", true)?;
    match &key.algorithm {
        KeyAlgorithm::Aes { .. } => {
            let k = bytes_to_b64_url_safe_string(handle);
            obj.set("kty", "oct")?;
            obj.set("k", k)?;
        },
        KeyAlgorithm::Hmac { hash, .. } => {
            let k = bytes_to_b64_url_safe_string(handle);
            obj.set("kty", "oct")?;
            obj.set("alg", format!("HS{}", hash.as_str()))?;
            obj.set("k", k)?;
        },
        KeyAlgorithm::Ec { curve, .. } => {
            let key_data = EcKeyData::new(&ctx, curve, handle)?;

            let (x, y) = key_data.coordinates();

            obj.set("kty", "EC")?;
            obj.set("crv", curve.as_str())?;
            obj.set("x", bytes_to_b64_url_safe_string(x))?;
            obj.set("y", bytes_to_b64_url_safe_string(y))?;

            if matches!(key.kind, KeyKind::Private) {
                let d = key_data.private_key(handle);

                obj.set("d", bytes_to_b64_url_safe_string(d))?;
            }
        },
        KeyAlgorithm::Ed25519 => {
            let key_pair = Ed25519KeyPair::from_pkcs8(handle).or_throw(&ctx)?;
            let pub_key = key_pair.public_key().as_ref();
            let x = bytes_to_b64_url_safe_string(pub_key);
            obj.set("kty", "OKP")?;
            obj.set("crv", "Ed25519")?;
            obj.set("x", x)?;
        },
        KeyAlgorithm::Rsa { .. } => {
            let pkcs8 = rsa_der_pkcs1_to_pkcs8(&ctx, handle)?;

            let pkcs8 = pkcs8.as_bytes();
            let key_pair = ring::signature::RsaKeyPair::from_pkcs8(pkcs8).or_throw(&ctx)?;
            let pub_key = key_pair.public_key().as_ref();
            let n = bytes_to_b64_url_safe_string(pub_key);
            obj.set("kty", "RSA")?;
            obj.set("n", n)?;
            obj.set("e", "AQAB")?; // Default RSA public exponent (65537)
        },
        _ => return algorithm_not_supported_error(&ctx),
    };

    Ok(obj)
}

struct EcKeyData {
    key_pair: EcdsaKeyPair,
    byte_length: usize,
}

impl EcKeyData {
    fn new(ctx: &Ctx, curve: &EllipticCurve, handle: &[u8]) -> Result<Self> {
        let alg = curve.as_signing_algorithm();
        let rng = &(*SYSTEM_RANDOM);
        let key_pair = EcdsaKeyPair::from_pkcs8(alg, handle, rng).or_throw(ctx)?;

        let byte_length = match curve {
            EllipticCurve::P256 => 32,
            EllipticCurve::P384 => 48,
        };

        Ok(Self {
            key_pair,
            byte_length,
        })
    }

    fn private_key<'a>(&self, pkcs8: &'a [u8]) -> &'a [u8] {
        let start_key = 36;
        let end_key = start_key + self.byte_length;
        &pkcs8[start_key..end_key]
    }

    fn coordinates(&self) -> (&[u8], &[u8]) {
        let pub_key = self.key_pair.public_key().as_ref();

        let start_x = 1;
        let end_x = start_x + self.byte_length;
        let start_y = end_x;
        let end_y = start_y + self.byte_length;
        let x = &pub_key[start_x..end_x];
        let y = &pub_key[start_y..end_y];
        (x, y)
    }
}
