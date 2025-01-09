use der::asn1::UintRef;
use der::Decode;
use der::Encode;
use llrt_encoding::bytes_from_b64_url_safe;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use pkcs8::EncodePrivateKey;
use pkcs8::PrivateKeyInfo;
use rquickjs::{atom::PredefinedAtom, Array, Ctx, Exception, Object, Result, TypedArray, Value};
use spki::{AlgorithmIdentifier, ObjectIdentifier};

use std::rc::Rc;

use crate::sha_hash::ShaAlgorithm;

use super::algorithm_mismatch_error;
use super::{
    algorithm_not_supported_error, crypto_key::KeyKind, to_name_and_maybe_object, EllipticCurve,
};

static SYMMETRIC_USAGES: &[&str] = &["encrypt", "decrypt", "wrapKey", "unwrapKey"];
static SIGNATURE_USAGES: &[&str] = &["sign", "verify"];
static EMPTY_USAGES: &[&str] = &[];
static SIGN_USAGES: &[&str] = &["sign"];
static RSA_OAEP_USAGES: &[&str] = &["decrypt", "unwrapKey"];
static DERIVE_USAGES: &[&str] = &["deriveKey", "deriveBits"];
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
        let algorithm_name = name.as_str();
        let mut is_symmetric = false;
        let algorithm = match algorithm_name {
            "Ed25519" => {
                if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                    import_okp_key(
                        ctx,
                        format,
                        kind,
                        data,
                        const_oid::db::rfc8410::ID_ED_25519,
                        algorithm_name,
                    )?;
                }

                Self::classify_and_check_signature_usages(
                    ctx,
                    algorithm_name,
                    &usages,
                    is_symmetric,
                    &mut private_usages,
                    &mut public_usages,
                )?;
                KeyAlgorithm::Ed25519
            },
            "X25519" => {
                if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                    import_okp_key(
                        ctx,
                        format,
                        kind,
                        data,
                        const_oid::db::rfc8410::ID_X_25519,
                        algorithm_name,
                    )?;
                }

                Self::classify_and_check_derive_usages(
                    ctx,
                    algorithm_name,
                    &usages,
                    is_symmetric,
                    &mut private_usages,
                    &mut public_usages,
                )?;
                KeyAlgorithm::X25519
            },
            "AES-CBC" | "AES-CTR" | "AES-GCM" | "AES-KW" => {
                is_symmetric = true;
                if algorithm_name == "AES-KW" {
                    Self::classify_and_check_usages(
                        ctx,
                        algorithm_name,
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
                        algorithm_name,
                        &usages,
                        is_symmetric,
                        &mut private_usages,
                        &mut public_usages,
                    )?;
                }

                let length = if let KeyAlgorithmMode::Import { data, format, kind } = mode {
                    import_symmetric_key(ctx, format, kind, data, algorithm_name, None)?
                } else {
                    obj.or_throw(ctx)?.get_required("length", "algorithm")?
                } as u16;

                //eprintln!("Length: {}", length);

                if !matches!(length, 128 | 192 | 256) {
                    return Err(Exception::throw_message(
                        ctx,
                        &format!(
                            "Algorithm 'length' must be one of: 128, 192, or 256 = {}",
                            length
                        ),
                    ));
                }

                KeyAlgorithm::Aes { length }
            },
            "ECDH" => {
                match &mode {
                    KeyAlgorithmMode::Generate => {
                        Self::classify_and_check_derive_usages(
                            ctx,
                            algorithm_name,
                            &usages,
                            is_symmetric,
                            &mut private_usages,
                            &mut public_usages,
                        )?;
                    },
                    KeyAlgorithmMode::Derive => {
                        Self::classify_and_check_symmetric_usages(
                            ctx,
                            algorithm_name,
                            &usages,
                            is_symmetric,
                            &mut private_usages,
                            &mut public_usages,
                        )?;
                    },
                    _ => {},
                }
                Self::from_ec(ctx, mode, obj, algorithm_name, EcAlgorithm::Ecdh)?
            },
            "ECDSA" => {
                Self::classify_and_check_signature_usages(
                    ctx,
                    algorithm_name,
                    &usages,
                    is_symmetric,
                    &mut private_usages,
                    &mut public_usages,
                )?;
                Self::from_ec(ctx, mode, obj, algorithm_name, EcAlgorithm::Ecdsa)?
            },
            "HMAC" => {
                is_symmetric = true;
                Self::classify_and_check_usages(
                    ctx,
                    algorithm_name,
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

                if let KeyAlgorithmMode::Import { data, format, kind } = mode {
                    let data_length =
                        import_symmetric_key(ctx, format, kind, data, algorithm_name, Some(&hash))?;
                    if length == 0 {
                        length = data_length as u16
                    }
                }

                KeyAlgorithm::Hmac { hash, length }
            },
            "RSA-OAEP" | "RSA-PSS" | "RSASSA-PKCS1-v1_5" => {
                if name == "RSA-OAEP" {
                    Self::classify_and_check_usages(
                        ctx,
                        algorithm_name,
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
                        algorithm_name,
                        &usages,
                        is_symmetric,
                        &mut private_usages,
                        &mut public_usages,
                    )?;
                }

                let obj = obj.or_throw(ctx)?;
                let hash = extract_sha_hash(ctx, &obj)?;

                let (modulus_length, public_exponent) =
                    if let KeyAlgorithmMode::Import { format, kind, data } = mode {
                        import_rsa_key(ctx, format, kind, data, algorithm_name, &hash)?
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
                        (modulus_length, public_exponent)
                    };

                let public_exponent = Rc::new(public_exponent);

                KeyAlgorithm::Rsa {
                    modulus_length,
                    public_exponent,
                    hash,
                }
            },
            "HKDF" => {
                is_symmetric = true;
                match mode {
                    KeyAlgorithmMode::Import { format, kind, data } => {
                        Self::classify_and_check_derive_usages(
                            ctx,
                            algorithm_name,
                            &usages,
                            is_symmetric,
                            &mut private_usages,
                            &mut public_usages,
                        )?;
                        import_derive_key(ctx, format, kind, data, algorithm_name)?;
                        KeyAlgorithm::HkdfImport
                    },
                    KeyAlgorithmMode::Derive => {
                        Self::classify_and_check_symmetric_usages(
                            ctx,
                            algorithm_name,
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
                }
            },

            "PBKDF2" => {
                is_symmetric = true;
                match mode {
                    KeyAlgorithmMode::Import { format, kind, data } => {
                        Self::classify_and_check_derive_usages(
                            ctx,
                            algorithm_name,
                            &usages,
                            is_symmetric,
                            &mut private_usages,
                            &mut public_usages,
                        )?;
                        import_derive_key(ctx, format, kind, data, algorithm_name)?;
                        KeyAlgorithm::Pbkdf2Import
                    },
                    KeyAlgorithmMode::Derive => {
                        Self::classify_and_check_symmetric_usages(
                            ctx,
                            algorithm_name,
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
                }
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

    fn classify_and_check_derive_usages<'js>(
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
            DERIVE_USAGES,
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

    fn from_ec<'js>(
        ctx: &Ctx<'js>,
        mode: KeyAlgorithmMode<'_, 'js>,
        obj: std::result::Result<Object<'js>, &str>,
        algorithm_name: &str,
        algorithm: EcAlgorithm,
    ) -> Result<KeyAlgorithm> {
        let obj = obj.or_throw(ctx)?;
        let curve_name: String = obj.get_required("namedCurve", "algorithm")?;
        let curve = EllipticCurve::try_from(curve_name.as_str()).or_throw(ctx)?;

        if let KeyAlgorithmMode::Import { format, kind, data } = mode {
            import_ec_key(ctx, format, kind, data, algorithm_name, &curve, &curve_name)?;
        }
        Ok(KeyAlgorithm::Ec { curve, algorithm })
    }
}

fn import_derive_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
) -> Result<()> {
    if let KeyFormat::Raw(object_bytes) = format {
        *data = object_bytes.into_bytes();
        *kind = KeyKind::Secret;
    } else {
        return Err(Exception::throw_message(
            ctx,
            &[algorithm_name, " only supports 'raw' import format"].concat(),
        ));
    }

    Ok(())
}

fn import_rsa_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
    hash: &ShaAlgorithm,
) -> Result<(u32, Box<[u8]>)> {
    let validate_oid = |other_oid: const_oid::ObjectIdentifier| -> Result<()> {
        if other_oid != const_oid::db::rfc5912::RSA_ENCRYPTION {
            return algorithm_mismatch_error(ctx, algorithm_name);
        }
        Ok(())
    };

    fn public_key_info(
        ctx: &Ctx<'_>,
        kind: &mut KeyKind,
        data: &mut Vec<u8>,
        public_key: rsa::pkcs1::RsaPublicKey<'_>,
    ) -> Result<(usize, Vec<u8>)> {
        *data = public_key.to_der().or_throw(ctx)?;
        *kind = KeyKind::Public;
        let modulus_length = public_key.modulus.as_bytes().len() * 8;
        let public_exponent = public_key.public_exponent.as_bytes().to_vec();
        Ok((modulus_length, public_exponent))
    }

    macro_rules! uint_ref_from_b64 {
        ($name:ident,$ctx:expr,$bytes:expr) => {
            let bytes = bytes_from_b64_url_safe($bytes).or_throw($ctx)?;
            let $name = UintRef::new(&bytes).or_throw($ctx)?;
        };
    }

    let (modulus_length, public_exponent) = match format {
        KeyFormat::Jwk(object) => {
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

            uint_ref_from_b64!(modulus, ctx, n.as_bytes());
            uint_ref_from_b64!(public_exponent, ctx, e.as_bytes());

            if let Some(d) = object.get_optional::<_, String>("d")? {
                let p: String = object.get_required("p", "keyData")?;
                let q: String = object.get_required("q", "keyData")?;
                let dp: String = object.get_required("dp", "keyData")?;
                let dq: String = object.get_required("dq", "keyData")?;
                let qi: String = object.get_required("qi", "keyData")?;

                uint_ref_from_b64!(private_exponent, ctx, d.as_bytes());
                uint_ref_from_b64!(prime1, ctx, p.as_bytes());
                uint_ref_from_b64!(prime2, ctx, q.as_bytes());
                uint_ref_from_b64!(exponent1, ctx, dp.as_bytes());
                uint_ref_from_b64!(exponent2, ctx, dq.as_bytes());
                uint_ref_from_b64!(coefficient, ctx, qi.as_bytes());

                let modulus_length = modulus.as_bytes().len() * 8;

                let private_key = rsa::pkcs1::RsaPrivateKey {
                    modulus,
                    public_exponent,
                    private_exponent,
                    prime1,
                    prime2,
                    exponent1,
                    exponent2,
                    coefficient,
                    other_prime_infos: None,
                };

                *data = private_key.to_der().or_throw(ctx)?;
                *kind = KeyKind::Private;
                (modulus_length, public_exponent.as_bytes().to_vec())
            } else {
                let public_key = rsa::pkcs1::RsaPublicKey {
                    modulus,
                    public_exponent,
                };
                public_key_info(ctx, kind, data, public_key)?
            }
        },
        KeyFormat::Raw(object_bytes) => {
            let public_key =
                rsa::pkcs1::RsaPublicKey::from_der(object_bytes.as_bytes()).or_throw(ctx)?;
            public_key_info(ctx, kind, data, public_key)?
        },
        KeyFormat::Pkcs8(object_bytes) => {
            let pk_info = PrivateKeyInfo::from_der(object_bytes.as_bytes()).or_throw(ctx)?;
            let object_identifier = pk_info.algorithm.oid;
            validate_oid(object_identifier)?;

            let private_key =
                rsa::pkcs1::RsaPrivateKey::from_der(pk_info.private_key).or_throw(ctx)?;

            let public_exponent = private_key.public_exponent.as_bytes().to_vec();
            let modulus_length = private_key.modulus.as_bytes().len() * 8;
            *data = pk_info.private_key.to_vec();
            *kind = KeyKind::Private;

            (modulus_length, public_exponent)
        },
        KeyFormat::Spki(object_bytes) => {
            let pk_info =
                spki::SubjectPublicKeyInfoRef::try_from(object_bytes.as_bytes()).or_throw(ctx)?;

            let object_identifier = pk_info.algorithm.oid;
            validate_oid(object_identifier)?;

            let public_key =
                rsa::pkcs1::RsaPublicKey::from_der(pk_info.subject_public_key.raw_bytes())
                    .or_throw(ctx)?;

            public_key_info(ctx, kind, data, public_key)?
        },
    };

    let public_exponent = public_exponent.into_boxed_slice();
    Ok((modulus_length as u32, public_exponent))
}

fn import_symmetric_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
    hash: Option<&ShaAlgorithm>,
) -> Result<usize> {
    *kind = KeyKind::Secret;

    match format {
        KeyFormat::Jwk(object) => {
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
        KeyFormat::Raw(object_bytes) => {
            let bytes = object_bytes.into_bytes();

            *data = bytes;
            return Ok(data.len() * 8);
        },
        _ => {},
    }
    algorithm_mismatch_error(ctx, algorithm_name)
}

fn import_ec_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
    kind: &mut KeyKind,
    data: &mut Vec<u8>,
    algorithm_name: &str,
    curve: &EllipticCurve,
    curve_name: &str,
) -> Result<()> {
    let validate_oid = |other_oid: const_oid::ObjectIdentifier| -> Result<()> {
        if other_oid != elliptic_curve::ALGORITHM_OID {
            return algorithm_mismatch_error(ctx, algorithm_name);
        }
        Ok(())
    };

    fn decode_to_curve<C: elliptic_curve::Curve>(
        ctx: &Ctx<'_>,
        value: &str,
    ) -> Result<elliptic_curve::FieldBytes<C>> {
        let value_bytes = value.as_bytes();

        let mut field_bytes = elliptic_curve::FieldBytes::<C>::default();
        let mut bytes = bytes_from_b64_url_safe(value_bytes).or_throw(ctx)?;
        if bytes.len() < field_bytes.len() {
            bytes.resize(field_bytes.len() - bytes.len(), 0);
        }

        field_bytes.copy_from_slice(&bytes);

        Ok(field_bytes)
    }

    fn decode_jwk_to_ec_point_bytes(
        ctx: &Ctx<'_>,
        curve: &EllipticCurve,
        x: &str,
        y: &str,
    ) -> Result<Vec<u8>> {
        let point_bytes = match curve {
            EllipticCurve::P256 => {
                let x = decode_to_curve::<p256::NistP256>(ctx, x)?;
                let y = decode_to_curve::<p256::NistP256>(ctx, y)?;

                p256::EncodedPoint::from_affine_coordinates(&x, &y, false).to_bytes()
            },
            EllipticCurve::P384 => {
                let x = decode_to_curve::<p384::NistP384>(ctx, x)?;
                let y = decode_to_curve::<p384::NistP384>(ctx, y)?;

                p384::EncodedPoint::from_affine_coordinates(&x, &y, false).to_bytes()
            },
            EllipticCurve::P521 => {
                let x = decode_to_curve::<p521::NistP521>(ctx, x)?;
                let y = decode_to_curve::<p521::NistP521>(ctx, y)?;

                p521::EncodedPoint::from_affine_coordinates(&x, &y, false).to_bytes()
            },
        };

        Ok(point_bytes.to_vec())
    }

    match format {
        KeyFormat::Jwk(object) => {
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

            if let Some(d) = object.get_optional::<_, String>("d")? {
                let private_key = match curve {
                    EllipticCurve::P256 => {
                        let d = decode_to_curve::<p256::NistP256>(ctx, &d)?;
                        let key = p256::SecretKey::from_bytes(&d).or_throw(ctx)?;
                        key.to_pkcs8_der().or_throw(ctx)?
                    },
                    EllipticCurve::P384 => {
                        let d = decode_to_curve::<p384::NistP384>(ctx, &d)?;
                        let key = p384::SecretKey::from_bytes(&d).or_throw(ctx)?;
                        key.to_pkcs8_der().or_throw(ctx)?
                    },
                    EllipticCurve::P521 => {
                        let d = decode_to_curve::<p521::NistP521>(ctx, &d)?;
                        let key = p521::SecretKey::from_bytes(&d).or_throw(ctx)?;
                        key.to_pkcs8_der().or_throw(ctx)?
                    },
                };

                *data = private_key.as_bytes().to_vec();
                *kind = KeyKind::Private;
            } else {
                *kind = KeyKind::Public;
                let x: String = object.get_required("x", "keyData")?;
                let y: String = object.get_required("y", "keyData")?;

                let point_bytes = decode_jwk_to_ec_point_bytes(ctx, curve, &x, &y)?;
                *data = point_bytes;
            }
        },
        KeyFormat::Raw(object_bytes) => {
            let bytes = object_bytes.into_bytes();
            if bytes.len() != 32 {
                return Err(Exception::throw_type(
                    ctx,
                    &[algorithm_name, " keys must be 32 bytes long"].concat(),
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

fn import_okp_key<'js>(
    ctx: &Ctx<'js>,
    format: KeyFormat<'js>,
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
        KeyFormat::Jwk(object) => {
            let crv: String = object.get_required("crv", "keyData")?;
            if crv != algorithm_name {
                return algorithm_mismatch_error(ctx, algorithm_name);
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
                    &[algorithm_name, " keys must be 32 bytes long"].concat(),
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

pub fn hash_mismatch_error<T>(ctx: &Ctx<'_>, hash: &ShaAlgorithm) -> Result<T> {
    Err(Exception::throw_message(
        ctx,
        &["Algorithm hash expected to be ", hash.as_str()].concat(),
    ))
}
