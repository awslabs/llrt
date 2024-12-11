use std::rc::Rc;

use llrt_utils::object::ObjectExt;
use rquickjs::{Class, Ctx, Exception, FromJs, Result, Value};

use super::{
    key_algorithm::{KeyAlgorithm, KeyDerivation},
    CryptoKey, EllipticCurve,
};

#[derive(Debug)]
pub enum DeriveAlgorithm {
    Edch {
        curve: EllipticCurve,
        public: Rc<[u8]>,
    },
    Derive(KeyDerivation),
}

impl<'js> FromJs<'js> for DeriveAlgorithm {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let obj = value.into_object_or_throw(ctx, "algorithm")?;

        let name: String = obj.get_required("name", "algorithm")?;

        Ok(match name.as_str() {
            "ECDH" => {
                let public_key: Class<CryptoKey> = obj.get_required("public", "algorithm")?;
                let public_key = public_key.borrow();
                let curve = if let KeyAlgorithm::Ec { curve } = &public_key.algorithm {
                    curve.clone()
                } else {
                    return Err(Exception::throw_message(
                        ctx,
                        "public key must be ECDSA or ECDH key",
                    ));
                };

                DeriveAlgorithm::Edch {
                    curve,
                    public: public_key.handle.clone(),
                }
            },
            "HKDF" => DeriveAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?),
            "PBKDF2" => DeriveAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?),
            _ => {
                return Err(Exception::throw_message(
                    ctx,
                    "Algorithm 'name' must be ECDH | HKDF | PBKDF2",
                ))
            },
        })
    }
}
