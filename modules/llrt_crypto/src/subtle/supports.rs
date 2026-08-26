// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use llrt_utils::bytes::ObjectBytes;
use rquickjs::{prelude::Opt, Array, Coerced, Ctx, FromJs, IntoJs, Result, Value};

use super::{
    algorithm_not_supported_error,
    crypto_key::KeyKind,
    derive_algorithm::DeriveAlgorithm,
    digest::supports_digest_algorithm,
    encapsulation::normalize_encapsulation_algorithm,
    encryption_algorithm::EncryptionAlgorithm,
    enforce_range_u32,
    key_algorithm::{
        EcAlgorithm, KeyAlgorithm, KeyAlgorithmMode, KeyAlgorithmWithUsages, KeyDerivation,
        KeyFormatData,
    },
    normalize_algorithm_name,
    sign_algorithm::SigningAlgorithm,
    to_name_and_maybe_object, EllipticCurve,
};

#[cfg(all(feature = "_subtle-full", feature = "crypto-openssl"))]
fn provider_supports_generate_key(algorithm: &KeyAlgorithm) -> bool {
    if let KeyAlgorithm::Rsa { modulus_length, .. } = algorithm {
        return *modulus_length >= 512;
    }
    true
}

#[cfg(all(feature = "_subtle-full", not(feature = "crypto-openssl")))]
fn provider_supports_generate_key(algorithm: &KeyAlgorithm) -> bool {
    if let KeyAlgorithm::Rsa { modulus_length, .. } = algorithm {
        return *modulus_length >= 1024;
    }
    true
}

#[cfg(not(feature = "_subtle-full"))]
fn provider_supports_generate_key(algorithm: &KeyAlgorithm) -> bool {
    match algorithm {
        KeyAlgorithm::ChaCha20Poly1305
        | KeyAlgorithm::MlDsa(_)
        | KeyAlgorithm::MlKem(_)
        | KeyAlgorithm::HybridKem(_) => true,
        #[cfg(feature = "crypto-graviola")]
        KeyAlgorithm::Aes { length, .. } => matches!(length, 128 | 256),
        #[cfg(feature = "crypto-graviola")]
        KeyAlgorithm::Hmac { .. } => true,
        _ => false,
    }
}

#[cfg(feature = "_subtle-full")]
fn provider_supports_sign(_algorithm: &SigningAlgorithm) -> bool {
    true
}

#[cfg(not(feature = "_subtle-full"))]
fn provider_supports_sign(algorithm: &SigningAlgorithm) -> bool {
    match algorithm {
        SigningAlgorithm::MlDsa { .. } => true,
        #[cfg(feature = "crypto-graviola")]
        SigningAlgorithm::Hmac => true,
        _ => false,
    }
}

#[cfg(feature = "_subtle-full")]
fn provider_supports_encryption(_algorithm: &EncryptionAlgorithm) -> bool {
    true
}

#[cfg(not(feature = "_subtle-full"))]
fn provider_supports_encryption(algorithm: &EncryptionAlgorithm) -> bool {
    match algorithm {
        EncryptionAlgorithm::ChaCha20Poly1305 { .. } => true,
        #[cfg(feature = "crypto-graviola")]
        EncryptionAlgorithm::AesGcm { tag_length, .. } => *tag_length == 128,
        _ => false,
    }
}

fn algorithm_name<'js>(ctx: &Ctx<'js>, algorithm: Value<'js>) -> Result<String> {
    let (name, _) = to_name_and_maybe_object(ctx, algorithm)?;
    Ok(normalize_algorithm_name(&name))
}

fn synthetic_key_usage(name: &str) -> Option<&'static str> {
    Some(match name {
        "AES-KW" => "wrapKey",
        "AES-CBC" | "AES-CTR" | "AES-GCM" | "ChaCha20-Poly1305" | "RSA-OAEP" => "encrypt",
        "ECDH" | "X25519" | "HKDF" | "PBKDF2" => "deriveKey",
        "ECDSA" | "Ed25519" | "HMAC" | "ML-DSA-44" | "ML-DSA-65" | "ML-DSA-87" | "RSA-PSS"
        | "RSASSA-PKCS1-v1_5" => "sign",
        "ML-KEM-512" | "ML-KEM-768" | "ML-KEM-1024" | "MLKEM768-P256" | "MLKEM768-X25519"
        | "MLKEM1024-P384" => "encapsulateKey",
        _ => return None,
    })
}

fn normalize_key_algorithm_with_synthetic_usage<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    algorithm: Value<'js>,
) -> Result<KeyAlgorithmWithUsages> {
    let name = algorithm_name(ctx, algorithm.clone())?;
    let Some(usage) = synthetic_key_usage(&name) else {
        return algorithm_not_supported_error(ctx);
    };
    let usages = Array::new(ctx.clone())?;
    usages.set(0, usage)?;
    KeyAlgorithm::from_js(ctx, mode, algorithm, usages)
}

fn webidl_algorithm_identifier<'js>(ctx: &Ctx<'js>, algorithm: Value<'js>) -> Result<Value<'js>> {
    if algorithm.is_object() {
        Ok(algorithm)
    } else {
        Coerced::<String>::from_js(ctx, algorithm)?.0.into_js(ctx)
    }
}

fn supports_generate_key<'js>(ctx: &Ctx<'js>, algorithm: Value<'js>) -> Result<bool> {
    let normalized =
        normalize_key_algorithm_with_synthetic_usage(ctx, KeyAlgorithmMode::Generate, algorithm)?;
    Ok(provider_supports_generate_key(&normalized.algorithm))
}

fn supports_import_key<'js>(ctx: &Ctx<'js>, algorithm: Value<'js>) -> Result<bool> {
    normalize_key_algorithm_with_synthetic_usage(ctx, KeyAlgorithmMode::ValidateImport, algorithm)?;
    Ok(cfg!(feature = "_subtle-full"))
}

fn supports_export_key<'js>(ctx: &Ctx<'js>, algorithm: Value<'js>) -> Result<bool> {
    let name = algorithm_name(ctx, algorithm)?;
    Ok(cfg!(feature = "_subtle-full")
        && synthetic_key_usage(&name).is_some()
        && !matches!(name.as_str(), "HKDF" | "PBKDF2"))
}

fn supports_encrypt<'js>(ctx: &Ctx<'js>, algorithm: Value<'js>) -> Result<bool> {
    let normalized = EncryptionAlgorithm::from_js(ctx, algorithm)?;
    let valid_parameters = match &normalized {
        EncryptionAlgorithm::AesCtr { counter, .. } => counter.len() == 16,
        EncryptionAlgorithm::AesGcm { iv, .. } => iv.len() == 12,
        EncryptionAlgorithm::ChaCha20Poly1305 { iv, tag_length, .. } => {
            iv.len() == 12 && *tag_length == 128
        },
        EncryptionAlgorithm::AesKw => false,
        _ => true,
    };
    Ok(valid_parameters && provider_supports_encryption(&normalized))
}

fn supports_wrap<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    additional: &SupportsAdditional<'js>,
    unwrap: bool,
) -> Result<bool> {
    if let SupportsAdditional::Algorithm(additional) = additional {
        let additional_supported = if unwrap {
            supports_import_key(ctx, additional.clone())?
        } else {
            supports_export_key(ctx, additional.clone())?
        };
        if !additional_supported {
            return Ok(false);
        }
    }

    let directly_supported = match algorithm_name(ctx, algorithm.clone()) {
        Ok(name) => name == "AES-KW",
        Err(_) => {
            ctx.catch();
            false
        },
    };
    if directly_supported {
        return Ok(cfg!(feature = "_subtle-full"));
    }

    let normalized = EncryptionAlgorithm::from_js(ctx, algorithm)?;
    let valid_parameters = match &normalized {
        EncryptionAlgorithm::AesCtr { counter, .. } => counter.len() == 16,
        EncryptionAlgorithm::AesGcm { iv, .. } => iv.len() == 12,
        EncryptionAlgorithm::ChaCha20Poly1305 { iv, tag_length, .. } => {
            iv.len() == 12 && *tag_length == 128
        },
        EncryptionAlgorithm::AesKw => false,
        _ => true,
    };
    if !valid_parameters || !cfg!(feature = "_subtle-full") {
        return Ok(false);
    }
    Ok(true)
}

fn supports_sign<'js>(ctx: &Ctx<'js>, algorithm: Value<'js>) -> Result<bool> {
    let normalized = SigningAlgorithm::from_js(ctx, algorithm)?;
    Ok(provider_supports_sign(&normalized))
}

enum SupportsAdditional<'js> {
    Length(Option<u32>),
    Algorithm(Value<'js>),
}

impl SupportsAdditional<'_> {
    fn length(&self) -> Option<u32> {
        match self {
            SupportsAdditional::Length(length) => *length,
            SupportsAdditional::Algorithm(_) => None,
        }
    }
}

fn supports_additional<'js>(
    ctx: &Ctx<'js>,
    value: Opt<Value<'js>>,
) -> Result<SupportsAdditional<'js>> {
    match value.0 {
        None => Ok(SupportsAdditional::Length(None)),
        Some(value) if value.is_null() || value.is_undefined() => {
            Ok(SupportsAdditional::Length(None))
        },
        Some(value) if value.is_string() || value.is_object() => {
            Ok(SupportsAdditional::Algorithm(value))
        },
        Some(value) => Ok(SupportsAdditional::Length(Some(enforce_range_u32(
            ctx, value, "length",
        )?))),
    }
}

fn supports_derive_bits_with_length<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    length: Option<u32>,
) -> Result<bool> {
    let normalized = DeriveAlgorithm::from_js(ctx, algorithm)?;
    Ok(cfg!(feature = "_subtle-full")
        && match normalized {
            DeriveAlgorithm::X25519 { .. } => length.is_none_or(|length| length <= 256),
            DeriveAlgorithm::Ecdh {
                curve,
                ec_algorithm,
                ..
            } => {
                if !matches!(ec_algorithm, EcAlgorithm::Ecdh) {
                    return Ok(false);
                }
                let maximum = match curve {
                    EllipticCurve::P256 => 256,
                    EllipticCurve::P384 => 384,
                    EllipticCurve::P521 => 528,
                };
                length.is_none_or(|length| length <= maximum)
            },
            DeriveAlgorithm::Derive(KeyDerivation::Hkdf { hash, .. }) => {
                length.is_some_and(|length| {
                    length.is_multiple_of(8) && length <= (hash.digest_len() * 8 * 255) as u32
                })
            },
            DeriveAlgorithm::Derive(KeyDerivation::Pbkdf2 { iterations, .. }) => {
                iterations != 0 && length.is_some_and(|length| length.is_multiple_of(8))
            },
        })
}

fn supports_derive_bits<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    additional: &SupportsAdditional<'js>,
) -> Result<bool> {
    supports_derive_bits_with_length(ctx, algorithm, additional.length())
}

fn supports_derive_key<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    additional: &SupportsAdditional<'js>,
) -> Result<bool> {
    let SupportsAdditional::Algorithm(target) = additional else {
        algorithm_name(ctx, algorithm)?;
        return Ok(false);
    };
    if !supports_import_key(ctx, target.clone())? {
        return Ok(false);
    }
    let target = KeyAlgorithm::from_js(
        ctx,
        KeyAlgorithmMode::Derive,
        target.clone(),
        Array::new(ctx.clone())?,
    )?;
    let target_length = match target.algorithm {
        KeyAlgorithm::Aes { length, .. } => Some(u32::from(length)),
        KeyAlgorithm::ChaCha20Poly1305 => Some(256),
        KeyAlgorithm::Hmac { length, .. } => Some(length),
        KeyAlgorithm::HkdfImport | KeyAlgorithm::Pbkdf2Import => None,
        _ => return Ok(false),
    };
    supports_derive_bits_with_length(ctx, algorithm, target_length)
}

fn supports_shared_key<'js>(
    ctx: &Ctx<'js>,
    algorithm: Value<'js>,
    shared_key_length: usize,
) -> Result<bool> {
    let name = algorithm_name(ctx, algorithm.clone())?;
    let Some(usage) = synthetic_key_usage(&name) else {
        return algorithm_not_supported_error(ctx);
    };
    let usages = Array::new(ctx.clone())?;
    usages.set(0, usage)?;
    let mut kind = KeyKind::Public;
    let mut data = Vec::new();
    KeyAlgorithm::from_js(
        ctx,
        KeyAlgorithmMode::Import {
            format: KeyFormatData::RawSecret(ObjectBytes::Vec(vec![0; shared_key_length])),
            kind: &mut kind,
            data: &mut data,
        },
        algorithm,
        usages,
    )?;
    Ok(matches!(kind, KeyKind::Secret))
}

fn supports_get_public_key(name: &str) -> bool {
    let supported = matches!(
        name,
        "ECDH"
            | "ECDSA"
            | "Ed25519"
            | "X25519"
            | "RSA-OAEP"
            | "RSA-PSS"
            | "RSASSA-PKCS1-v1_5"
            | "ML-DSA-44"
            | "ML-DSA-65"
            | "ML-DSA-87"
            | "ML-KEM-512"
            | "ML-KEM-768"
            | "ML-KEM-1024"
            | "MLKEM768-P256"
            | "MLKEM768-X25519"
            | "MLKEM1024-P384"
    );
    supported
        && (cfg!(feature = "_subtle-full")
            || matches!(
                name,
                "ML-DSA-44"
                    | "ML-DSA-65"
                    | "ML-DSA-87"
                    | "ML-KEM-512"
                    | "ML-KEM-768"
                    | "ML-KEM-1024"
                    | "MLKEM768-P256"
                    | "MLKEM768-X25519"
                    | "MLKEM1024-P384"
            ))
}

fn supports_inner<'js>(
    ctx: &Ctx<'js>,
    operation: &str,
    algorithm: Value<'js>,
    additional: &SupportsAdditional<'js>,
) -> Result<bool> {
    match operation {
        "generateKey" => supports_generate_key(ctx, algorithm),
        "importKey" => supports_import_key(ctx, algorithm),
        "exportKey" => supports_export_key(ctx, algorithm),
        "sign" | "verify" => supports_sign(ctx, algorithm),
        "encrypt" | "decrypt" => supports_encrypt(ctx, algorithm),
        "wrapKey" => supports_wrap(ctx, algorithm, additional, false),
        "unwrapKey" => supports_wrap(ctx, algorithm, additional, true),
        "deriveBits" => supports_derive_bits(ctx, algorithm, additional),
        "deriveKey" => supports_derive_key(ctx, algorithm, additional),
        "digest" => supports_digest_algorithm(ctx, algorithm),
        "encapsulateKey" | "decapsulateKey" => {
            let normalized = normalize_encapsulation_algorithm(ctx, algorithm.clone())?;
            if let SupportsAdditional::Algorithm(shared) = additional {
                if !supports_shared_key(ctx, shared.clone(), normalized.shared_key_length())? {
                    return Ok(false);
                }
                normalize_encapsulation_algorithm(ctx, algorithm)?;
                Ok(true)
            } else {
                Ok(true)
            }
        },
        "encapsulateBits" | "decapsulateBits" => {
            normalize_encapsulation_algorithm(ctx, algorithm)?;
            Ok(true)
        },
        "getPublicKey" => Ok(supports_get_public_key(&algorithm_name(ctx, algorithm)?)),
        _ => Ok(false),
    }
}

pub fn subtle_supports<'js>(
    ctx: Ctx<'js>,
    operation: Coerced<String>,
    algorithm: Value<'js>,
    additional: Opt<Value<'js>>,
) -> Result<bool> {
    let algorithm = webidl_algorithm_identifier(&ctx, algorithm)?;
    let additional = supports_additional(&ctx, additional)?;
    let operation = operation.0;
    Ok(
        match supports_inner(&ctx, &operation, algorithm, &additional) {
            Ok(supported) => supported,
            Err(_) => {
                ctx.catch();
                false
            },
        },
    )
}

#[cfg(all(test, any(feature = "crypto-ring", feature = "crypto-graviola")))]
mod tests {
    use super::*;
    #[cfg(feature = "crypto-ring")]
    use crate::provider::MlDsaVariant;
    use crate::subtle::key_algorithm::AesAlgorithm;

    #[cfg(feature = "crypto-ring")]
    #[test]
    fn pure_ring_reports_only_provider_independent_key_operations() {
        assert!(provider_supports_generate_key(&KeyAlgorithm::MlDsa(
            MlDsaVariant::MlDsa44
        )));
        assert!(!provider_supports_generate_key(&KeyAlgorithm::Aes {
            length: 256,
            algorithm: AesAlgorithm::Gcm,
        }));
        assert!(provider_supports_sign(&SigningAlgorithm::MlDsa {
            variant: MlDsaVariant::MlDsa44,
            context: Box::default(),
        }));
        assert!(!provider_supports_sign(&SigningAlgorithm::Hmac));
        assert!(supports_get_public_key("ML-KEM-768"));
        assert!(!supports_get_public_key("ECDSA"));
    }

    #[cfg(feature = "crypto-graviola")]
    #[test]
    fn pure_graviola_reports_its_partial_symmetric_support() {
        assert!(provider_supports_generate_key(&KeyAlgorithm::Aes {
            length: 128,
            algorithm: AesAlgorithm::Gcm,
        }));
        assert!(!provider_supports_generate_key(&KeyAlgorithm::Aes {
            length: 192,
            algorithm: AesAlgorithm::Gcm,
        }));
        assert!(provider_supports_sign(&SigningAlgorithm::Hmac));
        assert!(supports_get_public_key("ML-KEM-768"));
        assert!(!supports_get_public_key("ECDSA"));
        assert!(provider_supports_encryption(&EncryptionAlgorithm::AesGcm {
            iv: vec![0; 12].into_boxed_slice(),
            tag_length: 128,
            additional_data: None,
        }));
        for tag_length in [32, 64, 96, 104, 112, 120] {
            assert!(!provider_supports_encryption(
                &EncryptionAlgorithm::AesGcm {
                    iv: vec![0; 12].into_boxed_slice(),
                    tag_length,
                    additional_data: None,
                }
            ));
        }
    }
}
