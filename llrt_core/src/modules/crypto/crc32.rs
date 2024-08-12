// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::hash::Hasher;

use crc32c::Crc32cHasher;
use llrt_utils::bytes::ObjectBytes;
use rquickjs::{prelude::This, Class, Ctx, Result, Value};

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Crc32c {
    #[qjs(skip_trace)]
    hasher: crc32c::Crc32cHasher,
}

#[rquickjs::methods]
impl Crc32c {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {
            hasher: Crc32cHasher::default(),
        }
    }

    #[qjs(rename = "digest")]
    fn crc32c_digest(&self, _ctx: Ctx<'_>) -> u64 {
        self.hasher.finish()
    }

    #[qjs(rename = "update")]
    fn crc32c_update<'js>(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        value: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = ObjectBytes::from(&ctx, &value)?;
        this.0.borrow_mut().hasher.write(bytes.as_bytes());
        Ok(this.0)
    }
}

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct Crc32 {
    #[qjs(skip_trace)]
    hasher: crc32fast::Hasher,
}

#[rquickjs::methods]
impl Crc32 {
    #[qjs(constructor)]
    fn new() -> Self {
        Self {
            hasher: crc32fast::Hasher::new(),
        }
    }

    #[qjs(rename = "digest")]
    fn crc32_digest(&self, _ctx: Ctx<'_>) -> u64 {
        self.hasher.finish()
    }

    #[qjs(rename = "update")]
    fn crc32_update<'js>(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        value: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        let bytes = ObjectBytes::from(&ctx, &value)?;
        this.0.borrow_mut().hasher.write(bytes.as_bytes());
        Ok(this.0)
    }
}
