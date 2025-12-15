// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use llrt_utils::object::ObjectExt;
use rquickjs::{Class, Ctx, Exception, FromJs, Result, Value};

use super::{
    algorithm_mismatch_error,
    key_algorithm::{KeyAlgorithm, KeyDerivation},
    CryptoKey, EllipticCurve,
};

#[derive(Debug)]
pub enum DeriveAlgorithm {
    X25519 {
        public_key: Rc<[u8]>,
    },
    Ecdh {
        curve: EllipticCurve,
        public_key: Rc<[u8]>,
    },
    Derive(KeyDerivation),
}

impl<'js> FromJs<'js> for DeriveAlgorithm {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let obj = value.into_object_or_throw(ctx, "algorithm")?;

        let name: String = obj.get_required("name", "algorithm")?;

        Ok(match name.as_str() {
            "X25519" => {
                let public_key: Class<CryptoKey> = obj.get_required("public", "algorithm")?;
                let public_key = public_key.borrow();

                if !matches!(public_key.algorithm, KeyAlgorithm::X25519) {
                    return algorithm_mismatch_error(ctx, &name);
                }

                DeriveAlgorithm::X25519 {
                    public_key: public_key.handle.clone(),
                }
            },
            "ECDH" => {
                let public_key: Class<CryptoKey> = obj.get_required("public", "algorithm")?;
                let public_key = public_key.borrow();

                if let KeyAlgorithm::Ec { curve, .. } = &public_key.algorithm {
                    DeriveAlgorithm::Ecdh {
                        curve: *curve,
                        public_key: public_key.handle.clone(),
                    }
                } else {
                    return algorithm_mismatch_error(ctx, &name);
                }
            },
            "HKDF" => DeriveAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?),
            "PBKDF2" => DeriveAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?),
            _ => {
                return Err(Exception::throw_message(
                    ctx,
                    "Algorithm 'name' must be X25519 | ECDH | HKDF | PBKDF2",
                ))
            },
        })
    }
}
