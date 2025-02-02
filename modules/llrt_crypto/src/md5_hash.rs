// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use md5::{Digest as Md5Digest, Md5 as MdHasher};

use llrt_utils::bytes::{bytes_to_typed_array, ObjectBytes};
use rquickjs::{function::Opt, prelude::This, Class, Ctx, Result, Value};

use super::encoded_bytes;

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct Md5 {
    #[qjs(skip_trace)]
    hasher: MdHasher,
}

#[rquickjs::methods]
impl Md5 {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {
            hasher: MdHasher::new(),
        }
    }

    #[qjs(rename = "digest")]
    fn md5_digest<'js>(&self, ctx: Ctx<'js>, encoding: Opt<String>) -> Result<Value<'js>> {
        let digest = self.hasher.clone().finalize();
        let bytes: &[u8] = digest.as_ref();

        match encoding.0 {
            Some(encoding) => encoded_bytes(ctx, bytes, &encoding),
            None => bytes_to_typed_array(ctx, bytes),
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
