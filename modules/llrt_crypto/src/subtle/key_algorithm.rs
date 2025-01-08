use der::Encode;
use llrt_encoding::bytes_from_b64_url_safe;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use pkcs8::PrivateKeyInfo;
use rquickjs::{atom::PredefinedAtom, Array, Ctx, Exception, Object, Result, TypedArray, Value};
use spki::{AlgorithmIdentifier, ObjectIdentifier};

use std::rc::Rc;

use crate::sha_hash::ShaAlgorithm;

use super::{
    algorithm_not_supported_error, crypto_key::KeyKind, to_name_and_maybe_object, EllipticCurve,
};

static SYMMETRIC_USAGES: &[&str] = &["encrypt", "decrypt", "wrapKey", "unwrapKey"];
static SIGNATURE_USAGES: &[&str] = &["sign", "verify"];
static EMPTY_USAGES: &[&str] = &[];
static SIGN_USAGES: &[&str] = &["sign"];
static RSA_OAEP_USAGES: &[&str] = &["decrypt", "unwrapKey"];
static ECDH_X25519_USAGES: &[&str] = &["deriveKey", "deriveBits"];
static AES_KW_USAGES: &[&str] = &["wrapKey", "unwrapKey"];

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
            .into_bytes()
            .into_boxed_slice();

        let info = obj
            .get_required::<_, ObjectBytes>("info", "algorithm")?
            .into_bytes()
            .into_boxed_slice();

        Ok(KeyDerivation::Hkdf { hash, salt, info })
    }

    pub fn for_pbkf2_object<'js>(ctx: &&Ctx<'js>, obj: Object<'js>) -> Result<Self> {
        let hash = extract_sha_hash(ctx, &obj)?;

        let salt = obj
            .get_required::<_, ObjectBytes>("salt", "algorithm")?
            .into_bytes()
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

#[derive(PartialEq)]
pub enum KeyFormat<'js> {
    Jwk(Object<'js>),
    Raw(ObjectBytes<'js>),
    Spki(ObjectBytes<'js>),
    Pkcs8(ObjectBytes<'js>),
}

impl KeyFormat<'_> {}

#[derive(PartialEq)]
pub enum KeyAlgorithmMode<'a, 'js> {
    Import {
        format: KeyFormat<'js>,
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
        let (name, obj) = to_name_and_maybe_object(ctx, value)?;
        let mut public_usages = vec![];
        let mut private_usages = vec![];
        let name_ref = name.as_str();
        let mut is_symmetric = false;
        let algorithm = match name_ref {
            "Ed25519" => {
                if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                    import_ed25519(ctx, format, kind, data)?;
                }

                Self::classify_and_check_signature_usages(
                    ctx,
                    name_ref,
                    &usages,
                    is_symmetric,
                    &mut private_usages,
                    &mut public_usages,
                )?;
                KeyAlgorithm::Ed25519
            },
            "X25519" => {
                if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                    import_x25519(ctx, format, kind, data)?;
                }

                Self::classify_and_check_ecdh_x25519_usages(
                    ctx,
                    name_ref,
                    &usages,
                    is_symmetric,
                    &mut private_usages,
                    &mut public_usages,
                )?;
                KeyAlgorithm::X25519
            },
            "AES-CBC" | "AES-CTR" | "AES-GCM" | "AES-KW" => {
                is_symmetric = true;
                if name_ref == "AES-KW" {
                    Self::classify_and_check_usages(
                        ctx,
                        name_ref,
                        &usages,
                        AES_KW_USAGES,
                        EMPTY_USAGES,
                        is_symmetric,
                        &mut private_usages,
                        &mut public_usages,
                    )?;
                } else {
                    Self::classify_and_check_symmetric_usages(
                        ctx,
                        name_ref,
                        &usages,
                        is_symmetric,
                        &mut private_usages,
                        &mut public_usages,
                    )?;
                }

                let length = if let KeyAlgorithmMode::Import { data, .. } = mode {
                    data.len() as u16
                } else {
                    obj.or_throw(ctx)?.get_required("length", "algorithm")?
                };

                if !matches!(length, 128 | 192 | 256) {
                    return Err(Exception::throw_type(
                        ctx,
                        "Algorithm 'length' must be one of: 128, 192, or 256",
                    ));
                }

                KeyAlgorithm::Aes { length }
            },
            "ECDH" | "ECDSA" => {
                let obj = obj.or_throw(ctx)?;
                let curive: String = obj.get_required("namedCurve", "algorithm")?;
                let curve = EllipticCurve::try_from(curive.as_str()).or_throw(ctx)?;
                let mut algorithm = EcAlgorithm::Ecdh;
                if !matches!(mode, KeyAlgorithmMode::Import { .. }) {
                    match name_ref {
                        "ECDH" => match mode {
                            KeyAlgorithmMode::Generate => {
                                Self::classify_and_check_ecdh_x25519_usages(
                                    ctx,
                                    name_ref,
                                    &usages,
                                    is_symmetric,
                                    &mut private_usages,
                                    &mut public_usages,
                                )?
                            },
                            KeyAlgorithmMode::Derive => Self::classify_and_check_symmetric_usages(
                                ctx,
                                name_ref,
                                &usages,
                                is_symmetric,
                                &mut private_usages,
                                &mut public_usages,
                            )?,
                            _ => unreachable!(),
                        },
                        "ECDSA" => {
                            algorithm = EcAlgorithm::Ecdsa;
                            Self::classify_and_check_signature_usages(
                                ctx,
                                name_ref,
                                &usages,
                                is_symmetric,
                                &mut private_usages,
                                &mut public_usages,
                            )?
                        },
                        _ => unreachable!(),
                    }
                }
                KeyAlgorithm::Ec { curve, algorithm }
            },

            "HMAC" => {
                is_symmetric = true;
                Self::classify_and_check_usages(
                    ctx,
                    name_ref,
                    &usages,
                    SIGNATURE_USAGES,
                    EMPTY_USAGES,
                    is_symmetric,
                    &mut private_usages,
                    &mut public_usages,
                )?;

                let obj = obj.or_throw(ctx)?;
                let hash = extract_sha_hash(ctx, &obj)?;

                let mut length = obj.get_optional("length")?.unwrap_or_default();

                if length == 0 {
                    if let KeyAlgorithmMode::Import { data, .. } = mode {
                        length = data.len() as u16
                    }
                }

                KeyAlgorithm::Hmac { hash, length }
            },
            "RSA-OAEP" | "RSA-PSS" | "RSASSA-PKCS1-v1_5" => {
                if !matches!(mode, KeyAlgorithmMode::Import { .. }) {
                    if name == "RSA-OAEP" {
                        Self::classify_and_check_usages(
                            ctx,
                            name_ref,
                            &usages,
                            SYMMETRIC_USAGES,
                            RSA_OAEP_USAGES,
                            is_symmetric,
                            &mut private_usages,
                            &mut public_usages,
                        )?;
                    } else {
                        Self::classify_and_check_signature_usages(
                            ctx,
                            name_ref,
                            &usages,
                            is_symmetric,
                            &mut private_usages,
                            &mut public_usages,
                        )?;
                    }
                }

                let obj = obj.or_throw(ctx)?;
                let hash = extract_sha_hash(ctx, &obj)?;

                let modulus_length = obj.get_required("modulusLength", "algorithm")?;
                let public_exponent: TypedArray<u8> =
                    obj.get_required("publicExponent", "algorithm")?;
                let public_exponent: Box<[u8]> = public_exponent
                    .as_bytes()
                    .ok_or_else(|| Exception::throw_message(ctx, "array buffer has been detached"))?
                    .into();
                let public_exponent = Rc::new(public_exponent);

                KeyAlgorithm::Rsa {
                    modulus_length,
                    public_exponent,
                    hash,
                }
            },
            "HKDF" => match mode {
                KeyAlgorithmMode::Import { .. } => KeyAlgorithm::HkdfImport,
                KeyAlgorithmMode::Derive => {
                    Self::classify_and_check_symmetric_usages(
                        ctx,
                        name_ref,
                        &usages,
                        is_symmetric,
                        &mut private_usages,
                        &mut public_usages,
                    )?;

                    let obj = obj.or_throw(ctx)?;
                    KeyAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?)
                },
                _ => {
                    return algorithm_not_supported_error(ctx);
                },
            },

            "PBKDF2" => match mode {
                KeyAlgorithmMode::Import { .. } => KeyAlgorithm::Pbkdf2Import,
                KeyAlgorithmMode::Derive => {
                    Self::classify_and_check_symmetric_usages(
                        ctx,
                        name_ref,
                        &usages,
                        is_symmetric,
                        &mut private_usages,
                        &mut public_usages,
                    )?;

                    let obj = obj.or_throw(ctx)?;
                    KeyAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?)
                },
                _ => {
                    return algorithm_not_supported_error(ctx);
                },
            },
            _ => return algorithm_not_supported_error(ctx),
        };

        //some import key algorithms allows for unchecked usages, let's just classify
        if public_usages.is_empty() && private_usages.is_empty() {
            for usage in usages.iter() {
                let usage = usage?;
                classify_usage(usage, is_symmetric, &mut private_usages, &mut public_usages);
            }
        }

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

    fn classify_and_check_signature_usages<'js>(
        ctx: &Ctx<'js>,
        name: &str,
        usages: &Array<'js>,
        is_symmetric: bool,
        private_usages: &mut Vec<String>,
        public_usages: &mut Vec<String>,
    ) -> Result<()> {
        Self::classify_and_check_usages(
            ctx,
            name,
            usages,
            SIGNATURE_USAGES,
            SIGN_USAGES,
            is_symmetric,
            private_usages,
            public_usages,
        )
    }

    fn classify_and_check_ecdh_x25519_usages<'js>(
        ctx: &Ctx<'js>,
        name: &str,
        usages: &Array<'js>,
        is_symmetric: bool,
        private_usages: &mut Vec<String>,
        public_usages: &mut Vec<String>,
    ) -> Result<()> {
        Self::classify_and_check_usages(
            ctx,
            name,
            usages,
            ECDH_X25519_USAGES,
            EMPTY_USAGES,
            is_symmetric,
            private_usages,
            public_usages,
        )
    }

    fn classify_and_check_symmetric_usages<'js>(
        ctx: &Ctx<'js>,
        name: &str,
        usages: &Array<'js>,
        is_symmetric: bool,
        private_usages: &mut Vec<String>,
        public_usages: &mut Vec<String>,
    ) -> Result<()> {
        Self::classify_and_check_usages(
            ctx,
            name,
            usages,
            SYMMETRIC_USAGES,
            EMPTY_USAGES,
            is_symmetric,
            private_usages,
            public_usages,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn classify_and_check_usages<'js>(
        ctx: &Ctx<'js>,
        name: &str,
        key_usages: &Array<'js>,
        allowed_usages: &[&str],
        required_usages: &[&str],
        is_symmetric: bool,
        private_usages: &mut Vec<String>,
        public_usages: &mut Vec<String>,
    ) -> Result<()> {
        let usages_len = key_usages.len();

        let mut generated_public_usages = Vec::with_capacity(usages_len);
        let mut generated_private_usages = Vec::with_capacity(usages_len);
        let mut has_any_required_usages = required_usages.is_empty();
        for usage in key_usages.iter::<String>() {
            let value = usage?;
            if !allowed_usages.contains(&value.as_str()) {
                return Err(Exception::throw_range(
                    ctx,
                    &["'", &value, "' is not supported for ", name].concat(),
                ));
            }

            if !has_any_required_usages {
                has_any_required_usages = required_usages.contains(&value.as_str());
            }

            classify_usage(
                value,
                is_symmetric,
                &mut generated_private_usages,
                &mut generated_public_usages,
            );
        }

        if !has_any_required_usages {
            return Err(Exception::throw_range(
                ctx,
                &[name, " is missing some required usages"].concat(),
            ));
        }

        *public_usages = generated_public_usages;
        *private_usages = generated_private_usages;

        Ok(())
    }
}

fn import_crv_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    oid: ObjectIdentifier,
    name: &str,
) -> Result<()> {
    let validate_oid = |other_oid: const_oid::ObjectIdentifier| -> Result<()> {
        if other_oid != oid {
            return Err(Exception::throw_type(
                ctx,
                &["Only ", name, " keys are supported"].concat(),
            ));
        }
        Ok(())
    };

    match format {
        KeyFormat::Jwk(object) => {
            let crv: String = object.get_required("crv", "keyData")?;
            if crv != name {
                return Err(Exception::throw_type(
                    ctx,
                    &["Only ", name, " keys are supported"].concat(),
                ));
            }
            let x: String = object.get_required("x", "keyData")?;
            let public_key = bytes_from_b64_url_safe(x.as_bytes()).or_throw(ctx)?;

            if let Some(d) = object.get_optional::<_, String>("d")? {
                let private_key = bytes_from_b64_url_safe(d.as_bytes()).or_throw(ctx)?;

                let pk_info = PrivateKeyInfo::new(
                    AlgorithmIdentifier {
                        oid,
                        parameters: None,
                    },
                    &private_key,
                );

                *data = pk_info.to_der().or_throw(ctx)?;
                *kind = KeyKind::Private;
            } else {
                *data = public_key;
                *kind = KeyKind::Public;
            }
        },
        KeyFormat::Raw(object_bytes) => {
            let bytes = object_bytes.into_bytes();
            if bytes.len() != 32 {
                return Err(Exception::throw_type(
                    ctx,
                    &[name, " keys must be 32 bytes long"].concat(),
                ));
            }
            *data = bytes;
            *kind = KeyKind::Public;
        },
        KeyFormat::Spki(object_bytes) => {
            let spki =
                spki::SubjectPublicKeyInfoRef::try_from(object_bytes.as_bytes()).or_throw(ctx)?;
            validate_oid(spki.algorithm.oid)?;
            *data = spki.subject_public_key.raw_bytes().into();
            *kind = KeyKind::Public;
        },
        KeyFormat::Pkcs8(object_bytes) => {
            let pkcs8 = PrivateKeyInfo::try_from(object_bytes.as_bytes()).or_throw(ctx)?;
            validate_oid(pkcs8.algorithm.oid)?;
            *data = object_bytes.into_bytes();
            *kind = KeyKind::Private;
        },
    };
    Ok(())
}

fn import_ed25519<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
) -> Result<()> {
    import_crv_key(
        ctx,
        format,
        kind,
        data,
        const_oid::db::rfc8410::ID_ED_25519,
        "Ed25519",
    )
}

fn import_x25519<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
) -> Result<()> {
    import_crv_key(
        ctx,
        format,
        kind,
        data,
        const_oid::db::rfc8410::ID_X_25519,
        "X25519",
    )
}

fn classify_usage(
    value: String,
    is_symmetric: bool,
    private_usages: &mut Vec<String>,
    public_usages: &mut Vec<String>,
) {
    if is_symmetric {
        public_usages.push(value);
        return;
    }
    match value.as_str() {
        "sign" | "decrypt" | "unwrapKey" | "deriveKey" | "deriveBits" => {
            private_usages.push(value);
        },
        _ => {
            public_usages.push(value);
        },
    }
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
