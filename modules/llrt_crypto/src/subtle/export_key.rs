// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use der::{
    asn1::{self, BitString},
    Decode, Encode, SecretDocument,
};
use elliptic_curve::{
    sec1::{FromEncodedPoint, ModulusSize, ToEncodedPoint},
    AffinePoint, CurveArithmetic, FieldBytesSize,
};
use llrt_encoding::bytes_to_b64_url_safe_string;
use llrt_utils::result::ResultExt;
use pkcs8::{AssociatedOid, DecodePrivateKey, PrivateKeyInfo};

use pkcs8::EncodePrivateKey;
use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Object, Result};
use rsa::{pkcs1::DecodeRsaPrivateKey, RsaPrivateKey};
use spki::{AlgorithmIdentifier, AlgorithmIdentifierOwned, SubjectPublicKeyInfo};

use crate::{sha_hash::ShaAlgorithm, subtle::CryptoKey};

pub fn algorithm_export_error<T>(ctx: &Ctx<'_>, algorithm: &str, format: &str) -> Result<T> {
    Err(Exception::throw_message(
        ctx,
        &["Export of ", algorithm, " as ", format, " is not supported"].concat(),
    ))
}

use super::{
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
    if key.kind == KeyKind::Private {
        return Err(Exception::throw_type(
            &ctx,
            "Private Crypto keys can't be exported as raw format",
        ));
    };
    if !matches!(
        key.algorithm,
        KeyAlgorithm::Aes { .. }
            | KeyAlgorithm::Ec { .. }
            | KeyAlgorithm::Hmac { .. }
            | KeyAlgorithm::Rsa { .. }
            | KeyAlgorithm::Ed25519
            | KeyAlgorithm::X25519
    ) {
        return algorithm_export_error(&ctx, &key.name, "raw");
    }

    Ok(ArrayBuffer::new(ctx, key.handle.as_ref())?.into_object())
}

fn export_pkcs8<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
    let handle = key.handle.as_ref();

    if key.kind != KeyKind::Private {
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
            handle,
        )
        .to_der()
        .or_throw(&ctx)?,
        KeyAlgorithm::Rsa { .. } => rsa_der_pkcs1_to_pkcs8(&ctx, handle)?.as_bytes().to_vec(),
        _ => return algorithm_export_error(&ctx, &key.name, "pkcs8"),
    };

    Ok(ArrayBuffer::new(ctx, bytes)?.into_object())
}

fn rsa_der_pkcs1_to_pkcs8(ctx: &Ctx, handle: &[u8]) -> Result<SecretDocument> {
    let private_key = RsaPrivateKey::from_pkcs1_der(handle).or_throw(ctx)?;
    private_key.to_pkcs8_der().or_throw(ctx)
}

fn export_spki<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
    if key.kind != KeyKind::Public {
        return Err(Exception::throw_type(
            &ctx,
            "Private or Secret Crypto keys can't be exported as spki format",
        ));
    }

    let public_key_bytes = key.handle.as_ref();
    let bytes: Vec<u8> = match &key.algorithm {
        KeyAlgorithm::X25519 => {
            let key_info = spki::SubjectPublicKeyInfo {
                algorithm: spki::AlgorithmIdentifierRef {
                    oid: const_oid::db::rfc8410::ID_X_25519,
                    parameters: None,
                },
                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };

            key_info.to_der().unwrap()
        },
        KeyAlgorithm::Ec { curve, algorithm } => {
            let alg_id = match curve {
                EllipticCurve::P256 => AlgorithmIdentifierOwned {
                    oid: elliptic_curve::ALGORITHM_OID,
                    parameters: Some((&p256::NistP256::OID).into()),
                },
                EllipticCurve::P384 => AlgorithmIdentifierOwned {
                    oid: elliptic_curve::ALGORITHM_OID,
                    parameters: Some((&p384::NistP384::OID).into()),
                },
                EllipticCurve::P521 => AlgorithmIdentifierOwned {
                    oid: elliptic_curve::ALGORITHM_OID,
                    parameters: Some((&p521::NistP521::OID).into()),
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

                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };

            key_info.to_der().unwrap()
        },
        KeyAlgorithm::Ed25519 => {
            let key_info = spki::SubjectPublicKeyInfo {
                algorithm: spki::AlgorithmIdentifierOwned {
                    oid: const_oid::db::rfc8410::ID_ED_25519,
                    parameters: None,
                },
                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };
            key_info.to_der().unwrap()
        },

        KeyAlgorithm::Rsa { .. } => {
            //unwrap ok, key is always valid after this stage
            let key_info = spki::SubjectPublicKeyInfo {
                algorithm: spki::AlgorithmIdentifier {
                    oid: const_oid::db::rfc5912::RSA_ENCRYPTION,
                    parameters: Some(asn1::AnyRef::from(asn1::Null)),
                },
                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };

            key_info.to_der().unwrap()
        },
        _ => return algorithm_export_error(&ctx, &key.name, "spki"),
    };

    Ok(ArrayBuffer::new(ctx, bytes)?.into_object())
}

fn export_jwk<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
    let name = key.name.as_ref();
    let handle = key.handle.as_ref();
    let obj = Object::new(ctx.clone())?;
    obj.set("key_ops", key.usages())?;
    obj.set("ext", true)?;
    match &key.algorithm {
        KeyAlgorithm::Aes { length } => {
            let prefix = match length {
                128 => "A128",
                192 => "A192",
                256 => "A256",
                _ => unreachable!(),
            };
            let suffix = &name[("AES-".len())..];
            let alg = [prefix, suffix].concat();

            let k = bytes_to_b64_url_safe_string(handle);
            obj.set("kty", "oct")?;
            obj.set("k", k)?;
            obj.set("alg", alg)?
        },
        KeyAlgorithm::Hmac { hash, .. } => {
            let k = bytes_to_b64_url_safe_string(handle);
            obj.set("kty", "oct")?;
            obj.set("alg", ["HS", &hash.as_str()[4..]].concat())?;
            obj.set("k", k)?;
        },
        KeyAlgorithm::Ec { curve, .. } => {
            fn set_public_key_coords<C>(
                obj: &Object<'_>,
                public_key: elliptic_curve::PublicKey<C>,
            ) -> Result<()>
            where
                C: CurveArithmetic,
                AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C>,
                FieldBytesSize<C>: ModulusSize,
            {
                let p = public_key.to_encoded_point(false);
                let x = p.x().unwrap().as_slice();
                let y = p.y().unwrap().as_slice();
                obj.set("x", bytes_to_b64_url_safe_string(x))?;
                obj.set("y", bytes_to_b64_url_safe_string(y))?;
                Ok(())
            }

            fn set_private_key_props<C>(
                obj: &Object<'_>,
                private_key: elliptic_curve::SecretKey<C>,
            ) -> Result<()>
            where
                C: elliptic_curve::Curve + elliptic_curve::CurveArithmetic,
                AffinePoint<C>: FromEncodedPoint<C> + ToEncodedPoint<C>,
                FieldBytesSize<C>: ModulusSize,
            {
                let public_key = private_key.public_key();
                set_public_key_coords(obj, public_key)?;
                obj.set(
                    "d",
                    bytes_to_b64_url_safe_string(private_key.to_bytes().as_slice()),
                )?;
                Ok(())
            }

            match key.kind {
                KeyKind::Public => match curve {
                    EllipticCurve::P256 => {
                        let public_key = p256::PublicKey::from_sec1_bytes(handle).or_throw(&ctx)?;
                        set_public_key_coords(&obj, public_key)?;
                    },
                    EllipticCurve::P384 => {
                        let public_key = p384::PublicKey::from_sec1_bytes(handle).or_throw(&ctx)?;
                        set_public_key_coords(&obj, public_key)?;
                    },
                    EllipticCurve::P521 => {
                        let public_key = p521::PublicKey::from_sec1_bytes(handle).or_throw(&ctx)?;
                        set_public_key_coords(&obj, public_key)?;
                    },
                },
                KeyKind::Private => match curve {
                    EllipticCurve::P256 => {
                        let private_key = p256::SecretKey::from_pkcs8_der(handle).or_throw(&ctx)?;
                        set_private_key_props(&obj, private_key)?;
                    },
                    EllipticCurve::P384 => {
                        let private_key = p384::SecretKey::from_pkcs8_der(handle).or_throw(&ctx)?;
                        set_private_key_props(&obj, private_key)?;
                    },
                    EllipticCurve::P521 => {
                        let private_key = p521::SecretKey::from_pkcs8_der(handle).or_throw(&ctx)?;
                        set_private_key_props(&obj, private_key)?;
                    },
                },
                _ => unreachable!(),
            }

            obj.set("kty", "EC")?;
            obj.set("crv", curve.as_str())?;
        },
        KeyAlgorithm::Ed25519 => {
            if key.kind == KeyKind::Private {
                let pki = PrivateKeyInfo::try_from(handle).or_throw(&ctx)?;
                let pub_key = pki.public_key.as_ref().unwrap();
                set_okp_jwk_props(name, &obj, Some(pki.private_key), pub_key)?;
            } else {
                set_okp_jwk_props(name, &obj, None, handle)?;
            }
        },
        KeyAlgorithm::Rsa { hash, .. } => {
            let (n, e) = match key.kind {
                KeyKind::Public => {
                    let public_key = rsa::pkcs1::RsaPublicKey::from_der(handle).or_throw(&ctx)?;
                    let n = bytes_to_b64_url_safe_string(public_key.modulus.as_bytes());
                    let e = bytes_to_b64_url_safe_string(public_key.public_exponent.as_bytes());
                    (n, e)
                },
                KeyKind::Private => {
                    let private_key = rsa::pkcs1::RsaPrivateKey::from_der(handle).or_throw(&ctx)?;
                    let n = bytes_to_b64_url_safe_string(private_key.modulus.as_bytes());
                    let e = bytes_to_b64_url_safe_string(private_key.public_exponent.as_bytes());
                    let d = bytes_to_b64_url_safe_string(private_key.private_exponent.as_bytes());
                    let p = bytes_to_b64_url_safe_string(private_key.prime1.as_bytes());
                    let q = bytes_to_b64_url_safe_string(private_key.prime2.as_bytes());
                    let dp = bytes_to_b64_url_safe_string(private_key.exponent1.as_bytes());
                    let dq = bytes_to_b64_url_safe_string(private_key.exponent2.as_bytes());
                    let qi = bytes_to_b64_url_safe_string(private_key.coefficient.as_bytes());
                    obj.set("d", d)?;
                    obj.set("p", p)?;
                    obj.set("q", q)?;
                    obj.set("dp", dp)?;
                    obj.set("dq", dq)?;
                    obj.set("qi", qi)?;
                    (n, e)
                },
                _ => {
                    unreachable!()
                },
            };

            let alg_suffix = match hash {
                ShaAlgorithm::SHA1 => "1",
                ShaAlgorithm::SHA256 => "256",
                ShaAlgorithm::SHA384 => "384",
                ShaAlgorithm::SHA512 => "512",
            };

            let alg_prefix = match name {
                "RSASSA-PKCS1-v1_5" => "RS",
                "RSA-PSS" => "PS",
                "RSA-OAEP" => "RSA-OAEP-",
                _ => unreachable!(),
            };

            let alg = [alg_prefix, alg_suffix].concat();

            obj.set("kty", "RSA")?;
            obj.set("n", n)?;
            obj.set("e", e)?;
            obj.set("alg", alg)?;
        },
        KeyAlgorithm::X25519 => match key.kind {
            KeyKind::Private => {
                let array: [u8; 32] = handle.try_into().or_throw(&ctx)?;
                let secret = x25519_dalek::StaticSecret::from(array);
                let public_key = x25519_dalek::PublicKey::from(&secret);
                set_okp_jwk_props(name, &obj, Some(secret.as_bytes()), public_key.as_bytes())?;
            },
            KeyKind::Public => {
                let public_key = handle;
                set_okp_jwk_props(name, &obj, None, public_key)?;
            },
            _ => unreachable!(),
        },
        //cant be exported
        _ => return algorithm_export_error(&ctx, &key.name, "jwk"),
    };

    Ok(obj)
}

fn set_okp_jwk_props(
    crv: &str,
    obj: &Object<'_>,
    private_key: Option<&[u8]>,
    public_key: &[u8],
) -> Result<()> {
    let x = bytes_to_b64_url_safe_string(public_key);
    obj.set("kty", "OKP")?;
    obj.set("crv", crv)?;
    obj.set("x", x)?;
    if let Some(private_key) = private_key {
        let d = bytes_to_b64_url_safe_string(private_key);
        obj.set("d", d)?;
    }
    Ok(())
}
