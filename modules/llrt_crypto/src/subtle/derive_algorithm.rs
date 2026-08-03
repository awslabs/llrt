// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use llrt_utils::object::ObjectExt;
use rquickjs::{Class, Ctx, FromJs, Result, Value};

use super::{
    algorithm_invalid_access_error, algorithm_mismatch_error, algorithm_not_supported_error,
    crypto_key::{CryptoKey, KeyKind},
    key_algorithm::{EcAlgorithm, KeyAlgorithm, KeyDerivation},
    normalize_algorithm_name,
    util::ResultDomExt,
    EllipticCurve,
};

#[derive(Debug)]
pub enum DeriveAlgorithm {
    X25519 {
        public_key: Rc<[u8]>,
    },
    Ecdh {
        curve: EllipticCurve,
        ec_algorithm: EcAlgorithm,
        public_key: Rc<[u8]>,
    },
    Derive(KeyDerivation),
}

impl<'js> FromJs<'js> for DeriveAlgorithm {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let obj = value.into_object_or_throw(ctx, "algorithm")?;

        let name: String = obj.get_required("name", "algorithm")?;
        let name = normalize_algorithm_name(&name);

        Ok(match name.as_str() {
            "X25519" => {
                let public_key: Class<CryptoKey> = obj.get_required("public", "algorithm")?;
                let public_key = public_key.borrow();

                public_key.check_kind(KeyKind::Public).or_throw_dom(ctx)?;

                if !matches!(public_key.algorithm, KeyAlgorithm::X25519) {
                    return algorithm_invalid_access_error(ctx, &name);
                }

                DeriveAlgorithm::X25519 {
                    public_key: public_key.handle.clone(),
                }
            },
            "ECDH" => {
                let public_key: Class<CryptoKey> = obj.get_required("public", "algorithm")?;
                let public_key = public_key.borrow();

                public_key.check_kind(KeyKind::Public).or_throw_dom(ctx)?;

                if let KeyAlgorithm::Ec {
                    curve, algorithm, ..
                } = &public_key.algorithm
                {
                    DeriveAlgorithm::Ecdh {
                        curve: *curve,
                        ec_algorithm: algorithm.clone(),
                        public_key: public_key.handle.clone(),
                    }
                } else {
                    return algorithm_mismatch_error(ctx, &name);
                }
            },
            "HKDF" => DeriveAlgorithm::Derive(KeyDerivation::for_hkdf_object(ctx, obj)?),
            "PBKDF2" => DeriveAlgorithm::Derive(KeyDerivation::for_pbkf2_object(&ctx, obj)?),
            _ => return algorithm_not_supported_error(ctx),
        })
    }
}
