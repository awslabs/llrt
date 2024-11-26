// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{class::Trace, Array, Ctx, JsLifetime, Object, Result};

#[rquickjs::class]
#[derive(Clone, Trace, rquickjs::JsLifetime)]
pub struct CryptoKey<'js> {
    type_name: String,
    extractable: bool,
    #[qjs(skip_trace)]
    algorithm: Object<'js>,
    usages: Array<'js>,
    handle: Vec<u8>,
}

#[rquickjs::methods]
impl<'js> CryptoKey<'js> {
    #[qjs(constructor)]
    pub fn new(
        _ctx: Ctx<'js>,
        type_name: String,
        extractable: bool,
        algorithm: Object<'js>,
        usages: Array<'js>,
        handle: Vec<u8>,
    ) -> Result<Self> {
        Ok(Self {
            type_name,
            extractable,
            algorithm,
            usages,
            handle,
        })
    }

    #[qjs(get, rename = "type")]
    pub fn get_type(&self) -> &str {
        self.type_name.as_str()
    }

    #[qjs(get)]
    pub fn extractable(&self) -> bool {
        self.extractable
    }

    #[qjs(get)]
    pub fn algorithm(&self) -> Object<'js> {
        self.algorithm.clone()
    }

    #[qjs(get)]
    pub fn usages(&self) -> Array<'js> {
        self.usages.clone()
    }
}
