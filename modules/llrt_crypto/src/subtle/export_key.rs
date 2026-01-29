// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Unified key export implementation using CryptoProvider trait.

use llrt_encoding::bytes_to_b64_url_safe_string;
use llrt_utils::result::ResultExt;
use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Object, Result};

use crate::provider::CryptoProvider;
use crate::CRYPTO_PROVIDER;

use super::{
    crypto_key::KeyKind,
    key_algorithm::{KeyAlgorithm, KeyFormat},
    CryptoKey,
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
    }
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
    }
    match &key.algorithm {
        KeyAlgorithm::Aes { .. } | KeyAlgorithm::Hmac { .. } => Ok(key.handle.to_vec()),
        KeyAlgorithm::Ec { curve, .. } => CRYPTO_PROVIDER
            .export_ec_public_key_sec1(&key.handle, *curve, false)
            .or_throw(ctx),
        KeyAlgorithm::Ed25519 => CRYPTO_PROVIDER
            .export_okp_public_key_raw(&key.handle, false)
            .or_throw(ctx),
        KeyAlgorithm::X25519 => CRYPTO_PROVIDER
            .export_okp_public_key_raw(&key.handle, false)
            .or_throw(ctx),
        KeyAlgorithm::Rsa { .. } => CRYPTO_PROVIDER
            .export_rsa_public_key_pkcs1(&key.handle)
            .or_throw(ctx),
        _ => algorithm_export_error(ctx, &key.name, "raw"),
    }
}

fn export_pkcs8(ctx: &Ctx<'_>, key: &CryptoKey) -> Result<Vec<u8>> {
    if key.kind != KeyKind::Private {
        return Err(Exception::throw_type(
            ctx,
            "Public or Secret Crypto keys can't be exported as pkcs8 format",
        ));
    }
    match &key.algorithm {
        KeyAlgorithm::Ec { curve, .. } => CRYPTO_PROVIDER
            .export_ec_private_key_pkcs8(&key.handle, *curve)
            .or_throw(ctx),
        KeyAlgorithm::Ed25519 => CRYPTO_PROVIDER
            .export_okp_private_key_pkcs8(
                &key.handle,
                const_oid::db::rfc8410::ID_ED_25519.as_bytes(),
            )
            .or_throw(ctx),
        KeyAlgorithm::X25519 => CRYPTO_PROVIDER
            .export_okp_private_key_pkcs8(
                &key.handle,
                const_oid::db::rfc8410::ID_X_25519.as_bytes(),
            )
            .or_throw(ctx),
        KeyAlgorithm::Rsa { .. } => CRYPTO_PROVIDER
            .export_rsa_private_key_pkcs8(&key.handle)
            .or_throw(ctx),
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
    match &key.algorithm {
        KeyAlgorithm::Ec { curve, .. } => CRYPTO_PROVIDER
            .export_ec_public_key_spki(&key.handle, *curve)
            .or_throw(ctx),
        KeyAlgorithm::Ed25519 => CRYPTO_PROVIDER
            .export_okp_public_key_spki(&key.handle, const_oid::db::rfc8410::ID_ED_25519.as_bytes())
            .or_throw(ctx),
        KeyAlgorithm::X25519 => CRYPTO_PROVIDER
            .export_okp_public_key_spki(&key.handle, const_oid::db::rfc8410::ID_X_25519.as_bytes())
            .or_throw(ctx),
        KeyAlgorithm::Rsa { .. } => CRYPTO_PROVIDER
            .export_rsa_public_key_spki(&key.handle)
            .or_throw(ctx),
        _ => algorithm_export_error(ctx, &key.name, "spki"),
    }
}

fn export_jwk<'js>(ctx: &Ctx<'js>, key: &CryptoKey) -> Result<Object<'js>> {
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
            let suffix = &key.name[("AES-".len())..];
            obj.set("kty", "oct")?;
            obj.set("k", bytes_to_b64_url_safe_string(&key.handle))?;
            obj.set("alg", [prefix, suffix].concat())?;
        },
        KeyAlgorithm::Hmac { hash, .. } => {
            obj.set("kty", "oct")?;
            obj.set("alg", ["HS", &hash.as_str()[4..]].concat())?;
            obj.set("k", bytes_to_b64_url_safe_string(&key.handle))?;
        },
        KeyAlgorithm::Ec { curve, .. } => {
            let jwk = CRYPTO_PROVIDER
                .export_ec_jwk(&key.handle, *curve, key.kind == KeyKind::Private)
                .or_throw(ctx)?;
            obj.set("kty", "EC")?;
            obj.set("crv", curve.as_str())?;
            obj.set("x", bytes_to_b64_url_safe_string(&jwk.x))?;
            obj.set("y", bytes_to_b64_url_safe_string(&jwk.y))?;
            if let Some(d) = jwk.d {
                obj.set("d", bytes_to_b64_url_safe_string(&d))?;
            }
        },
        KeyAlgorithm::Ed25519 => {
            let jwk = CRYPTO_PROVIDER
                .export_okp_jwk(&key.handle, key.kind == KeyKind::Private, true)
                .or_throw(ctx)?;
            obj.set("kty", "OKP")?;
            obj.set("crv", "Ed25519")?;
            obj.set("x", bytes_to_b64_url_safe_string(&jwk.x))?;
            if let Some(d) = jwk.d {
                obj.set("d", bytes_to_b64_url_safe_string(&d))?;
            }
        },
        KeyAlgorithm::X25519 => {
            let jwk = CRYPTO_PROVIDER
                .export_okp_jwk(&key.handle, key.kind == KeyKind::Private, false)
                .or_throw(ctx)?;
            obj.set("kty", "OKP")?;
            obj.set("crv", "X25519")?;
            obj.set("x", bytes_to_b64_url_safe_string(&jwk.x))?;
            if let Some(d) = jwk.d {
                obj.set("d", bytes_to_b64_url_safe_string(&d))?;
            }
        },
        KeyAlgorithm::Rsa { hash, .. } => {
            let jwk = CRYPTO_PROVIDER
                .export_rsa_jwk(&key.handle, key.kind == KeyKind::Private)
                .or_throw(ctx)?;
            let alg_suffix = hash.as_numeric_str();
            let alg_prefix = match key.name.as_ref() {
                "RSASSA-PKCS1-v1_5" => "RS",
                "RSA-PSS" => "PS",
                "RSA-OAEP" => "RSA-OAEP-",
                _ => unreachable!(),
            };
            obj.set("kty", "RSA")?;
            obj.set("n", bytes_to_b64_url_safe_string(&jwk.n))?;
            obj.set("e", bytes_to_b64_url_safe_string(&jwk.e))?;
            obj.set("alg", [alg_prefix, alg_suffix].concat())?;
            if let Some(d) = jwk.d {
                obj.set("d", bytes_to_b64_url_safe_string(&d))?;
                obj.set("p", bytes_to_b64_url_safe_string(&jwk.p.unwrap()))?;
                obj.set("q", bytes_to_b64_url_safe_string(&jwk.q.unwrap()))?;
                obj.set("dp", bytes_to_b64_url_safe_string(&jwk.dp.unwrap()))?;
                obj.set("dq", bytes_to_b64_url_safe_string(&jwk.dq.unwrap()))?;
                obj.set("qi", bytes_to_b64_url_safe_string(&jwk.qi.unwrap()))?;
            }
        },
        _ => return algorithm_export_error(ctx, &key.name, "jwk"),
    }
    Ok(obj)
}
