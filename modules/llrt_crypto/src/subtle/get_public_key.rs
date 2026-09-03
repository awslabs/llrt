// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;

use llrt_exceptions::DOMException;
use rquickjs::{Array, Class, Ctx, Result};

use crate::{
    provider::{modern, CryptoProvider, RsaJwkImport},
    CRYPTO_PROVIDER,
};

use super::{
    crypto_key::{CryptoKey, KeyKind},
    key_algorithm::KeyAlgorithm,
    util::ResultDomExt,
};

pub fn subtle_get_public_key<'js>(
    ctx: Ctx<'js>,
    key: Class<'js, CryptoKey<'js>>,
    usages: Array<'js>,
) -> impl Future<Output = Result<Class<'js, CryptoKey<'js>>>> + 'js {
    let prepared = {
        let key = key.borrow();
        if !key.algorithm.supports_get_public_key() {
            Err(DOMException::not_supported_error(
                &ctx,
                "This algorithm cannot derive a public key",
            ))
        } else if key.kind != KeyKind::Private {
            Err(DOMException::invalid_access_error(
                &ctx,
                "getPublicKey requires a private key",
            ))
        } else {
            key.algorithm
                .validate_public_usages(&ctx, &key.name, &usages)
        }
    };

    async move {
        let usages = prepared?;
        let (name, algorithm, public_key) = {
            let key = key.borrow();
            let public_key = match &key.algorithm {
                KeyAlgorithm::Ec { curve, .. } => CRYPTO_PROVIDER
                    .export_ec_public_key_sec1(&key.handle, *curve, true)
                    .or_throw_dom(&ctx)?,
                KeyAlgorithm::Ed25519 => {
                    CRYPTO_PROVIDER
                        .export_okp_jwk(&key.handle, true, true)
                        .or_throw_dom(&ctx)?
                        .x
                },
                KeyAlgorithm::X25519 => {
                    CRYPTO_PROVIDER
                        .export_okp_jwk(&key.handle, true, false)
                        .or_throw_dom(&ctx)?
                        .x
                },
                KeyAlgorithm::Rsa { .. } => {
                    let jwk = CRYPTO_PROVIDER
                        .export_rsa_jwk(&key.handle, true)
                        .or_throw_dom(&ctx)?;
                    CRYPTO_PROVIDER
                        .import_rsa_jwk(RsaJwkImport {
                            n: &jwk.n,
                            e: &jwk.e,
                            d: None,
                            p: None,
                            q: None,
                            dp: None,
                            dq: None,
                            qi: None,
                        })
                        .or_throw_dom(&ctx)?
                        .key_data
                },
                KeyAlgorithm::MlDsa(variant) => {
                    modern::ml_dsa_public_key(*variant, &key.handle).or_throw_dom(&ctx)?
                },
                KeyAlgorithm::MlKem(variant) => {
                    modern::ml_kem_public_key(*variant, &key.handle).or_throw_dom(&ctx)?
                },
                KeyAlgorithm::HybridKem(variant) => {
                    modern::hybrid_kem_public_key(*variant, &key.handle).or_throw_dom(&ctx)?
                },
                _ => unreachable!(),
            };
            (key.name.clone(), key.algorithm.clone(), public_key)
        };

        Class::instance(
            ctx,
            CryptoKey::new(KeyKind::Public, name, true, algorithm, usages, public_key),
        )
    }
}
