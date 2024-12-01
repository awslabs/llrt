// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use rquickjs::{class::Trace, Array, Ctx, JsLifetime, Result, Value};

#[rquickjs::class]
#[derive(Clone, Trace, rquickjs::JsLifetime)]
pub struct CryptoKey<'js> {
    type_name: String,
    extractable: bool,
    algorithm: Value<'js>,
    usages: Array<'js>,
    #[qjs(skip_trace)]
    handle: Rc<[u8]>,
}

#[rquickjs::methods]
impl<'js> CryptoKey<'js> {
    #[qjs(get, rename = "type")]
    pub fn get_type(&self) -> &str {
        self.type_name.as_str()
    }

    #[qjs(get)]
    pub fn extractable(&self) -> bool {
        self.extractable
    }

    #[qjs(get)]
    pub fn algorithm(&self) -> Value<'js> {
        self.algorithm.clone()
    }

    #[qjs(get)]
    pub fn usages(&self) -> Array<'js> {
        self.usages.clone()
    }
}

impl<'js> CryptoKey<'js> {
    pub fn new(
        _ctx: Ctx<'js>,
        type_name: String,
        extractable: bool,
        algorithm: Value<'js>,
        usages: Array<'js>,
        handle: &[u8],
    ) -> Result<Self> {
        Ok(Self {
            type_name,
            extractable,
            algorithm,
            usages,
            handle: handle.into(),
        })
    }
    pub fn get_handle(&self) -> &[u8] {
        &self.handle
    }
}
