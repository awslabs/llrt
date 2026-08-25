// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::rc::Rc;

#[cfg(feature = "_subtle-full")]
use der::{
    asn1::{BitStringRef, OctetString, OctetStringRef},
    Decode, Encode,
};
#[cfg(feature = "_subtle-full")]
use ed25519_dalek::SigningKey;
#[cfg(feature = "_subtle-full")]
use llrt_encoding::bytes_from_b64_url_safe;
use llrt_exceptions::DOMException;
#[cfg(feature = "_subtle-full")]
use llrt_utils::result::ResultExt;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, str_enum};
#[cfg(feature = "_subtle-full")]
use pkcs8::PrivateKeyInfoRef;
use rquickjs::{
    atom::PredefinedAtom, Array, Coerced, Ctx, Exception, FromJs, Object, Result, TypedArray, Value,
};
#[cfg(feature = "_subtle-full")]
use spki::{AlgorithmIdentifier, ObjectIdentifier};
#[cfg(feature = "_subtle-full")]
use x25519_dalek::{PublicKey, StaticSecret};

use crate::{
    hash::HashAlgorithm,
    provider::{hmac_length_is_byte_aligned, parse_rsa_public_exponent, MAX_HMAC_KEY_LENGTH_BITS},
};

#[cfg(feature = "_subtle-full")]
use super::{algorithm_mismatch_error, util::DataError};
use super::{
    algorithm_not_supported_error,
    crypto_key::KeyKind,
    enforce_range_u16, enforce_range_u32, get_optional_dictionary_value,
    get_required_dictionary_value, normalize_algorithm_name, to_name_and_maybe_object,
    util::{NotSupportedError, ResultDomExt},
    EllipticCurve,
};

#[derive(Clone, Copy, PartialEq)]
pub enum KeyUsage {
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    DeriveKey,
    DeriveBits,
    WrapKey,
    UnwrapKey,
}

impl TryFrom<&str> for KeyUsage {
    type Error = String;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        Ok(match s {
            "encrypt" => KeyUsage::Encrypt,
            "decrypt" => KeyUsage::Decrypt,
            "wrapKey" => KeyUsage::WrapKey,
            "unwrapKey" => KeyUsage::UnwrapKey,
            "sign" => KeyUsage::Sign,
            "verify" => KeyUsage::Verify,
            "deriveKey" => KeyUsage::DeriveKey,
            "deriveBits" => KeyUsage::DeriveBits,
            _ => return Err(["Invalid key usage: ", s].concat()),
        })
    }
}

impl KeyUsage {
    const CANONICAL_ORDER: [Self; 8] = [
        Self::Encrypt,
        Self::Decrypt,
        Self::Sign,
        Self::Verify,
        Self::DeriveKey,
        Self::DeriveBits,
        Self::WrapKey,
        Self::UnwrapKey,
    ];

    const fn as_str(self) -> &'static str {
        match self {
            Self::Encrypt => "encrypt",
            Self::Decrypt => "decrypt",
            Self::Sign => "sign",
            Self::Verify => "verify",
            Self::DeriveKey => "deriveKey",
            Self::DeriveBits => "deriveBits",
            Self::WrapKey => "wrapKey",
            Self::UnwrapKey => "unwrapKey",
        }
    }

    fn classify_and_check_usages<'js>(
        ctx: &Ctx<'js>,
        key_usage_algorithm: KeyUsageAlgorithm,
        key_usages: &Array<'js>,
        private_usages: &mut Vec<String>,
        public_usages: &mut Vec<String>,
        kind: Option<&KeyKind>,
    ) -> Result<()> {
        let (mut private_usages_mask, mut public_usages_mask) = key_usage_algorithm.masks();

        match kind {
            Some(KeyKind::Private) => public_usages_mask = 0,
            Some(KeyKind::Secret) | Some(KeyKind::Public) => private_usages_mask = 0,
            None => {},
        };

        let allowed_usages = private_usages_mask | public_usages_mask;

        let mut generated_public_usages = Vec::with_capacity(4);
        let mut generated_private_usages = Vec::with_capacity(4);

        let mut has_any_usages = false;
        let mut seen_usages = 0;

        for usage in key_usages.iter::<String>() {
            has_any_usages = true;
            let value = usage?;
            let usage = KeyUsage::try_from(value.as_str()).map_err(|_| {
                DOMException::syntax_error(ctx, ["Invalid key usage '", &value, "'"].concat())
            })?;
            let usage = usage.mask();
            if allowed_usages & usage != usage {
                return Err(DOMException::syntax_error(
                    ctx,
                    ["Invalid key usage '", &value, "'"].concat(),
                ));
            }
            seen_usages |= usage;
        }

        for usage in Self::CANONICAL_ORDER {
            let usage_mask = usage.mask();
            if seen_usages & usage_mask == 0 {
                continue;
            }
            let value = usage.as_str().to_string();
            if private_usages_mask == public_usages_mask {
                generated_private_usages.push(value.clone());
                generated_public_usages.push(value);
            } else if private_usages_mask & usage_mask == usage_mask {
                generated_private_usages.push(value);
            } else if public_usages_mask & usage_mask == usage_mask {
                generated_public_usages.push(value);
            }
        }

        *private_usages = generated_private_usages;
        *public_usages = generated_public_usages;

        if !has_any_usages
            && key_usage_algorithm.requires_non_empty_usages()
            && !matches!(kind, Some(KeyKind::Public))
        {
            return Err(DOMException::syntax_error(ctx, "Key usages empty"));
        }

        if private_usages != public_usages {
            let valid_usage = match kind {
                Some(KeyKind::Secret) | Some(KeyKind::Public) => {
                    private_usages.is_empty() && !public_usages.is_empty()
                },
                Some(KeyKind::Private) => !private_usages.is_empty() && public_usages.is_empty(),
                None => true,
            };

            if !valid_usage {
                return Err(DOMException::syntax_error(ctx, "Invalid key usage"));
            }
        }

        Ok(())
    }

    const fn mask(self) -> u16 {
        1 << self as u16
    }
}

#[repr(u16)]
#[derive(Clone, Copy)]
pub enum KeyUsageAlgorithm {
    //single mask algorithms (symmetric)
    AesKw = KeyUsage::WrapKey.mask() | KeyUsage::UnwrapKey.mask(),
    //all non-KW AES
    Symmetric = (KeyUsage::Encrypt.mask())
        | (KeyUsage::Decrypt.mask())
        | (KeyUsage::WrapKey.mask())
        | (KeyUsage::UnwrapKey.mask()),

    Hmac = (KeyUsage::Sign.mask()) | (KeyUsage::Verify.mask()),

    // asymmetric derive algorithms - use high bits as private usages
    // ECDH/X25519
    DeriveAsymmetric = ((KeyUsage::DeriveKey.mask() | KeyUsage::DeriveBits.mask()) << 8),

    // HKDF/PBKDF2
    DeriveSymmetric = KeyUsage::DeriveKey.mask() | KeyUsage::DeriveBits.mask(),

    RsaOaep = ((KeyUsage::Decrypt.mask() | KeyUsage::UnwrapKey.mask()) << 8) //private
    | KeyUsage::Encrypt.mask() | KeyUsage::WrapKey.mask(), //public

    //ECDSA, ED25519, all non-OEAP RSA
    Sign = (KeyUsage::Sign.mask() << 8) //private
        | KeyUsage::Verify.mask(), //public
}
impl KeyUsageAlgorithm {
    fn masks(&self) -> (u16, u16) {
        let value = *self as u16;
        let private_mask = value >> 8;
        let public_mask = value & 0xFF;
        (private_mask, public_mask)
    }

    fn requires_non_empty_usages(self) -> bool {
        matches!(
            self,
            Self::Symmetric
                | Self::AesKw
                | Self::Hmac
                | Self::DeriveAsymmetric
                | Self::DeriveSymmetric
                | Self::Sign
                | Self::RsaOaep
        )
    }
}

#[derive(Debug, Clone)]
pub enum KeyDerivation {
    Hkdf {
        hash: HashAlgorithm,
        salt: Box<[u8]>,
        info: Box<[u8]>,
    },
    Pbkdf2 {
        hash: HashAlgorithm,
        salt: Box<[u8]>,
        iterations: u32,
    },
}

impl KeyDerivation {
    pub fn for_hkdf_object<'js>(ctx: &Ctx<'js>, obj: Object<'js>) -> Result<Self> {
        let hash = extract_sha_hash(ctx, &obj)?;

        let salt = obj
            .get_required::<_, ObjectBytes>("salt", "algorithm")?
            .into_bytes(ctx)?
            .into_boxed_slice();

        let info = obj
            .get_required::<_, ObjectBytes>("info", "algorithm")?
            .into_bytes(ctx)?
            .into_boxed_slice();

        Ok(KeyDerivation::Hkdf { hash, salt, info })
    }

    pub fn for_pbkf2_object<'js>(ctx: &&Ctx<'js>, obj: Object<'js>) -> Result<Self> {
        let hash = extract_sha_hash(ctx, &obj)?;

        let salt = obj
            .get_required::<_, ObjectBytes>("salt", "algorithm")?
            .into_bytes(ctx)?
            .into_boxed_slice();

        let value = get_required_dictionary_value(&obj, "iterations", "algorithm")?;
        let iterations = enforce_range_u32(ctx, value, "iterations")?;
        Ok(KeyDerivation::Pbkdf2 {
            hash,
            salt,
            iterations,
        })
    }
}

#[derive(Debug, Clone)]
pub enum EcAlgorithm {
    Ecdh,
    Ecdsa,
}

#[derive(PartialEq, Debug, Clone)]
pub enum AesAlgorithm {
    Cbc,
    Ctr,
    Gcm,
    Kw,
}

#[derive(Debug, Clone)]
pub enum KeyAlgorithm {
    Aes {
        length: u16,
        algorithm: AesAlgorithm,
    },
    Ec {
        curve: EllipticCurve,
        algorithm: EcAlgorithm,
    },
    X25519,
    Ed25519,
    Hmac {
        hash: HashAlgorithm,
        length: u32,
    },
    Rsa {
        modulus_length: u32,
        public_exponent: Rc<Box<[u8]>>,
        hash: HashAlgorithm,
    },
    Derive(KeyDerivation),
    HkdfImport,
    Pbkdf2Import,
}

pub enum KeyFormat {
    Jwk,
    Raw,
    Spki,
    Pkcs8,
}

str_enum!(KeyFormat, Jwk => "jwk", Raw => "raw", Spki => "spki", Pkcs8 => "pkcs8");

impl<'js> FromJs<'js> for KeyFormat {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let string = Coerced::<String>::from_js(ctx, value)?.0;
        Self::try_from(string.as_str()).map_err(|_| {
            Exception::throw_type(ctx, &format!("'{string}' is not a valid KeyFormat"))
        })
    }
}

#[derive(PartialEq)]
pub enum KeyFormatData<'js> {
    Jwk(Object<'js>),
    Raw(ObjectBytes<'js>),
    Spki(ObjectBytes<'js>),
    Pkcs8(ObjectBytes<'js>),
}

#[derive(PartialEq)]
pub enum KeyAlgorithmMode<'a, 'js> {
    Import {
        format: KeyFormatData<'js>,
        kind: &'a mut KeyKind,
        data: &'a mut Vec<u8>,
    },
    ValidateImport,
    Generate,
    Derive,
}

pub struct KeyAlgorithmWithUsages {
    pub name: String,
    pub algorithm: KeyAlgorithm,
    pub public_usages: Vec<String>,
    pub private_usages: Vec<String>,
}

fn from_ed25519<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    algorithm_name: &str,
    usages: &Array<'js>,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) -> Result<KeyAlgorithm> {
    #[cfg(feature = "_subtle-full")]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        algorithm_name: &str,
    ) -> Result<Option<KeyKind>> {
        if let KeyAlgorithmMode::Import { format, kind, data } = mode {
            import_okp_key(
                ctx,
                format,
                kind,
                data,
                const_oid::db::rfc8410::ID_ED_25519,
                algorithm_name,
                true,
            )?;
            Ok(Some(*kind))
        } else {
            Ok(None)
        }
    }

    #[cfg(not(feature = "_subtle-full"))]
    #[inline]
    fn import<'js>(
        _ctx: &Ctx<'js>,
        _mode: KeyAlgorithmMode<'_, 'js>,
        _algorithm_name: &str,
    ) -> Result<Option<KeyKind>> {
        Ok(None)
    }

    let key_kind = import(ctx, mode, algorithm_name)?;
    KeyUsage::classify_and_check_usages(
        ctx,
        KeyUsageAlgorithm::Sign,
        usages,
        private_usages,
        public_usages,
        key_kind.as_ref(),
    )?;
    Ok(KeyAlgorithm::Ed25519)
}

fn from_x25519<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    algorithm_name: &str,
    usages: &Array<'js>,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) -> Result<KeyAlgorithm> {
    #[cfg(feature = "_subtle-full")]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        algorithm_name: &str,
    ) -> Result<Option<KeyKind>> {
        if let KeyAlgorithmMode::Import { format, kind, data } = mode {
            import_okp_key(
                ctx,
                format,
                kind,
                data,
                const_oid::db::rfc8410::ID_X_25519,
                algorithm_name,
                false,
            )?;
            Ok(Some(*kind))
        } else {
            Ok(None)
        }
    }

    #[cfg(not(feature = "_subtle-full"))]
    #[inline]
    fn import<'js>(
        _ctx: &Ctx<'js>,
        _mode: KeyAlgorithmMode<'_, 'js>,
        _algorithm_name: &str,
    ) -> Result<Option<KeyKind>> {
        Ok(None)
    }

    let key_kind = import(ctx, mode, algorithm_name)?;
    KeyUsage::classify_and_check_usages(
        ctx,
        KeyUsageAlgorithm::DeriveAsymmetric,
        usages,
        private_usages,
        public_usages,
        key_kind.as_ref(),
    )?;
    Ok(KeyAlgorithm::X25519)
}

fn from_aes<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    obj: Result<Object<'js>>,
    algorithm_name: &str,
    usages: &Array<'js>,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) -> Result<KeyAlgorithm> {
    #[cfg(feature = "_subtle-full")]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: Result<Object<'js>>,
        algorithm_name: &str,
    ) -> Result<(u16, Option<KeyKind>)> {
        match mode {
            KeyAlgorithmMode::Import { data, format, kind } => {
                let length =
                    import_symmetric_key(ctx, format, kind, data, algorithm_name, None)? as u16;
                Ok((length, Some(*kind)))
            },
            KeyAlgorithmMode::ValidateImport => Ok((128, None)),
            _ => {
                let value = get_required_dictionary_value(&obj?, "length", "algorithm")?;
                let length = enforce_range_u16(ctx, value, "length")?;
                Ok((length, None))
            },
        }
    }

    #[cfg(not(feature = "_subtle-full"))]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: Result<Object<'js>>,
        _algorithm_name: &str,
    ) -> Result<(u16, Option<KeyKind>)> {
        if matches!(mode, KeyAlgorithmMode::ValidateImport) {
            return Ok((128, None));
        }
        let value = get_required_dictionary_value(&obj?, "length", "algorithm")?;
        let length = enforce_range_u16(ctx, value, "length")?;
        Ok((length, None))
    }

    let (length, key_kind) = import(ctx, mode, obj, algorithm_name)?;

    if !matches!(length, 128 | 192 | 256) {
        return Err(DOMException::operation_error(
            ctx,
            format!(
                "Algorithm 'length' must be one of: 128, 192, or 256 = {}",
                length
            ),
        ));
    }

    let algorithm = match algorithm_name {
        "AES-CBC" => AesAlgorithm::Cbc,
        "AES-CTR" => AesAlgorithm::Ctr,
        "AES-GCM" => AesAlgorithm::Gcm,
        "AES-KW" => AesAlgorithm::Kw,
        _ => return Err(DOMException::operation_error(ctx, "Invalid algorithm name")),
    };

    KeyUsage::classify_and_check_usages(
        ctx,
        if algorithm == AesAlgorithm::Kw {
            KeyUsageAlgorithm::AesKw
        } else {
            KeyUsageAlgorithm::Symmetric
        },
        usages,
        private_usages,
        public_usages,
        key_kind.as_ref(),
    )?;

    Ok(KeyAlgorithm::Aes { length, algorithm })
}

fn from_hmac<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    obj: Result<Object<'js>>,
    algorithm_name: &str,
    usages: &Array<'js>,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) -> Result<KeyAlgorithm> {
    let obj = obj?;
    let hash = extract_sha_hash(ctx, &obj)?;
    if !matches!(
        hash,
        HashAlgorithm::Sha1 | HashAlgorithm::Sha256 | HashAlgorithm::Sha384 | HashAlgorithm::Sha512
    ) {
        return Err(DOMException::not_supported_error(
            ctx,
            "Unsupported HMAC hash algorithm",
        ));
    }
    let length = get_optional_dictionary_value(&obj, "length")?
        .map(|value| enforce_range_u32(ctx, value, "length"))
        .transpose()?;
    if matches!(length, Some(length) if !hmac_length_is_byte_aligned(length)) {
        return Err(DOMException::not_supported_error(
            ctx,
            "HMAC key length must be a multiple of 8",
        ));
    }
    let validating_import = mode == KeyAlgorithmMode::ValidateImport;
    let enforce_implementation_limit =
        matches!(&mode, KeyAlgorithmMode::Generate | KeyAlgorithmMode::Derive);
    let mut length = match mode {
        KeyAlgorithmMode::Import { .. } | KeyAlgorithmMode::ValidateImport => {
            if length == Some(0) {
                return Err(DOMException::data_error(
                    ctx,
                    "HMAC import length must be greater than zero",
                ));
            }
            if validating_import {
                Some(length.unwrap_or(8))
            } else {
                length
            }
        },
        KeyAlgorithmMode::Generate => match length {
            Some(0) => {
                return Err(DOMException::operation_error(
                    ctx,
                    "HMAC generation length must be greater than zero",
                ));
            },
            Some(length) => Some(length),
            None => Some((hash.block_len() * 8) as u32),
        },
        KeyAlgorithmMode::Derive => match length {
            Some(0) => return Err(Exception::throw_type(ctx, "Invalid HMAC key length")),
            Some(length) => Some(length),
            None => Some((hash.block_len() * 8) as u32),
        },
    };

    #[cfg(feature = "_subtle-full")]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        algorithm_name: &str,
        hash: &HashAlgorithm,
        length: &mut Option<u32>,
    ) -> Result<Option<KeyKind>> {
        if let KeyAlgorithmMode::Import { data, format, kind } = mode {
            let data_length =
                import_symmetric_key(ctx, format, kind, data, algorithm_name, Some(hash))?;
            let data_length: u32 = data_length.try_into().map_err(|_| {
                DOMException::data_error(ctx, "HMAC key length exceeds unsigned long")
            })?;
            if data_length == 0 {
                return Err(DOMException::data_error(ctx, "HMAC key data is empty"));
            }
            if let Some(requested_length) = *length {
                if requested_length != data_length {
                    return Err(DOMException::data_error(
                        ctx,
                        "HMAC length does not match the key data",
                    ));
                }
            } else {
                *length = Some(data_length);
            }
            Ok(Some(*kind))
        } else {
            Ok(None)
        }
    }

    #[cfg(not(feature = "_subtle-full"))]
    #[inline]
    fn import<'js>(
        _ctx: &Ctx<'js>,
        _mode: KeyAlgorithmMode<'_, 'js>,
        _algorithm_name: &str,
        _hash: &HashAlgorithm,
        _length: &mut Option<u32>,
    ) -> Result<Option<KeyKind>> {
        Ok(None)
    }

    let key_kind = import(ctx, mode, algorithm_name, &hash, &mut length)?;
    let length = length.ok_or_else(|| {
        DOMException::operation_error(ctx, "HMAC key length could not be resolved")
    })?;
    if enforce_implementation_limit && length > MAX_HMAC_KEY_LENGTH_BITS {
        return Err(DOMException::operation_error(
            ctx,
            "HMAC key length exceeds the implementation limit",
        ));
    }

    KeyUsage::classify_and_check_usages(
        ctx,
        KeyUsageAlgorithm::Hmac,
        usages,
        private_usages,
        public_usages,
        key_kind.as_ref(),
    )?;

    Ok(KeyAlgorithm::Hmac { hash, length })
}

fn from_rsa<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    obj: Result<Object<'js>>,
    algorithm_name: &str,
    usages: &Array<'js>,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) -> Result<KeyAlgorithm> {
    let obj = obj?;
    let hash = extract_sha_hash(ctx, &obj)?;
    let is_generate = mode == KeyAlgorithmMode::Generate;

    #[cfg(feature = "_subtle-full")]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: &Object<'js>,
        algorithm_name: &str,
        hash: &HashAlgorithm,
    ) -> Result<(u32, Box<[u8]>, Option<KeyKind>)> {
        match mode {
            KeyAlgorithmMode::Import { format, kind, data } => {
                let (mod_length, exp) =
                    import_rsa_key(ctx, format, kind, data, algorithm_name, hash)?;
                Ok((mod_length, exp, Some(*kind)))
            },
            KeyAlgorithmMode::ValidateImport => Ok((0, Box::new([]), None)),
            _ => {
                let value = get_required_dictionary_value(obj, "modulusLength", "algorithm")?;
                let modulus_length = enforce_range_u32(ctx, value, "modulusLength")?;
                let public_exponent: TypedArray<u8> =
                    obj.get_required("publicExponent", "algorithm")?;
                let public_exponent = public_exponent
                    .as_bytes()
                    .ok_or_else(|| {
                        DOMException::not_supported_error(ctx, "Array buffer has been detached")
                    })?
                    .to_owned()
                    .into_boxed_slice();
                Ok((modulus_length, public_exponent, None))
            },
        }
    }

    #[cfg(not(feature = "_subtle-full"))]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: &Object<'js>,
        _algorithm_name: &str,
        _hash: &HashAlgorithm,
    ) -> Result<(u32, Box<[u8]>, Option<KeyKind>)> {
        if matches!(mode, KeyAlgorithmMode::ValidateImport) {
            return Ok((0, Box::new([]), None));
        }
        let value = get_required_dictionary_value(obj, "modulusLength", "algorithm")?;
        let modulus_length = enforce_range_u32(ctx, value, "modulusLength")?;
        let public_exponent: TypedArray<u8> = obj.get_required("publicExponent", "algorithm")?;
        let public_exponent = public_exponent
            .as_bytes()
            .ok_or_else(|| {
                DOMException::not_supported_error(ctx, "Array buffer has been detached")
            })?
            .to_owned()
            .into_boxed_slice();
        Ok((modulus_length, public_exponent, None))
    }

    let (modulus_length, public_exponent, key_kind) =
        import(ctx, mode, &obj, algorithm_name, &hash)?;

    KeyUsage::classify_and_check_usages(
        ctx,
        if algorithm_name == "RSA-OAEP" {
            KeyUsageAlgorithm::RsaOaep
        } else {
            KeyUsageAlgorithm::Sign
        },
        usages,
        private_usages,
        public_usages,
        key_kind.as_ref(),
    )?;

    if is_generate {
        parse_rsa_public_exponent(&public_exponent).or_throw_dom(ctx)?;
    }

    Ok(KeyAlgorithm::Rsa {
        modulus_length,
        public_exponent: Rc::new(public_exponent),
        hash,
    })
}

fn from_hkdf<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    obj: Result<Object<'js>>,
    algorithm_name: &str,
    usages: &Array<'js>,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) -> Result<KeyAlgorithm> {
    #[cfg(feature = "_subtle-full")]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: Result<Object<'js>>,
        algorithm_name: &str,
    ) -> Result<(KeyAlgorithm, Option<KeyKind>)> {
        match mode {
            KeyAlgorithmMode::Import { format, kind, data } => {
                import_derive_key(ctx, format, kind, data, algorithm_name)?;
                Ok((KeyAlgorithm::HkdfImport, Some(*kind)))
            },
            KeyAlgorithmMode::Derive => {
                let obj = obj?;
                Ok((
                    KeyAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?),
                    None,
                ))
            },
            KeyAlgorithmMode::ValidateImport => Ok((KeyAlgorithm::HkdfImport, None)),
            _ => algorithm_not_supported_error(ctx),
        }
    }

    #[cfg(not(feature = "_subtle-full"))]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: Result<Object<'js>>,
        _algorithm_name: &str,
    ) -> Result<(KeyAlgorithm, Option<KeyKind>)> {
        match mode {
            KeyAlgorithmMode::Derive => {
                let obj = obj?;
                Ok((
                    KeyAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?),
                    None,
                ))
            },
            KeyAlgorithmMode::ValidateImport => Ok((KeyAlgorithm::HkdfImport, None)),
            _ => algorithm_not_supported_error(ctx),
        }
    }

    let (algorithm, key_kind) = import(ctx, mode, obj, algorithm_name)?;

    KeyUsage::classify_and_check_usages(
        ctx,
        KeyUsageAlgorithm::DeriveSymmetric,
        usages,
        private_usages,
        public_usages,
        key_kind.as_ref(),
    )?;

    Ok(algorithm)
}

fn from_pbkdf2<'js>(
    ctx: &Ctx<'js>,
    mode: KeyAlgorithmMode<'_, 'js>,
    obj: Result<Object<'js>>,
    algorithm_name: &str,
    usages: &Array<'js>,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) -> Result<KeyAlgorithm> {
    #[cfg(feature = "_subtle-full")]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: Result<Object<'js>>,
        algorithm_name: &str,
    ) -> Result<(KeyAlgorithm, Option<KeyKind>)> {
        match mode {
            KeyAlgorithmMode::Import { format, kind, data } => {
                import_derive_key(ctx, format, kind, data, algorithm_name)?;
                Ok((KeyAlgorithm::Pbkdf2Import, Some(*kind)))
            },
            KeyAlgorithmMode::Derive => {
                let obj = obj?;
                Ok((
                    KeyAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?),
                    None,
                ))
            },
            KeyAlgorithmMode::ValidateImport => Ok((KeyAlgorithm::Pbkdf2Import, None)),
            _ => algorithm_not_supported_error(ctx),
        }
    }

    #[cfg(not(feature = "_subtle-full"))]
    #[inline]
    fn import<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: Result<Object<'js>>,
        _algorithm_name: &str,
    ) -> Result<(KeyAlgorithm, Option<KeyKind>)> {
        match mode {
            KeyAlgorithmMode::Derive => {
                let obj = obj?;
                Ok((
                    KeyAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?),
                    None,
                ))
            },
            KeyAlgorithmMode::ValidateImport => Ok((KeyAlgorithm::Pbkdf2Import, None)),
            _ => algorithm_not_supported_error(ctx),
        }
    }

    let (algorithm, key_kind) = import(ctx, mode, obj, algorithm_name)?;

    KeyUsage::classify_and_check_usages(
        ctx,
        KeyUsageAlgorithm::DeriveSymmetric,
        usages,
        private_usages,
        public_usages,
        key_kind.as_ref(),
    )?;

    Ok(algorithm)
}

impl KeyAlgorithm {
    pub fn from_js<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        value: Value<'js>,
        usages: Array<'js>,
    ) -> Result<KeyAlgorithmWithUsages> {
        // When _subtle-full is not enabled, Import mode is not supported
        #[cfg(not(feature = "_subtle-full"))]
        if matches!(mode, KeyAlgorithmMode::Import { .. }) {
            return Err(DOMException::not_supported_error(
                ctx,
                "Key import is not supported with this crypto provider",
            ));
        }
        let (name, obj) = to_name_and_maybe_object(ctx, value)?;
        let name = normalize_algorithm_name(&name);
        let usages = if mode == KeyAlgorithmMode::ValidateImport && usages.len() == 0 {
            let synthetic_usages = Array::new(ctx.clone())?;
            let usage = match name.as_str() {
                "AES-KW" => "wrapKey",
                "AES-CBC" | "AES-CTR" | "AES-GCM" | "RSA-OAEP" => "encrypt",
                "ECDH" | "X25519" | "HKDF" | "PBKDF2" => "deriveKey",
                _ => "sign",
            };
            synthetic_usages.set(0, usage)?;
            synthetic_usages
        } else {
            usages
        };
        let mut public_usages = vec![];
        let mut private_usages = vec![];
        let algorithm_name = name.as_ref();
        let algorithm = match algorithm_name {
            "Ed25519" => from_ed25519(
                ctx,
                mode,
                algorithm_name,
                &usages,
                &mut private_usages,
                &mut public_usages,
            )?,
            "X25519" => from_x25519(
                ctx,
                mode,
                algorithm_name,
                &usages,
                &mut private_usages,
                &mut public_usages,
            )?,
            "AES-CBC" | "AES-CTR" | "AES-GCM" | "AES-KW" => from_aes(
                ctx,
                mode,
                obj,
                algorithm_name,
                &usages,
                &mut private_usages,
                &mut public_usages,
            )?,
            "ECDH" => Self::from_ec(
                ctx,
                mode,
                obj,
                algorithm_name,
                EcAlgorithm::Ecdh,
                &usages,
                &mut private_usages,
                &mut public_usages,
                KeyUsageAlgorithm::DeriveAsymmetric,
            )?,
            "ECDSA" => Self::from_ec(
                ctx,
                mode,
                obj,
                algorithm_name,
                EcAlgorithm::Ecdsa,
                &usages,
                &mut private_usages,
                &mut public_usages,
                KeyUsageAlgorithm::Sign,
            )?,
            "HMAC" => from_hmac(
                ctx,
                mode,
                obj,
                algorithm_name,
                &usages,
                &mut private_usages,
                &mut public_usages,
            )?,
            "RSA-OAEP" | "RSA-PSS" | "RSASSA-PKCS1-v1_5" => from_rsa(
                ctx,
                mode,
                obj,
                algorithm_name,
                &usages,
                &mut private_usages,
                &mut public_usages,
            )?,
            "HKDF" => from_hkdf(
                ctx,
                mode,
                obj,
                algorithm_name,
                &usages,
                &mut private_usages,
                &mut public_usages,
            )?,
            "PBKDF2" => from_pbkdf2(
                ctx,
                mode,
                obj,
                algorithm_name,
                &usages,
                &mut private_usages,
                &mut public_usages,
            )?,
            _ => return algorithm_not_supported_error(ctx),
        };

        Ok(KeyAlgorithmWithUsages {
            name,
            algorithm,
            public_usages,
            private_usages,
        })
    }

    pub fn as_object<'js, T: AsRef<str>>(&self, ctx: &Ctx<'js>, name: T) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        obj.set(PredefinedAtom::Name, name.as_ref())?;
        match self {
            KeyAlgorithm::Aes { length, .. } => {
                obj.set(PredefinedAtom::Length, length)?;
            },
            KeyAlgorithm::Ec { curve, .. } => {
                obj.set("namedCurve", curve.as_str())?;
            },

            KeyAlgorithm::Hmac { hash, length } => {
                let hash_obj = create_hash_object(ctx, hash)?;
                obj.set("hash", hash_obj)?;

                obj.set(PredefinedAtom::Length, length)?;
            },
            KeyAlgorithm::Rsa {
                modulus_length,
                public_exponent,
                hash,
            } => {
                let public_exponent = public_exponent.as_ref().to_vec();
                let array = TypedArray::new(ctx.clone(), public_exponent)?;

                let hash_obj = create_hash_object(ctx, hash)?;
                obj.set("hash", hash_obj)?;

                obj.set("modulusLength", modulus_length)?;
                obj.set("publicExponent", array)?;
            },
            KeyAlgorithm::Derive(KeyDerivation::Hkdf { hash, salt, info }) => {
                let salt = TypedArray::<u8>::new(ctx.clone(), salt.to_vec())?;
                let info = TypedArray::<u8>::new(ctx.clone(), info.to_vec())?;

                obj.set("hash", hash.as_str())?;
                obj.set("salt", salt)?;
                obj.set("info", info)?;
            },
            KeyAlgorithm::Derive(KeyDerivation::Pbkdf2 {
                hash,
                salt,
                iterations,
            }) => {
                let salt = TypedArray::<u8>::new(ctx.clone(), salt.to_vec())?;
                obj.set("hash", hash.as_str())?;
                obj.set("salt", salt)?;
                obj.set("iterations", iterations)?;
            },
            _ => {},
        };
        Ok(obj)
    }

    #[allow(clippy::too_many_arguments)]
    fn from_ec<'js>(
        ctx: &Ctx<'js>,
        #[allow(unused_variables)] mode: KeyAlgorithmMode<'_, 'js>,
        obj: Result<Object<'js>>,
        #[allow(unused_variables)] algorithm_name: &str,
        algorithm: EcAlgorithm,
        key_usages: &Array<'js>,
        private_usages: &mut Vec<String>,
        public_usages: &mut Vec<String>,
        key_usage_algorithm: KeyUsageAlgorithm,
    ) -> Result<KeyAlgorithm> {
        let obj = obj?;
        let curve_name: String = obj.get_required("namedCurve", "algorithm")?;
        let curve = EllipticCurve::try_from(curve_name.as_str())
            .map_err(NotSupportedError)
            .or_throw_dom(ctx)?;

        #[cfg(feature = "_subtle-full")]
        let key_kind = if let KeyAlgorithmMode::Import { format, kind, data } = mode {
            import_ec_key(ctx, format, kind, data, algorithm_name, &curve, &curve_name)?;
            Some(kind)
        } else {
            None
        };
        #[cfg(not(feature = "_subtle-full"))]
        let key_kind: Option<&KeyKind> = None;

        KeyUsage::classify_and_check_usages(
            ctx,
            key_usage_algorithm,
            key_usages,
            private_usages,
            public_usages,
            key_kind.as_deref(),
        )?;

        Ok(KeyAlgorithm::Ec { curve, algorithm })
    }
}

#[cfg(feature = "_subtle-full")]
fn import_derive_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormatData<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
) -> Result<()> {
    if let KeyFormatData::Raw(object_bytes) = format {
        *data = object_bytes.into_bytes(ctx)?;
        *kind = KeyKind::Secret;
    } else {
        return Err(DOMException::not_supported_error(
            ctx,
            [algorithm_name, " only supports 'raw' import format"].concat(),
        ));
    }

    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn import_rsa_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormatData<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
    hash: &HashAlgorithm,
) -> Result<(u32, Box<[u8]>)> {
    use crate::{
        provider::{CryptoProvider, RsaJwkImport},
        CRYPTO_PROVIDER,
    };

    let validate_oid = |other_oid: const_oid::ObjectIdentifier| -> Result<()> {
        if other_oid != const_oid::db::rfc5912::RSA_ENCRYPTION {
            return algorithm_mismatch_error(ctx, algorithm_name);
        }
        Ok(())
    };

    let (modulus_length, public_exponent) = match format {
        KeyFormatData::Jwk(object) => {
            validate_jwk_kty(ctx, &object, "RSA")?;

            if let Some(alg) = object.get_optional::<_, String>("alg")? {
                let numeric_hash_str = match algorithm_name {
                    "RSASSA-PKCS1-v1_5" => alg.strip_prefix("RS"),
                    "RSA-PSS" => alg.strip_prefix("PS"),
                    "RSA-OAEP" => alg.strip_prefix("RSA-OAEP-"),
                    _ => None,
                };
                let Some(numeric_hash_str) = numeric_hash_str else {
                    return algorithm_mismatch_error(ctx, algorithm_name);
                };
                if numeric_hash_str != hash.as_numeric_str() {
                    return hash_mismatch_error(ctx, hash);
                }
            }

            let n_bytes = get_jwk_required_bytes(ctx, &object, "n")?;
            let e_bytes = get_jwk_required_bytes(ctx, &object, "e")?;

            let d_bytes = get_jwk_optional_bytes(ctx, &object, "d")?;

            let result = if let Some(ref d_bytes) = d_bytes {
                let p_bytes = get_jwk_required_bytes(ctx, &object, "p")?;
                let q_bytes = get_jwk_required_bytes(ctx, &object, "q")?;
                let dp_bytes = get_jwk_required_bytes(ctx, &object, "dp")?;
                let dq_bytes = get_jwk_required_bytes(ctx, &object, "dq")?;
                let qi_bytes = get_jwk_required_bytes(ctx, &object, "qi")?;

                let jwk = RsaJwkImport {
                    n: &n_bytes,
                    e: &e_bytes,
                    d: Some(d_bytes),
                    p: Some(&p_bytes),
                    q: Some(&q_bytes),
                    dp: Some(&dp_bytes),
                    dq: Some(&dq_bytes),
                    qi: Some(&qi_bytes),
                };
                CRYPTO_PROVIDER.import_rsa_jwk(jwk).or_throw_dom(ctx)?
            } else {
                let jwk = RsaJwkImport {
                    n: &n_bytes,
                    e: &e_bytes,
                    d: None,
                    p: None,
                    q: None,
                    dp: None,
                    dq: None,
                    qi: None,
                };
                CRYPTO_PROVIDER.import_rsa_jwk(jwk).or_throw_dom(ctx)?
            };

            *data = result.key_data;
            *kind = if result.is_private {
                KeyKind::Private
            } else {
                KeyKind::Public
            };
            (result.modulus_length as usize, result.public_exponent)
        },
        KeyFormatData::Raw(object_bytes) => {
            let result = CRYPTO_PROVIDER
                .import_rsa_public_key_pkcs1(object_bytes.as_bytes(ctx)?)
                .or_throw_dom(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Public;
            (result.modulus_length as usize, result.public_exponent)
        },
        KeyFormatData::Pkcs8(object_bytes) => {
            let pk_info = PrivateKeyInfoRef::from_der(object_bytes.as_bytes(ctx)?).or_throw(ctx)?;
            validate_oid(pk_info.algorithm.oid)?;
            let result = CRYPTO_PROVIDER
                .import_rsa_private_key_pkcs8(object_bytes.as_bytes(ctx)?)
                .or_throw_dom(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Private;
            (result.modulus_length as usize, result.public_exponent)
        },
        KeyFormatData::Spki(object_bytes) => {
            let pk_info = spki::SubjectPublicKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?)
                .or_throw(ctx)?;
            validate_oid(pk_info.algorithm.oid)?;
            let result = CRYPTO_PROVIDER
                .import_rsa_public_key_spki(object_bytes.as_bytes(ctx)?)
                .or_throw_dom(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Public;
            (result.modulus_length as usize, result.public_exponent)
        },
    };

    let public_exponent = public_exponent.into_boxed_slice();
    Ok((modulus_length as u32, public_exponent))
}

#[cfg(feature = "_subtle-full")]
fn import_symmetric_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormatData<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
    hash: Option<&HashAlgorithm>,
) -> Result<usize> {
    *kind = KeyKind::Secret;

    match format {
        KeyFormatData::Jwk(object) => {
            validate_jwk_kty(ctx, &object, "oct")?;

            let k: String = get_jwk_required_string(ctx, &object, "k")?;
            let alg: String = get_jwk_required_string(ctx, &object, "alg")?;

            let prefix = &alg[..1];

            match (prefix, hash) {
                //HMAC - HS256, HS512 etc
                ("H", Some(hash)) => {
                    if &alg[2..] != hash.as_numeric_str() {
                        return hash_mismatch_error(ctx, hash);
                    }
                },
                //AES - A256KW, A256GCM, A256CRT, A512CBC etc
                ("A", None) => {
                    //extract AES-{suffix}
                    let aes_variant = &alg[4..];

                    if !algorithm_name.ends_with(aes_variant) {
                        return algorithm_mismatch_error(ctx, algorithm_name);
                    }
                },
                _ => return algorithm_mismatch_error(ctx, algorithm_name),
            }

            *data = bytes_from_b64_url_safe(k.as_bytes()).or_throw(ctx)?;
            Ok(data.len() * 8)
        },
        KeyFormatData::Raw(object_bytes) => {
            let bytes = object_bytes.into_bytes(ctx)?;

            *data = bytes;
            Ok(data.len() * 8)
        },
        _ => algorithm_mismatch_error(ctx, algorithm_name),
    }
}

// EC algorithm OID for validation
#[cfg(feature = "_subtle-full")]
const EC_ALGORITHM_OID: const_oid::ObjectIdentifier =
    const_oid::ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");

#[cfg(feature = "_subtle-full")]
fn import_ec_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormatData<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
    curve: &EllipticCurve,
    curve_name: &str,
) -> Result<()> {
    use crate::{
        provider::{CryptoProvider, EcJwkImport},
        CRYPTO_PROVIDER,
    };

    let validate_oid = |other_oid: const_oid::ObjectIdentifier| -> Result<()> {
        if other_oid != EC_ALGORITHM_OID {
            return algorithm_mismatch_error(ctx, algorithm_name);
        }
        Ok(())
    };

    // Get expected coordinate length for the curve
    let coord_len = match curve {
        EllipticCurve::P256 => 32,
        EllipticCurve::P384 => 48,
        EllipticCurve::P521 => 66,
    };

    match format {
        KeyFormatData::Jwk(object) => {
            validate_jwk_kty(ctx, &object, "EC")?;

            validate_jwk_use(ctx, &object, true)?;

            validate_jwk_crv(ctx, &object, curve_name)?;

            let x_bytes = get_jwk_required_bytes(ctx, &object, "x")?;
            validate_jwk_bytes_len(ctx, algorithm_name, "x coordinate", &x_bytes, coord_len)?;

            let y_bytes = get_jwk_required_bytes(ctx, &object, "y")?;
            validate_jwk_bytes_len(ctx, algorithm_name, "y coordinate", &y_bytes, coord_len)?;

            let d_bytes = get_jwk_optional_bytes(ctx, &object, "d")?;

            if let Some(ref d_bytes) = d_bytes {
                validate_jwk_bytes_len(ctx, algorithm_name, "private key", d_bytes, coord_len)?;
            }

            let jwk = EcJwkImport {
                x: &x_bytes,
                y: &y_bytes,
                d: d_bytes.as_deref(),
            };

            let result = CRYPTO_PROVIDER
                .import_ec_jwk(jwk, *curve)
                .or_throw_dom(ctx)?;
            *data = result.key_data;
            *kind = if result.is_private {
                KeyKind::Private
            } else {
                KeyKind::Public
            };
        },
        KeyFormatData::Raw(object_bytes) => {
            let bytes = object_bytes.as_bytes(ctx)?;
            let result = CRYPTO_PROVIDER
                .import_ec_public_key_sec1(bytes, *curve)
                .or_throw_dom(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Public;
        },
        KeyFormatData::Spki(object_bytes) => {
            let spki = spki::SubjectPublicKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?)
                .or_throw_data_error(ctx)?;
            validate_oid(spki.algorithm.oid)?;
            let result = CRYPTO_PROVIDER
                .import_ec_public_key_spki(object_bytes.as_bytes(ctx)?, *curve)
                .or_throw_dom(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Public;
        },
        KeyFormatData::Pkcs8(object_bytes) => {
            let pkcs8 = PrivateKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?)
                .or_throw_data_error(ctx)?;
            validate_oid(pkcs8.algorithm.oid)?;
            let result = CRYPTO_PROVIDER
                .import_ec_private_key_pkcs8(object_bytes.as_bytes(ctx)?)
                .or_throw_dom(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Private;
        },
    };
    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn import_okp_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormatData<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    oid: ObjectIdentifier,
    algorithm_name: &str,
    is_ed25519: bool,
) -> Result<()> {
    let validate_oid = |other_oid: const_oid::ObjectIdentifier| -> Result<()> {
        if other_oid != oid {
            return algorithm_mismatch_error(ctx, algorithm_name);
        }
        Ok(())
    };

    match format {
        KeyFormatData::Jwk(object) => {
            validate_jwk_kty(ctx, &object, "OKP")?;

            validate_jwk_crv(ctx, &object, algorithm_name)?;

            if is_ed25519 {
                validate_jwk_alg(ctx, &object)?;
            }

            validate_jwk_use(ctx, &object, is_ed25519)?;

            let public_key = get_jwk_required_bytes(ctx, &object, "x")?;
            validate_jwk_bytes_len(ctx, algorithm_name, "public key", &public_key, 32)?;

            let private_key = get_jwk_optional_bytes(ctx, &object, "d")?;

            if let Some(private_key) = private_key {
                validate_jwk_bytes_len(ctx, algorithm_name, "private key", &private_key, 32)?;

                validate_okp_jwk_key_pair(ctx, &private_key, &public_key, is_ed25519)?;

                if is_ed25519 {
                    // Ed25519 internal representation is the complete PKCS#8 DER.
                    let inner = OctetStringRef::new(private_key.as_slice()).or_throw(ctx)?;
                    let inner_der = inner.to_der().or_throw(ctx)?;
                    let pk_info = PrivateKeyInfoRef {
                        algorithm: AlgorithmIdentifier {
                            oid,
                            parameters: None,
                        },
                        private_key: OctetStringRef::new(&inner_der).or_throw(ctx)?,
                        public_key: Some(BitStringRef::from_bytes(&public_key).or_throw(ctx)?),
                    };
                    *data = pk_info.to_der().or_throw(ctx)?;
                } else {
                    // X25519 internal representation is raw 32-byte scalar.
                    *data = private_key;
                }
                *kind = KeyKind::Private;
            } else {
                *data = public_key;
                *kind = KeyKind::Public;
            }
        },
        KeyFormatData::Raw(object_bytes) => {
            let bytes = object_bytes.into_bytes(ctx)?;
            if bytes.len() != 32 {
                return Err(DOMException::data_error(
                    ctx,
                    [algorithm_name, " keys must be 32 bytes long"].concat(),
                ));
            }
            *data = bytes;
            *kind = KeyKind::Public;
        },
        KeyFormatData::Spki(object_bytes) => {
            let spki = spki::SubjectPublicKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?)
                .or_throw_data_error(ctx)?;
            validate_oid(spki.algorithm.oid)?;

            let public_key = spki.subject_public_key.raw_bytes();
            if public_key.len() != 32 {
                return Err(DOMException::data_error(
                    ctx,
                    [algorithm_name, " public key must be 32 bytes"].concat(),
                ));
            }

            *data = public_key.to_vec();
            *kind = KeyKind::Public;
        },
        KeyFormatData::Pkcs8(object_bytes) => {
            let bytes = object_bytes.into_bytes(ctx)?;
            let pkcs8 = PrivateKeyInfoRef::try_from(bytes.as_slice()).or_throw_data_error(ctx)?;
            validate_oid(pkcs8.algorithm.oid)?;
            if is_ed25519 {
                // Ed25519 internal representation is the complete PKCS#8 DER.
                *data = bytes;
            } else {
                // X25519 internal representation is the inner OCTET STRING.
                *data = OctetString::from_der(pkcs8.private_key.as_bytes())
                    .or_throw(ctx)?
                    .as_bytes()
                    .to_vec();
                if data.len() != 32 {
                    return Err(DOMException::data_error(
                        ctx,
                        [algorithm_name, " private key must be 32 bytes"].concat(),
                    ));
                }
            }
            *kind = KeyKind::Private;
        },
    }

    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn get_jwk_required_string<'js>(
    ctx: &Ctx<'js>,
    object: &Object<'js>,
    name: &str,
) -> Result<String> {
    object
        .get_required(name, "keyData")
        .or_throw_data_error(ctx)
}

#[cfg(feature = "_subtle-full")]
fn get_jwk_required_bytes<'js>(
    ctx: &Ctx<'js>,
    object: &Object<'js>,
    name: &str,
) -> Result<Vec<u8>> {
    let value = get_jwk_required_string(ctx, object, name)?;
    bytes_from_b64_url_safe(value.as_bytes()).or_throw_data_error(ctx)
}

#[cfg(feature = "_subtle-full")]
fn get_jwk_optional_bytes<'js>(
    ctx: &Ctx<'js>,
    object: &Object<'js>,
    name: &str,
) -> Result<Option<Vec<u8>>> {
    let value = object.get_optional::<_, String>(name)?;
    value
        .map(|value| bytes_from_b64_url_safe(value.as_bytes()).or_throw_data_error(ctx))
        .transpose()
}

#[cfg(feature = "_subtle-full")]
fn validate_jwk_kty<'js>(ctx: &Ctx<'js>, object: &Object<'js>, expected: &str) -> Result<()> {
    let kty = get_jwk_required_string(ctx, object, "kty")?;
    if kty != expected {
        return Err(DOMException::data_error(
            ctx,
            ["JWK 'kty' parameter must be '", expected, "'"].concat(),
        ));
    }
    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn validate_jwk_crv<'js>(ctx: &Ctx<'js>, object: &Object<'js>, expected: &str) -> Result<()> {
    let crv = get_jwk_required_string(ctx, object, "crv")?;
    if crv != expected {
        return Err(DOMException::data_error(
            ctx,
            ["JWK 'crv' parameter must be '", expected, "'"].concat(),
        ));
    }
    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn validate_jwk_use(ctx: &Ctx<'_>, object: &Object<'_>, is_ed25519: bool) -> Result<()> {
    if let Some(use_) = object.get_optional::<_, String>("use")? {
        let expected = if is_ed25519 { "sig" } else { "enc" };
        if use_ != expected {
            return Err(DOMException::data_error(
                ctx,
                "JWK 'use' parameter is invalid",
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn validate_jwk_alg(ctx: &Ctx<'_>, object: &Object<'_>) -> Result<()> {
    if let Some(alg) = object.get_optional::<_, String>("alg")? {
        if alg != "Ed25519" && alg != "EdDSA" {
            return Err(DOMException::data_error(
                ctx,
                "JWK 'alg' parameter is invalid",
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn validate_jwk_bytes_len(
    ctx: &Ctx<'_>,
    algorithm_name: &str,
    field: &str,
    bytes: &[u8],
    expected: usize,
) -> Result<()> {
    if bytes.len() != expected {
        return Err(DOMException::data_error(
            ctx,
            [algorithm_name, " JWK ", field, " has invalid length"].concat(),
        ));
    }
    Ok(())
}

#[cfg(feature = "_subtle-full")]
fn validate_okp_jwk_key_pair<'js>(
    ctx: &Ctx<'js>,
    private_key: &[u8],
    public_key: &[u8],
    is_ed25519: bool,
) -> Result<()> {
    let derived_public_key = if is_ed25519 {
        let secret_key: [u8; 32] = private_key.try_into().or_throw_data_error(ctx)?;
        SigningKey::from_bytes(&secret_key)
            .verifying_key()
            .to_bytes()
            .to_vec()
    } else {
        let secret_key: [u8; 32] = private_key.try_into().or_throw_data_error(ctx)?;
        let secret = StaticSecret::from(secret_key);
        PublicKey::from(&secret).as_bytes().to_vec()
    };
    if derived_public_key.as_slice() != public_key {
        return Err(DOMException::data_error(ctx, "JWK key pair is invalid"));
    }
    Ok(())
}

pub fn extract_sha_hash<'js>(ctx: &Ctx<'js>, obj: &Object<'js>) -> Result<HashAlgorithm> {
    let hash: Value = obj.get_required("hash", "algorithm")?;
    let hash = if let Some(string) = hash.as_string() {
        string.to_string()
    } else if let Some(obj) = hash.into_object() {
        obj.get_required("name", "hash")
    } else {
        return Err(DOMException::not_supported_error(
            ctx,
            "hash must be a string or an object",
        ));
    }?;
    let hash = normalize_algorithm_name(&hash);
    HashAlgorithm::from_strict_str(hash.as_str()).or_throw_dom(ctx)
}

fn create_hash_object<'js>(ctx: &Ctx<'js>, hash: &HashAlgorithm) -> Result<Object<'js>> {
    let hash_obj = Object::new(ctx.clone())?;
    hash_obj.set(PredefinedAtom::Name, hash.as_str())?;
    Ok(hash_obj)
}

#[cfg(feature = "_subtle-full")]
pub fn hash_mismatch_error<T>(ctx: &Ctx<'_>, hash: &HashAlgorithm) -> Result<T> {
    Err(DOMException::type_mismatch_error(
        ctx,
        ["Algorithm hash expected to be ", hash.as_str()].concat(),
    ))
}

#[cfg(feature = "_subtle-full")]
trait DataErrorResultExt<T> {
    fn or_throw_data_error(self, ctx: &Ctx<'_>) -> Result<T>;
}

#[cfg(feature = "_subtle-full")]
impl<T, E> DataErrorResultExt<T> for std::result::Result<T, E>
where
    E: std::fmt::Display,
{
    fn or_throw_data_error(self, ctx: &Ctx<'_>) -> Result<T> {
        self.map_err(|e| DataError(e.to_string())).or_throw_dom(ctx)
    }
}
