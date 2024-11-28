// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{class::Trace, Array, Ctx, JsLifetime, Result, Value};

#[rquickjs::class]
#[derive(Clone, Trace, rquickjs::JsLifetime)]
pub struct CryptoKey<'js> {
    type_name: String,
    extractable: bool,
    algorithm: Value<'js>,
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
        algorithm: Value<'js>,
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
    pub fn algorithm(&self) -> Value<'js> {
        self.algorithm.clone()
    }

    #[qjs(get)]
    pub fn usages(&self) -> Array<'js> {
        self.usages.clone()
    }
}

#[rquickjs::class]
#[derive(Clone, Trace, rquickjs::JsLifetime)]
pub struct CryptoKeyPair<'js> {
    private_key: CryptoKey<'js>,
    public_key: CryptoKey<'js>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> CryptoKeyPair<'js> {
    #[qjs(get)]
    pub fn private_key(&self) -> CryptoKey<'js> {
        self.private_key.clone()
    }

    #[qjs(get)]
    pub fn public_key(&self) -> CryptoKey<'js> {
        self.public_key.clone()
    }
}

impl<'js> CryptoKeyPair<'js> {
    pub fn new(
        _ctx: Ctx<'js>,
        private_key: CryptoKey<'js>,
        public_key: CryptoKey<'js>,
    ) -> Result<Self> {
        Ok(Self {
            private_key,
            public_key,
        })
    }
}
