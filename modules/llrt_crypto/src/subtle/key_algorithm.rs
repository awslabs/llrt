use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{atom::PredefinedAtom, Array, Ctx, Exception, Object, Result, TypedArray, Value};

use std::{collections::HashSet, rc::Rc};

use crate::sha_hash::ShaAlgorithm;

use super::{algorithm_not_supported_error, to_name_and_maybe_object, EllipticCurve};

static SYMMETRIC_USAGES: &[&str] = &["encrypt", "decrypt", "wrapKey", "unwrapKey"];
static SIGNATURE_USAGES: &[&str] = &["sign", "verify"];
static EMPTY_USAGES: &[&str] = &[];
static SIGN_USAGES: &[&str] = &["sign"];
static RSA_OAEP_USAGES: &[&str] = &["decrypt", "unwrapKey"];
static ECDH_USAGES: &[&str] = &["deriveKey", "deriveBits"];
static AES_KW_USAGES: &[&str] = &["wrapKey", "unwrapKey"];

static SUPPORTED_USAGES_ARRAY: &[(&str, &[&str])] = &[
    ("AES-CBC", SYMMETRIC_USAGES),
    ("AES-CTR", SYMMETRIC_USAGES),
    ("AES-GCM", SYMMETRIC_USAGES),
    ("AES-KW", AES_KW_USAGES),
    ("ECDH", ECDH_USAGES),
    ("ECDSA", SIGNATURE_USAGES),
    ("Ed25519", SIGNATURE_USAGES),
    ("HMAC", SIGNATURE_USAGES),
    ("RSA-OAEP", SYMMETRIC_USAGES),
    ("RSA-PSS", SIGNATURE_USAGES),
    ("RSASSA-PKCS1-v1_5", SIGNATURE_USAGES),
    ("PBKDF2", ECDH_USAGES),
    ("HKDF", ECDH_USAGES),
];

static MANDATORY_USAGES_ARRAY: &[(&str, &[&str])] = &[
    ("AES-CBC", EMPTY_USAGES),
    ("AES-CTR", EMPTY_USAGES),
    ("AES-GCM", EMPTY_USAGES),
    ("AES-KW", EMPTY_USAGES),
    ("ECDH", ECDH_USAGES),
    ("ECDSA", SIGN_USAGES),
    ("Ed25519", SIGN_USAGES),
    ("HMAC", EMPTY_USAGES),
    ("RSA-OAEP", RSA_OAEP_USAGES),
    ("RSA-PSS", SIGN_USAGES),
    ("RSASSA-PKCS1-v1_5", SIGN_USAGES),
    ("PBKDF2", EMPTY_USAGES),
    ("HKDF", EMPTY_USAGES),
];

fn find_usages<'a>(
    ctx: &Ctx<'_>,
    table: &'a [(&str, &[&str])],
    algorithm: &str,
) -> Result<&'a [&'a str]> {
    if let Some(res) = table
        .iter()
        .find(|(name, _)| *name == algorithm)
        .map(|(_, usages)| *usages)
    {
        return Ok(res);
    };

    algorithm_not_supported_error(ctx)
}

pub fn classify_and_check_usages<'js>(
    ctx: &Ctx<'js>,
    name: &str,
    key_usages: &Array<'js>,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut key_usages_set = HashSet::with_capacity(8);
    for value in key_usages.clone().into_iter() {
        if let Some(string) = value?.as_string() {
            key_usages_set.insert(string.to_string()?);
        }
    }

    let supported_usages = find_usages(ctx, SUPPORTED_USAGES_ARRAY, name)?;
    let mandatory_usages = find_usages(ctx, MANDATORY_USAGES_ARRAY, name)?;
    let mut private_usages: Vec<String> = Vec::with_capacity(key_usages_set.len());
    let mut public_usages: Vec<String> = Vec::with_capacity(key_usages_set.len());

    for usage in key_usages_set {
        if !supported_usages.contains(&usage.as_str()) {
            return Err(Exception::throw_range(
                ctx,
                &["'", &usage, "' is not supported for ", name].concat(),
            ));
        }
        if mandatory_usages.contains(&usage.as_str()) {
            private_usages.push(usage);
        } else {
            public_usages.push(usage);
        }
    }
    if !mandatory_usages.is_empty() && private_usages.is_empty() {
        return Err(Exception::throw_range(
            ctx,
            &[name, " is missing some mandatory usages"].concat(),
        ));
    }

    Ok((private_usages, public_usages))
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
pub enum KeyAlgorithm {
    Aes {
        length: u16,
    },
    Ec {
        curve: EllipticCurve,
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
pub enum KeyAlgorithmMode {
    Import,
    Generate,
    Derive,
}

impl KeyAlgorithm {
    pub fn from_js<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode,
        value: Value<'js>,
    ) -> Result<(Self, String)> {
        let (name, obj) = to_name_and_maybe_object(ctx, value)?;

        let algorithm = match name.as_str() {
            "Ed25519" => KeyAlgorithm::Ed25519,
            "X25519" => KeyAlgorithm::X25519,
            "AES-CBC" | "AES-CTR" | "AES-GCM" | "AES-KW" => {
                let length: u16 = obj.or_throw(ctx)?.get_required("length", "algorithm")?;

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
                KeyAlgorithm::Ec { curve }
            },

            "HMAC" => {
                let obj = obj.or_throw(ctx)?;
                let hash = extract_sha_hash(ctx, &obj)?;

                let length = obj.get_optional("length")?.unwrap_or_default();

                KeyAlgorithm::Hmac { hash, length }
            },
            "RSA-OAEP" | "RSA-PSS" | "RSASSA-PKCS1-v1_5" => {
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
                KeyAlgorithmMode::Import => KeyAlgorithm::HkdfImport,
                KeyAlgorithmMode::Derive => {
                    let obj = obj.or_throw(ctx)?;
                    KeyAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?)
                },
                _ => {
                    return algorithm_not_supported_error(ctx);
                },
            },

            "PBKDF2" => match mode {
                KeyAlgorithmMode::Import => KeyAlgorithm::Pbkdf2Import,
                KeyAlgorithmMode::Derive => {
                    let obj = obj.or_throw(ctx)?;
                    KeyAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?)
                },
                _ => {
                    return algorithm_not_supported_error(ctx);
                },
            },
            _ => return algorithm_not_supported_error(ctx),
        };
        Ok((algorithm, name))
    }

    pub fn as_object<'js, T: AsRef<str>>(&self, ctx: &Ctx<'js>, name: T) -> Result<Object<'js>> {
        let obj = Object::new(ctx.clone())?;
        obj.set(PredefinedAtom::Name, name.as_ref())?;
        match self {
            KeyAlgorithm::Aes { length } => {
                obj.set(PredefinedAtom::Length, length)?;
            },
            KeyAlgorithm::Ec { curve } => {
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
