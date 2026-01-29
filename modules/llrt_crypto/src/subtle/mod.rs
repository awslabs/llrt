// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod crypto_key;
mod derive;
mod derive_algorithm;
mod digest;
mod encryption;
mod encryption_algorithm;
#[cfg(feature = "_rustcrypto")]
mod export_key;
#[cfg(all(feature = "openssl", not(feature = "_rustcrypto")))]
mod export_key_openssl;
mod generate_key;
#[cfg(feature = "_rustcrypto")]
mod import_key;
#[cfg(all(feature = "openssl", not(feature = "_rustcrypto")))]
mod import_key_openssl;
#[cfg(any(feature = "_rustcrypto", feature = "_subtle-full"))]
mod key_algorithm;
mod sign;
mod sign_algorithm;
mod verify;
#[cfg(feature = "_rustcrypto")]
mod wrapping;
#[cfg(all(feature = "openssl", not(feature = "_rustcrypto")))]
mod wrapping_openssl;

pub use crypto_key::CryptoKey;
pub use derive::subtle_derive_bits;
pub use derive::subtle_derive_key;
pub use digest::subtle_digest;
pub use encryption::subtle_decrypt;
pub use encryption::subtle_encrypt;
#[cfg(feature = "_rustcrypto")]
pub use export_key::subtle_export_key;
#[cfg(all(feature = "openssl", not(feature = "_rustcrypto")))]
pub use export_key_openssl::subtle_export_key;
pub use generate_key::subtle_generate_key;
#[cfg(feature = "_rustcrypto")]
pub use import_key::subtle_import_key;
#[cfg(all(feature = "openssl", not(feature = "_rustcrypto")))]
pub use import_key_openssl::subtle_import_key;
#[cfg(any(feature = "_rustcrypto", feature = "_subtle-full"))]
use key_algorithm::KeyAlgorithm;
pub use sign::subtle_sign;
pub use verify::subtle_verify;
#[cfg(feature = "_rustcrypto")]
pub use wrapping::subtle_unwrap_key;
#[cfg(feature = "_rustcrypto")]
pub use wrapping::subtle_wrap_key;
#[cfg(all(feature = "openssl", not(feature = "_rustcrypto")))]
pub use wrapping_openssl::subtle_unwrap_key;
#[cfg(all(feature = "openssl", not(feature = "_rustcrypto")))]
pub use wrapping_openssl::subtle_wrap_key;

// Stub implementations for limited crypto providers (not openssl, not rustcrypto)
#[cfg(not(any(feature = "_rustcrypto", feature = "_subtle-full")))]
mod key_algorithm;
#[cfg(not(any(feature = "_rustcrypto", feature = "_subtle-full")))]
use key_algorithm::KeyAlgorithm;

use llrt_utils::{object::ObjectExt, str_enum};
use rquickjs::{atom::PredefinedAtom, Ctx, Exception, Object, Result, Value};

use crate::provider::{CryptoProvider, SimpleDigest};

use crate::sha_hash::ShaAlgorithm;

#[rquickjs::class]
#[derive(rquickjs::JsLifetime, rquickjs::class::Trace)]
pub struct SubtleCrypto {}

#[rquickjs::methods]
impl SubtleCrypto {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>) -> Result<Self> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(SubtleCrypto)
    }
}

// AES variant types - only available when _rustcrypto feature is enabled
#[cfg(feature = "_rustcrypto")]
mod aes_variants;
#[cfg(feature = "_rustcrypto")]
pub use aes_variants::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EllipticCurve {
    P256,
    P384,
    P521,
}

str_enum!(EllipticCurve,P256 => "P-256", P384 => "P-384", P521 => "P-521");

pub enum EncryptionMode {
    Encryption,
    #[allow(dead_code)]
    Wrapping(u8), //padding byte
}

pub fn rsa_hash_digest<'a>(
    ctx: &Ctx<'_>,
    key: &'a CryptoKey,
    data: &'a [u8],
    algorithm_name: &str,
) -> Result<(&'a ShaAlgorithm, Vec<u8>)> {
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

    let mut hasher = crate::CRYPTO_PROVIDER.digest(*hash);
    hasher.update(data);
    let digest = hasher.finalize();

    Ok((hash, digest))
}

pub fn validate_aes_length(
    ctx: &Ctx<'_>,
    key: &CryptoKey,
    handle: &[u8],
    expected_algorithm: &str,
) -> Result<()> {
    let length = match key.algorithm {
        KeyAlgorithm::Aes { length } => length,
        _ => return algorithm_mismatch_error(ctx, expected_algorithm),
    };
    if length != handle.len() as u16 * 8 {
        return Err(Exception::throw_message(
            ctx,
            &[
                "Invalid key handle length for ",
                expected_algorithm,
                ". Expected ",
                &length.to_string(),
                " bits, found ",
                &handle.len().to_string(),
                " bits",
            ]
            .concat(),
        ));
    }
    Ok(())
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

// Stub implementations for providers without rustcrypto or openssl
#[cfg(not(any(feature = "_rustcrypto", feature = "openssl")))]
mod stubs;
#[cfg(not(any(feature = "_rustcrypto", feature = "openssl")))]
pub use stubs::subtle_export_key;
#[cfg(not(any(feature = "_rustcrypto", feature = "openssl")))]
pub use stubs::subtle_import_key;
#[cfg(not(any(feature = "_rustcrypto", feature = "openssl")))]
pub use stubs::subtle_unwrap_key;
#[cfg(not(any(feature = "_rustcrypto", feature = "openssl")))]
pub use stubs::subtle_wrap_key;
