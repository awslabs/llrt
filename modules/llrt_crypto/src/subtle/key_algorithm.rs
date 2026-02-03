// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::rc::Rc;

#[cfg(feature = "_subtle-full")]
use der::{asn1::OctetStringRef, Decode, Encode};
#[cfg(feature = "_subtle-full")]
use llrt_encoding::bytes_from_b64_url_safe;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt, str_enum};
#[cfg(feature = "_subtle-full")]
use pkcs8::PrivateKeyInfoRef;
use rquickjs::{
    atom::PredefinedAtom, Array, Ctx, Exception, FromJs, Object, Result, TypedArray, Value,
};
#[cfg(feature = "_subtle-full")]
use spki::{AlgorithmIdentifier, ObjectIdentifier};

use crate::sha_hash::ShaAlgorithm;

#[cfg(feature = "_subtle-full")]
use super::algorithm_mismatch_error;
use super::{
    algorithm_not_supported_error, crypto_key::KeyKind, to_name_and_maybe_object, EllipticCurve,
};

#[derive(Clone, Copy, PartialEq)]
pub enum KeyUsage {
    //7 values, can be max 255 (u8) 0b11111111
    Encrypt,
    Decrypt,
    WrapKey,
    UnwrapKey,
    Sign,
    Verify,
    DeriveKey,
    DeriveBits,
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

        for usage in key_usages.iter::<String>() {
            has_any_usages = true;
            let value = usage?;
            let usage = KeyUsage::try_from(value.as_str()).or_throw(ctx)?;
            let usage = usage.mask();
            if allowed_usages & usage != usage {
                return Err(Exception::throw_message(
                    ctx,
                    &["Invalid key usage '", &value, "'"].concat(),
                ));
            }

            if private_usages_mask == public_usages_mask {
                generated_private_usages.push(value.clone());
                generated_public_usages.push(value);
            } else if private_usages_mask & usage == usage {
                generated_private_usages.push(value);
            } else if public_usages_mask & usage == usage {
                generated_public_usages.push(value);
            }
        }

        *private_usages = generated_private_usages;
        *public_usages = generated_public_usages;

        if !has_any_usages {
            return Err(Exception::throw_message(ctx, "Key usages empty"));
        }

        if private_usages_mask > 0 && private_usages.is_empty() {
            return Err(Exception::throw_message(
                ctx,
                "No required private key usages provided",
            ));
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
                return Err(Exception::throw_message(ctx, "Invalid key usage"));
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

    //two mask algorithms (asymmetric) - use high bits as for private, low bits for public
    //HKDF, PBKDF2, X25519
    Derive = ((KeyUsage::DeriveKey.mask() | KeyUsage::DeriveBits.mask()) << 8)
        | KeyUsage::DeriveKey.mask()
        | KeyUsage::DeriveBits.mask(),

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
}

#[derive(Debug, Clone)]
pub enum KeyDerivation {
    Hkdf {
        hash: ShaAlgorithm,
        salt: Box<[u8]>,
        info: Box<[u8]>,
    },
    Pbkdf2 {
        hash: ShaAlgorithm,
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

        let iterations = obj.get_required("iterations", "algorithm")?;
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

#[derive(Debug, Clone)]
pub enum KeyAlgorithm {
    Aes {
        length: u16,
    },
    Ec {
        curve: EllipticCurve,
        algorithm: EcAlgorithm,
    },
    X25519,
    Ed25519,
    Hmac {
        hash: ShaAlgorithm,
        length: u16,
    },
    Rsa {
        modulus_length: u32,
        public_exponent: Rc<Box<[u8]>>,
        hash: ShaAlgorithm,
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
        if let Some(string) = value.as_string() {
            let string = string.to_string()?;
            match string.as_str() {
                "jwk" => return Ok(KeyFormat::Jwk),
                "raw" => return Ok(KeyFormat::Raw),
                "spki" => return Ok(KeyFormat::Spki),
                "pkcs8" => return Ok(KeyFormat::Pkcs8),
                _ => {},
            };
        }
        Err(Exception::throw_message(
            ctx,
            "Key import/export format must be 'jwk','raw','spki' or 'pkcs8'",
        ))
    }
}

pub enum KeyFormatData<'js> {
    Jwk(Object<'js>),
    Raw(ObjectBytes<'js>),
    Spki(ObjectBytes<'js>),
    Pkcs8(ObjectBytes<'js>),
}

pub enum KeyAlgorithmMode<'a, 'js> {
    Import {
        format: KeyFormatData<'js>,
        kind: &'a mut KeyKind,
        data: &'a mut Vec<u8>,
    },
    Generate,
    Derive,
}

pub struct KeyAlgorithmWithUsages {
    pub name: String,
    pub algorithm: KeyAlgorithm,
    pub public_usages: Vec<String>,
    pub private_usages: Vec<String>,
}

impl KeyAlgorithm {
    pub fn from_js<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        value: Value<'js>,
        usages: Array<'js>,
    ) -> Result<KeyAlgorithmWithUsages> {
        // When _rustcrypto is not enabled, Import mode is not supported
        #[cfg(not(feature = "_subtle-full"))]
        if matches!(mode, KeyAlgorithmMode::Import { .. }) {
            return Err(Exception::throw_message(
                ctx,
                "Key import is not supported with this crypto provider",
            ));
        }

        let (name, obj) = to_name_and_maybe_object(ctx, value)?;
        let mut public_usages = vec![];
        let mut private_usages = vec![];
        let algorithm_name = name.as_str();
        let algorithm = match algorithm_name {
            "Ed25519" => {
                #[cfg(feature = "_subtle-full")]
                let key_kind = if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                    import_okp_key(
                        ctx,
                        format,
                        kind,
                        data,
                        const_oid::db::rfc8410::ID_ED_25519,
                        algorithm_name,
                    )?;
                    Some(kind)
                } else {
                    None
                };
                #[cfg(not(feature = "_subtle-full"))]
                let key_kind: Option<&KeyKind> = None;

                KeyUsage::classify_and_check_usages(
                    ctx,
                    KeyUsageAlgorithm::Sign,
                    &usages,
                    &mut private_usages,
                    &mut public_usages,
                    key_kind.as_deref(),
                )?;
                KeyAlgorithm::Ed25519
            },
            "X25519" => {
                #[cfg(feature = "_subtle-full")]
                let key_kind = if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                    import_okp_key(
                        ctx,
                        format,
                        kind,
                        data,
                        const_oid::db::rfc8410::ID_X_25519,
                        algorithm_name,
                    )?;
                    Some(kind)
                } else {
                    None
                };
                #[cfg(not(feature = "_subtle-full"))]
                let key_kind: Option<&KeyKind> = None;

                KeyUsage::classify_and_check_usages(
                    ctx,
                    KeyUsageAlgorithm::Derive,
                    &usages,
                    &mut private_usages,
                    &mut public_usages,
                    key_kind.as_deref(),
                )?;
                KeyAlgorithm::X25519
            },
            "AES-CBC" | "AES-CTR" | "AES-GCM" | "AES-KW" => {
                #[cfg(feature = "_subtle-full")]
                let (length, key_kind) = {
                    let mut key_kind = None;
                    let length = if let KeyAlgorithmMode::Import { data, format, kind } = mode {
                        let l =
                            import_symmetric_key(ctx, format, kind, data, algorithm_name, None)?;
                        key_kind = Some(kind);
                        l
                    } else {
                        obj.or_throw(ctx)?.get_required("length", "algorithm")?
                    } as u16;
                    (length, key_kind)
                };
                #[cfg(not(feature = "_subtle-full"))]
                let (length, key_kind): (u16, Option<&KeyKind>) = {
                    let length: u16 = obj.or_throw(ctx)?.get_required("length", "algorithm")?;
                    (length, None)
                };

                if !matches!(length, 128 | 192 | 256) {
                    return Err(Exception::throw_message(
                        ctx,
                        &format!(
                            "Algorithm 'length' must be one of: 128, 192, or 256 = {}",
                            length
                        ),
                    ));
                }

                KeyUsage::classify_and_check_usages(
                    ctx,
                    if name == "AES-KW" {
                        KeyUsageAlgorithm::AesKw
                    } else {
                        KeyUsageAlgorithm::Symmetric
                    },
                    &usages,
                    &mut private_usages,
                    &mut public_usages,
                    key_kind.as_deref(),
                )?;

                KeyAlgorithm::Aes { length }
            },
            "ECDH" => Self::from_ec(
                ctx,
                mode,
                obj,
                algorithm_name,
                EcAlgorithm::Ecdh,
                &usages,
                &mut private_usages,
                &mut public_usages,
                KeyUsageAlgorithm::Derive,
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
            "HMAC" => {
                let obj = obj.or_throw(ctx)?;
                let hash = extract_sha_hash(ctx, &obj)?;

                #[cfg(feature = "_subtle-full")]
                let mut length: u16 = obj.get_optional("length")?.unwrap_or_default();
                #[cfg(not(feature = "_subtle-full"))]
                let length: u16 = obj.get_optional("length")?.unwrap_or_default();

                #[cfg(feature = "_subtle-full")]
                let key_kind = if let KeyAlgorithmMode::Import { data, format, kind } = mode {
                    let data_length =
                        import_symmetric_key(ctx, format, kind, data, algorithm_name, Some(&hash))?;
                    if length == 0 {
                        length = data_length as u16
                    }
                    Some(kind)
                } else {
                    None
                };
                #[cfg(not(feature = "_subtle-full"))]
                let key_kind: Option<&KeyKind> = None;

                KeyUsage::classify_and_check_usages(
                    ctx,
                    KeyUsageAlgorithm::Hmac,
                    &usages,
                    &mut private_usages,
                    &mut public_usages,
                    key_kind.as_deref(),
                )?;

                KeyAlgorithm::Hmac { hash, length }
            },
            "RSA-OAEP" | "RSA-PSS" | "RSASSA-PKCS1-v1_5" => {
                let obj = obj.or_throw(ctx)?;
                let hash = extract_sha_hash(ctx, &obj)?;

                #[cfg(feature = "_subtle-full")]
                let (modulus_length, public_exponent, key_kind) =
                    if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                        let (mod_length, exp) =
                            import_rsa_key(ctx, format, kind, data, algorithm_name, &hash)?;
                        (mod_length, exp, Some(kind))
                    } else {
                        let modulus_length = obj.get_required("modulusLength", "algorithm")?;
                        let public_exponent: TypedArray<u8> =
                            obj.get_required("publicExponent", "algorithm")?;
                        let public_exponent = public_exponent
                            .as_bytes()
                            .ok_or_else(|| {
                                Exception::throw_message(ctx, "Array buffer has been detached")
                            })?
                            .to_owned()
                            .into_boxed_slice();
                        (modulus_length, public_exponent, None)
                    };

                #[cfg(not(feature = "_subtle-full"))]
                let (modulus_length, public_exponent, key_kind): (
                    u32,
                    Box<[u8]>,
                    Option<&KeyKind>,
                ) = {
                    let modulus_length = obj.get_required("modulusLength", "algorithm")?;
                    let public_exponent: TypedArray<u8> =
                        obj.get_required("publicExponent", "algorithm")?;
                    let public_exponent = public_exponent
                        .as_bytes()
                        .ok_or_else(|| {
                            Exception::throw_message(ctx, "Array buffer has been detached")
                        })?
                        .to_owned()
                        .into_boxed_slice();
                    (modulus_length, public_exponent, None)
                };

                KeyUsage::classify_and_check_usages(
                    ctx,
                    if name == "RSA-OAEP" {
                        KeyUsageAlgorithm::RsaOaep
                    } else {
                        KeyUsageAlgorithm::Sign
                    },
                    &usages,
                    &mut private_usages,
                    &mut public_usages,
                    key_kind.as_deref(),
                )?;

                let public_exponent = Rc::new(public_exponent);

                KeyAlgorithm::Rsa {
                    modulus_length,
                    public_exponent,
                    hash,
                }
            },
            "HKDF" => {
                let (algorithm, key_kind): (KeyAlgorithm, Option<&mut KeyKind>) = match mode {
                    #[cfg(feature = "_subtle-full")]
                    KeyAlgorithmMode::Import { format, kind, data } => {
                        import_derive_key(ctx, format, kind, data, algorithm_name)?;

                        (KeyAlgorithm::HkdfImport, Some(kind))
                    },
                    KeyAlgorithmMode::Derive => {
                        let obj = obj.or_throw(ctx)?;
                        (
                            KeyAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?),
                            None,
                        )
                    },
                    _ => {
                        return algorithm_not_supported_error(ctx);
                    },
                };
                KeyUsage::classify_and_check_usages(
                    ctx,
                    KeyUsageAlgorithm::Derive,
                    &usages,
                    &mut private_usages,
                    &mut public_usages,
                    key_kind.as_deref(),
                )?;
                algorithm
            },

            "PBKDF2" => {
                let (algorithm, key_kind): (KeyAlgorithm, Option<&mut KeyKind>) = match mode {
                    #[cfg(feature = "_subtle-full")]
                    KeyAlgorithmMode::Import { format, kind, data } => {
                        import_derive_key(ctx, format, kind, data, algorithm_name)?;
                        (KeyAlgorithm::Pbkdf2Import, Some(kind))
                    },
                    KeyAlgorithmMode::Derive => {
                        let obj = obj.or_throw(ctx)?;
                        (
                            KeyAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?),
                            None,
                        )
                    },
                    _ => {
                        return algorithm_not_supported_error(ctx);
                    },
                };
                KeyUsage::classify_and_check_usages(
                    ctx,
                    KeyUsageAlgorithm::Derive,
                    &usages,
                    &mut private_usages,
                    &mut public_usages,
                    key_kind.as_deref(),
                )?;
                algorithm
            },
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
            KeyAlgorithm::Aes { length } => {
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
        obj: std::result::Result<Object<'js>, &str>,
        #[allow(unused_variables)] algorithm_name: &str,
        algorithm: EcAlgorithm,
        key_usages: &Array<'js>,
        private_usages: &mut Vec<String>,
        public_usages: &mut Vec<String>,
        key_usage_algorithm: KeyUsageAlgorithm,
    ) -> Result<KeyAlgorithm> {
        let obj = obj.or_throw(ctx)?;
        let curve_name: String = obj.get_required("namedCurve", "algorithm")?;
        let curve = EllipticCurve::try_from(curve_name.as_str()).or_throw(ctx)?;

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
        return Err(Exception::throw_message(
            ctx,
            &[algorithm_name, " only supports 'raw' import format"].concat(),
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
    hash: &ShaAlgorithm,
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
            let kty: String = object.get_required("kty", "keyData")?;
            let alg: String = object.get_required("alg", "keyData")?;
            if kty != "RSA" {
                return algorithm_mismatch_error(ctx, algorithm_name);
            }
            let prefix = &alg[..2];
            let numeric_hash_str = match prefix {
                "RS" => {
                    if algorithm_name == "RSA-OAEP" {
                        if !alg.starts_with(algorithm_name) {
                            return algorithm_mismatch_error(ctx, algorithm_name);
                        }
                        &alg["RSA-OAEP-".len()..]
                    } else if algorithm_name != "RSASSA-PKCS1-v1_5" {
                        return algorithm_mismatch_error(ctx, algorithm_name);
                    } else {
                        &alg["RS".len()..]
                    }
                },
                "PS" => {
                    if algorithm_name != "RSA-PSS" {
                        return algorithm_mismatch_error(ctx, algorithm_name);
                    }
                    &alg["PS".len()..]
                },
                _ => return algorithm_mismatch_error(ctx, algorithm_name),
            };
            if numeric_hash_str != hash.as_numeric_str() {
                return hash_mismatch_error(ctx, hash);
            }

            let n: String = object.get_required("n", "keyData")?;
            let e: String = object.get_required("e", "keyData")?;
            let n_bytes = bytes_from_b64_url_safe(n.as_bytes()).or_throw(ctx)?;
            let e_bytes = bytes_from_b64_url_safe(e.as_bytes()).or_throw(ctx)?;

            let result = if let Some(d) = object.get_optional::<_, String>("d")? {
                let p: String = object.get_required("p", "keyData")?;
                let q: String = object.get_required("q", "keyData")?;
                let dp: String = object.get_required("dp", "keyData")?;
                let dq: String = object.get_required("dq", "keyData")?;
                let qi: String = object.get_required("qi", "keyData")?;

                let d_bytes = bytes_from_b64_url_safe(d.as_bytes()).or_throw(ctx)?;
                let p_bytes = bytes_from_b64_url_safe(p.as_bytes()).or_throw(ctx)?;
                let q_bytes = bytes_from_b64_url_safe(q.as_bytes()).or_throw(ctx)?;
                let dp_bytes = bytes_from_b64_url_safe(dp.as_bytes()).or_throw(ctx)?;
                let dq_bytes = bytes_from_b64_url_safe(dq.as_bytes()).or_throw(ctx)?;
                let qi_bytes = bytes_from_b64_url_safe(qi.as_bytes()).or_throw(ctx)?;

                let jwk = RsaJwkImport {
                    n: &n_bytes,
                    e: &e_bytes,
                    d: Some(&d_bytes),
                    p: Some(&p_bytes),
                    q: Some(&q_bytes),
                    dp: Some(&dp_bytes),
                    dq: Some(&dq_bytes),
                    qi: Some(&qi_bytes),
                };
                CRYPTO_PROVIDER.import_rsa_jwk(jwk).or_throw(ctx)?
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
                CRYPTO_PROVIDER.import_rsa_jwk(jwk).or_throw(ctx)?
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
                .or_throw(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Public;
            (result.modulus_length as usize, result.public_exponent)
        },
        KeyFormatData::Pkcs8(object_bytes) => {
            let pk_info = PrivateKeyInfoRef::from_der(object_bytes.as_bytes(ctx)?).or_throw(ctx)?;
            validate_oid(pk_info.algorithm.oid)?;
            let result = CRYPTO_PROVIDER
                .import_rsa_private_key_pkcs8(object_bytes.as_bytes(ctx)?)
                .or_throw(ctx)?;
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
                .or_throw(ctx)?;
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
    hash: Option<&ShaAlgorithm>,
) -> Result<usize> {
    *kind = KeyKind::Secret;

    match format {
        KeyFormatData::Jwk(object) => {
            let kty: String = object.get_required("kty", "keyData")?;
            if kty == "oct" {
                let k: String = object.get_required("k", "keyData")?;
                let alg: String = object.get_required("alg", "keyData")?;

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
                        let (_, name_suffix) = algorithm_name.split_once("-").unwrap_or_default();
                        let aes_variant = &alg[4..];

                        if aes_variant != name_suffix {
                            return algorithm_mismatch_error(ctx, algorithm_name);
                        }
                    },
                    _ => return algorithm_mismatch_error(ctx, algorithm_name),
                }

                *data = bytes_from_b64_url_safe(k.as_bytes()).or_throw(ctx)?;
                return Ok(data.len() * 8);
            }
        },
        KeyFormatData::Raw(object_bytes) => {
            let bytes = object_bytes.into_bytes(ctx)?;

            *data = bytes;
            return Ok(data.len() * 8);
        },
        _ => {},
    }
    algorithm_mismatch_error(ctx, algorithm_name)
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
            let kty: String = object.get_required("kty", "keyData")?;
            if kty != "EC" {
                return algorithm_mismatch_error(ctx, algorithm_name);
            }

            let jwk_crv: String = object.get_required("crv", "keyData")?;
            if curve_name != jwk_crv {
                return Err(Exception::throw_type(
                    ctx,
                    &["Key is using a ", curve_name].concat(),
                ));
            }

            let x: String = object.get_required("x", "keyData")?;
            let y: String = object.get_required("y", "keyData")?;
            let mut x_bytes = bytes_from_b64_url_safe(x.as_bytes()).or_throw(ctx)?;
            let mut y_bytes = bytes_from_b64_url_safe(y.as_bytes()).or_throw(ctx)?;

            // Pad to coordinate length if needed
            if x_bytes.len() < coord_len {
                let mut padded = vec![0u8; coord_len - x_bytes.len()];
                padded.extend_from_slice(&x_bytes);
                x_bytes = padded;
            }
            if y_bytes.len() < coord_len {
                let mut padded = vec![0u8; coord_len - y_bytes.len()];
                padded.extend_from_slice(&y_bytes);
                y_bytes = padded;
            }

            let d_bytes = if let Some(d) = object.get_optional::<_, String>("d")? {
                let mut d_bytes = bytes_from_b64_url_safe(d.as_bytes()).or_throw(ctx)?;
                if d_bytes.len() < coord_len {
                    let mut padded = vec![0u8; coord_len - d_bytes.len()];
                    padded.extend_from_slice(&d_bytes);
                    d_bytes = padded;
                }
                Some(d_bytes)
            } else {
                None
            };

            let jwk = EcJwkImport {
                x: &x_bytes,
                y: &y_bytes,
                d: d_bytes.as_deref(),
            };

            let result = CRYPTO_PROVIDER.import_ec_jwk(jwk, *curve).or_throw(ctx)?;
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
                .or_throw(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Public;
        },
        KeyFormatData::Spki(object_bytes) => {
            let spki = spki::SubjectPublicKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?)
                .or_throw(ctx)?;
            validate_oid(spki.algorithm.oid)?;
            let result = CRYPTO_PROVIDER
                .import_ec_public_key_spki(object_bytes.as_bytes(ctx)?)
                .or_throw(ctx)?;
            *data = result.key_data;
            *kind = KeyKind::Public;
        },
        KeyFormatData::Pkcs8(object_bytes) => {
            let pkcs8 = PrivateKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?).or_throw(ctx)?;
            validate_oid(pkcs8.algorithm.oid)?;
            let result = CRYPTO_PROVIDER
                .import_ec_private_key_pkcs8(object_bytes.as_bytes(ctx)?)
                .or_throw(ctx)?;
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
) -> Result<()> {
    let validate_oid = |other_oid: const_oid::ObjectIdentifier| -> Result<()> {
        if other_oid != oid {
            return algorithm_mismatch_error(ctx, algorithm_name);
        }
        Ok(())
    };

    match format {
        KeyFormatData::Jwk(object) => {
            let crv: String = object.get_required("crv", "keyData")?;
            if crv != algorithm_name {
                return algorithm_mismatch_error(ctx, algorithm_name);
            }
            let x: String = object.get_required("x", "keyData")?;
            let public_key = bytes_from_b64_url_safe(x.as_bytes()).or_throw(ctx)?;

            if let Some(d) = object.get_optional::<_, String>("d")? {
                let private_key = bytes_from_b64_url_safe(d.as_bytes()).or_throw(ctx)?;

                let pk_info = PrivateKeyInfoRef::new(
                    AlgorithmIdentifier {
                        oid,
                        parameters: None,
                    },
                    OctetStringRef::new(private_key.as_slice()).or_throw(ctx)?,
                );

                *data = pk_info.to_der().or_throw(ctx)?;
                *kind = KeyKind::Private;
            } else {
                *data = public_key;
                *kind = KeyKind::Public;
            }
        },
        KeyFormatData::Raw(object_bytes) => {
            let bytes = object_bytes.into_bytes(ctx)?;
            if bytes.len() != 32 {
                return Err(Exception::throw_type(
                    ctx,
                    &[algorithm_name, " keys must be 32 bytes long"].concat(),
                ));
            }
            *data = bytes;
            *kind = KeyKind::Public;
        },
        KeyFormatData::Spki(object_bytes) => {
            let spki = spki::SubjectPublicKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?)
                .or_throw(ctx)?;
            validate_oid(spki.algorithm.oid)?;
            *data = spki.subject_public_key.raw_bytes().into();
            *kind = KeyKind::Public;
        },
        KeyFormatData::Pkcs8(object_bytes) => {
            let pkcs8 = PrivateKeyInfoRef::try_from(object_bytes.as_bytes(ctx)?).or_throw(ctx)?;
            validate_oid(pkcs8.algorithm.oid)?;
            *data = object_bytes.into_bytes(ctx)?;
            *kind = KeyKind::Private;
        },
    };
    Ok(())
}

pub fn extract_sha_hash<'js>(ctx: &Ctx<'js>, obj: &Object<'js>) -> Result<ShaAlgorithm> {
    let hash: Value = obj.get_required("hash", "algorithm")?;
    let hash = if let Some(string) = hash.as_string() {
        string.to_string()
    } else if let Some(obj) = hash.into_object() {
        obj.get_required("name", "hash")
    } else {
        return Err(Exception::throw_message(
            ctx,
            "hash must be a string or an object",
        ));
    }?;
    ShaAlgorithm::try_from(hash.as_str()).or_throw(ctx)
}

fn create_hash_object<'js>(ctx: &Ctx<'js>, hash: &ShaAlgorithm) -> Result<Object<'js>> {
    let hash_obj = Object::new(ctx.clone())?;
    hash_obj.set(PredefinedAtom::Name, hash.as_str())?;
    Ok(hash_obj)
}

#[cfg(feature = "_subtle-full")]
pub fn hash_mismatch_error<T>(ctx: &Ctx<'_>, hash: &ShaAlgorithm) -> Result<T> {
    Err(Exception::throw_message(
        ctx,
        &["Algorithm hash expected to be ", hash.as_str()].concat(),
    ))
}
