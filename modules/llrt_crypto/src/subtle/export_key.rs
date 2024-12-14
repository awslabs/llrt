use llrt_utils::result::ResultExt;
use ring::signature::{EcdsaKeyPair, Ed25519KeyPair, KeyPair};
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{ArrayBuffer, Class, Ctx, Exception, Result};

use crate::{subtle::CryptoKey, SYSTEM_RANDOM};

use super::{algorithm_not_supported_error, key_algorithm::KeyAlgorithm};

pub async fn subtle_export_key<'js>(
    ctx: Ctx<'js>,
    format: String,
    key: Class<'js, CryptoKey>,
) -> Result<ArrayBuffer<'js>> {
    let key = key.borrow();

    if !key.extractable {
        return Err(Exception::throw_type(
            &ctx,
            "The CryptoKey is non extractable",
        ));
    };

    //TODO add more formats
    if format != "raw" {
        return Err(Exception::throw_type(
            &ctx,
            &["Format '", &format, "' is not implemented"].concat(),
        ));
    }
    export_raw(ctx, &key)
}

fn export_raw<'js>(ctx: Ctx<'js>, key: &CryptoKey) -> Result<ArrayBuffer<'js>> {
    let handle = key.handle.as_ref();
    let bytes: Vec<u8> = match &key.algorithm {
        KeyAlgorithm::Aes { .. } | KeyAlgorithm::Hmac { .. } => handle.into(),
        KeyAlgorithm::Ec { curve } => {
            let alg = curve.as_signing_algorithm();
            let rng = &(*SYSTEM_RANDOM);
            let key_pair = EcdsaKeyPair::from_pkcs8(alg, &key.handle, rng).or_throw(&ctx)?;
            key_pair.public_key().as_ref().into()
        },
        KeyAlgorithm::Ed25519 => {
            let key_pair = Ed25519KeyPair::from_pkcs8(handle).or_throw(&ctx)?;
            key_pair.public_key().as_ref().into()
        },
        KeyAlgorithm::Rsa { .. } => {
            let key_pair = ring::signature::RsaKeyPair::from_pkcs8(handle).or_throw(&ctx)?;
            key_pair.public_key().as_ref().into()
        },
        _ => return algorithm_not_supported_error(&ctx),
    };

    ArrayBuffer::new(ctx, bytes)
}
