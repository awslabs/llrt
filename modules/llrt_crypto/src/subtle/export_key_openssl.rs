// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! OpenSSL-based key export implementation.

use der::{asn1::BitString, Encode};
use llrt_encoding::bytes_to_b64_url_safe_string;
use llrt_utils::result::ResultExt;
use openssl::bn::{BigNum, BigNumRef};
use openssl::ec::EcGroup;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey, Private, Public};
use openssl::rsa::Rsa;
use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Object, Result};
use spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned};

use super::{
    crypto_key::KeyKind,
    key_algorithm::{KeyAlgorithm, KeyFormat},
    CryptoKey, EllipticCurve,
};

pub fn algorithm_export_error<T>(ctx: &Ctx<'_>, algorithm: &str, format: &str) -> Result<T> {
    Err(Exception::throw_message(
        ctx,
        &["Export of ", algorithm, " as ", format, " is not supported"].concat(),
    ))
}

pub enum ExportOutput<'js> {
    Bytes(Vec<u8>),
    Object(Object<'js>),
}

pub async fn subtle_export_key<'js>(
    ctx: Ctx<'js>,
    format: KeyFormat,
    key: Class<'js, CryptoKey>,
) -> Result<Object<'js>> {
    let key = key.borrow();

    let export = export_key(&ctx, format, &key)?;

    Ok(match export {
        ExportOutput::Bytes(bytes) => ArrayBuffer::new(ctx, bytes)?.into_object(),
        ExportOutput::Object(object) => object,
    })
}

pub fn export_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat,
    key: &CryptoKey,
) -> Result<ExportOutput<'js>> {
    if !key.extractable {
        return Err(Exception::throw_type(
            ctx,
            "The CryptoKey is non extractable",
        ));
    };
    let bytes = match format {
        KeyFormat::Jwk => return Ok(ExportOutput::Object(export_jwk(ctx, key)?)),
        KeyFormat::Raw => export_raw(ctx, key),
        KeyFormat::Spki => export_spki(ctx, key),
        KeyFormat::Pkcs8 => export_pkcs8(ctx, key),
    }?;
    Ok(ExportOutput::Bytes(bytes))
}

fn export_raw(ctx: &Ctx<'_>, key: &CryptoKey) -> Result<Vec<u8>> {
    if key.kind == KeyKind::Private {
        return Err(Exception::throw_type(
            ctx,
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
        return algorithm_export_error(ctx, &key.name, "raw");
    }
    Ok(key.handle.to_vec())
}

fn export_pkcs8(ctx: &Ctx<'_>, key: &CryptoKey) -> Result<Vec<u8>> {
    let handle = key.handle.as_ref();

    if key.kind != KeyKind::Private {
        return Err(Exception::throw_type(
            ctx,
            "Public or Secret Crypto keys can't be exported as pkcs8 format",
        ));
    }

    match &key.algorithm {
        KeyAlgorithm::Ec { .. } => {
            // Handle is already PKCS#8 DER
            Ok(handle.to_vec())
        },
        KeyAlgorithm::Ed25519 => {
            // Handle is already PKCS#8 DER
            Ok(handle.to_vec())
        },
        KeyAlgorithm::X25519 => {
            // Handle is raw 32-byte secret, wrap in PKCS#8
            let pkey = PKey::private_key_from_raw_bytes(handle, Id::X25519).or_throw(ctx)?;
            pkey.private_key_to_der().or_throw(ctx)
        },
        KeyAlgorithm::Rsa { .. } => {
            // Handle is PKCS#1 DER, convert to PKCS#8
            let rsa = Rsa::<Private>::private_key_from_der(handle).or_throw(ctx)?;
            let pkey = PKey::from_rsa(rsa).or_throw(ctx)?;
            pkey.private_key_to_der().or_throw(ctx)
        },
        _ => algorithm_export_error(ctx, &key.name, "pkcs8"),
    }
}

fn export_spki(ctx: &Ctx<'_>, key: &CryptoKey) -> Result<Vec<u8>> {
    if key.kind != KeyKind::Public {
        return Err(Exception::throw_type(
            ctx,
            "Private or Secret Crypto keys can't be exported as spki format",
        ));
    }

    let public_key_bytes = key.handle.as_ref();

    match &key.algorithm {
        KeyAlgorithm::X25519 => {
            let key_info = SubjectPublicKeyInfoOwned {
                algorithm: AlgorithmIdentifierOwned {
                    oid: const_oid::db::rfc8410::ID_X_25519,
                    parameters: None,
                },
                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };
            Ok(key_info.to_der().unwrap())
        },
        KeyAlgorithm::Ec { curve, .. } => {
            let curve_oid = match curve {
                EllipticCurve::P256 => const_oid::db::rfc5912::SECP_256_R_1,
                EllipticCurve::P384 => const_oid::db::rfc5912::SECP_384_R_1,
                EllipticCurve::P521 => const_oid::db::rfc5912::SECP_521_R_1,
            };

            let key_info = SubjectPublicKeyInfoOwned {
                algorithm: AlgorithmIdentifierOwned {
                    oid: const_oid::db::rfc5912::ID_EC_PUBLIC_KEY,
                    parameters: Some((&curve_oid).into()),
                },
                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };
            Ok(key_info.to_der().unwrap())
        },
        KeyAlgorithm::Ed25519 => {
            let key_info = SubjectPublicKeyInfoOwned {
                algorithm: AlgorithmIdentifierOwned {
                    oid: const_oid::db::rfc8410::ID_ED_25519,
                    parameters: None,
                },
                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };
            Ok(key_info.to_der().unwrap())
        },
        KeyAlgorithm::Rsa { .. } => {
            let key_info = SubjectPublicKeyInfoOwned {
                algorithm: AlgorithmIdentifierOwned {
                    oid: const_oid::db::rfc5912::RSA_ENCRYPTION,
                    parameters: Some(der::asn1::AnyRef::from(der::asn1::Null).into()),
                },
                subject_public_key: BitString::from_bytes(public_key_bytes).unwrap(),
            };
            Ok(key_info.to_der().unwrap())
        },
        _ => algorithm_export_error(ctx, &key.name, "spki"),
    }
}

fn export_jwk<'js>(ctx: &Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
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

            obj.set("kty", "oct")?;
            obj.set("k", bytes_to_b64_url_safe_string(handle))?;
            obj.set("alg", alg)?;
        },
        KeyAlgorithm::Hmac { hash, .. } => {
            obj.set("kty", "oct")?;
            obj.set("alg", ["HS", &hash.as_str()[4..]].concat())?;
            obj.set("k", bytes_to_b64_url_safe_string(handle))?;
        },
        KeyAlgorithm::Ec { curve, .. } => {
            let nid = curve_to_nid(curve);
            let group = EcGroup::from_curve_name(nid).or_throw(ctx)?;

            match key.kind {
                KeyKind::Public => {
                    // Handle is SEC1 uncompressed point - parse it
                    let mut bn_ctx = openssl::bn::BigNumContext::new().or_throw(ctx)?;
                    let point = openssl::ec::EcPoint::from_bytes(&group, handle, &mut bn_ctx)
                        .or_throw(ctx)?;
                    let mut x = BigNum::new().or_throw(ctx)?;
                    let mut y = BigNum::new().or_throw(ctx)?;
                    point
                        .affine_coordinates(&group, &mut x, &mut y, &mut bn_ctx)
                        .or_throw(ctx)?;
                    set_ec_public_coords(&obj, &x, &y, curve)?;
                },
                KeyKind::Private => {
                    // Handle is PKCS#8 DER
                    let pkey = PKey::<Private>::private_key_from_der(handle).or_throw(ctx)?;
                    let ec_key = pkey.ec_key().or_throw(ctx)?;
                    let private_num = ec_key.private_key();
                    let point = ec_key.public_key();
                    let mut bn_ctx = openssl::bn::BigNumContext::new().or_throw(ctx)?;
                    let mut x = BigNum::new().or_throw(ctx)?;
                    let mut y = BigNum::new().or_throw(ctx)?;
                    point
                        .affine_coordinates(&group, &mut x, &mut y, &mut bn_ctx)
                        .or_throw(ctx)?;
                    set_ec_public_coords(&obj, &x, &y, curve)?;
                    obj.set("d", bn_to_b64_padded(private_num, curve_byte_len(curve)))?;
                },
                _ => unreachable!(),
            }

            obj.set("kty", "EC")?;
            obj.set("crv", curve.as_str())?;
        },
        KeyAlgorithm::Ed25519 => {
            if key.kind == KeyKind::Private {
                // Handle is PKCS#8 DER
                let pkey = PKey::<Private>::private_key_from_der(handle).or_throw(ctx)?;
                let raw_private = pkey.raw_private_key().or_throw(ctx)?;
                let raw_public = pkey.raw_public_key().or_throw(ctx)?;
                set_okp_jwk_props(name, &obj, Some(&raw_private), &raw_public)?;
            } else {
                // Handle is raw 32-byte public key
                set_okp_jwk_props(name, &obj, None, handle)?;
            }
        },
        KeyAlgorithm::Rsa { hash, .. } => {
            let alg_suffix = hash.as_numeric_str();
            let alg_prefix = match name {
                "RSASSA-PKCS1-v1_5" => "RS",
                "RSA-PSS" => "PS",
                "RSA-OAEP" => "RSA-OAEP-",
                _ => unreachable!(),
            };
            let alg = [alg_prefix, alg_suffix].concat();

            match key.kind {
                KeyKind::Public => {
                    // Handle is PKCS#1 DER public key
                    let rsa = Rsa::<Public>::public_key_from_der_pkcs1(handle).or_throw(ctx)?;
                    obj.set("n", bn_to_b64(rsa.n()))?;
                    obj.set("e", bn_to_b64(rsa.e()))?;
                },
                KeyKind::Private => {
                    // Handle is PKCS#1 DER private key
                    let rsa = Rsa::<Private>::private_key_from_der(handle).or_throw(ctx)?;
                    obj.set("n", bn_to_b64(rsa.n()))?;
                    obj.set("e", bn_to_b64(rsa.e()))?;
                    obj.set("d", bn_to_b64(rsa.d()))?;
                    obj.set("p", bn_to_b64(rsa.p().unwrap()))?;
                    obj.set("q", bn_to_b64(rsa.q().unwrap()))?;
                    obj.set("dp", bn_to_b64(rsa.dmp1().unwrap()))?;
                    obj.set("dq", bn_to_b64(rsa.dmq1().unwrap()))?;
                    obj.set("qi", bn_to_b64(rsa.iqmp().unwrap()))?;
                },
                _ => unreachable!(),
            }

            obj.set("kty", "RSA")?;
            obj.set("alg", alg)?;
        },
        KeyAlgorithm::X25519 => {
            if key.kind == KeyKind::Private {
                // Handle is raw 32-byte secret
                let pkey = PKey::private_key_from_raw_bytes(handle, Id::X25519).or_throw(ctx)?;
                let raw_public = pkey.raw_public_key().or_throw(ctx)?;
                set_okp_jwk_props(name, &obj, Some(handle), &raw_public)?;
            } else {
                // Handle is raw 32-byte public key
                set_okp_jwk_props(name, &obj, None, handle)?;
            }
        },
        _ => return algorithm_export_error(ctx, &key.name, "jwk"),
    };

    Ok(obj)
}

fn curve_to_nid(curve: &EllipticCurve) -> Nid {
    match curve {
        EllipticCurve::P256 => Nid::X9_62_PRIME256V1,
        EllipticCurve::P384 => Nid::SECP384R1,
        EllipticCurve::P521 => Nid::SECP521R1,
    }
}

fn curve_byte_len(curve: &EllipticCurve) -> usize {
    match curve {
        EllipticCurve::P256 => 32,
        EllipticCurve::P384 => 48,
        EllipticCurve::P521 => 66,
    }
}

fn bn_to_b64(bn: &BigNumRef) -> String {
    bytes_to_b64_url_safe_string(&bn.to_vec())
}

fn bn_to_b64_padded(bn: &BigNumRef, len: usize) -> String {
    let mut bytes = bn.to_vec();
    // Pad to expected length
    while bytes.len() < len {
        bytes.insert(0, 0);
    }
    bytes_to_b64_url_safe_string(&bytes)
}

fn set_ec_public_coords(
    obj: &Object<'_>,
    x: &BigNum,
    y: &BigNum,
    curve: &EllipticCurve,
) -> Result<()> {
    let len = curve_byte_len(curve);
    obj.set("x", bn_to_b64_padded(x, len))?;
    obj.set("y", bn_to_b64_padded(y, len))?;
    Ok(())
}

fn set_okp_jwk_props(
    crv: &str,
    obj: &Object<'_>,
    private_key: Option<&[u8]>,
    public_key: &[u8],
) -> Result<()> {
    obj.set("kty", "OKP")?;
    obj.set("crv", crv)?;
    obj.set("x", bytes_to_b64_url_safe_string(public_key))?;
    if let Some(private_key) = private_key {
        obj.set("d", bytes_to_b64_url_safe_string(private_key))?;
    }
    Ok(())
}
