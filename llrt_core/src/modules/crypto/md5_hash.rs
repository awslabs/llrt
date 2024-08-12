use llrt_utils::bytes::ObjectBytes;
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use md5::{Digest as Md5Digest, Md5 as MdHasher};

use rquickjs::{function::Opt, prelude::This, Class, Ctx, Result, Value};

use crate::utils::object::bytes_to_typed_array;

use super::encoded_bytes;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
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
        value: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        let mut bytes = ObjectBytes::from(&ctx, &value)?;
        this.0.borrow_mut().hasher.update(bytes.get_bytes());
        Ok(this.0)
    }
}
