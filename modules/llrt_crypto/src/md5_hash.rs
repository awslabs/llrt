// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::bytes::{bytes_to_typed_array, ObjectBytes};
use rquickjs::{function::Opt, prelude::This, Class, Ctx, Result, Value};

use crate::{provider::{CryptoProvider, SimpleDigest}, sha_hash::ShaAlgorithm};
use super::{encoded_bytes, CRYPTO_PROVIDER};

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct Md5 {
    #[qjs(skip_trace)]
    hasher: <crate::provider::DefaultProvider as CryptoProvider>::Digest,
}

#[rquickjs::methods]
impl Md5 {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {
            hasher: CRYPTO_PROVIDER.digest(ShaAlgorithm::MD5),
        }
    }

    #[qjs(rename = "digest")]
    fn md5_digest<'js>(&mut self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let digest = std::mem::replace(&mut self.hasher, CRYPTO_PROVIDER.digest(ShaAlgorithm::MD5)).finalize();

        match encoding.0 {
            Some(encoding) => encoded_bytes(ctx, &digest, &encoding),
            None => bytes_to_typed_array(ctx, &digest),
        }
    }

    #[qjs(rename = "update")]
    fn md5_update<'js>(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        bytes: ObjectBytes<'js>,
    ) -> Result<Class<'js, Self>> {
        this.0.borrow_mut().hasher.update(bytes.as_bytes(&ctx)?);
        Ok(this.0)
    }
}
