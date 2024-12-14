// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use rquickjs::{
    class::{Trace, Tracer},
    Ctx, Result, Value,
};

use super::key_algorithm::KeyAlgorithm;

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct CryptoKey {
    type_name: &'static str,
    pub extractable: bool,
    pub algorithm: KeyAlgorithm,
    pub name: Box<str>,
    usages: Vec<String>,
    pub handle: Rc<[u8]>,
}

impl CryptoKey {
    pub fn new<N, H>(
        type_name: &'static str,
        name: N,
        extractable: bool,
        algorithm: KeyAlgorithm,
        usages: Vec<String>,
        handle: H,
    ) -> Self
    where
        N: Into<Box<str>>,
        H: Into<Rc<[u8]>>,
    {
        Self {
            type_name,
            extractable,
            algorithm,
            name: name.into(),
            usages,
            handle: handle.into(),
        }
    }
}

impl<'js> Trace<'js> for CryptoKey {
    fn trace<'a>(&self, _: Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
impl CryptoKey {
    #[qjs(get, rename = "type")]
    pub fn get_type(&self) -> &str {
        self.type_name
    }

    #[qjs(get)]
    pub fn extractable(&self) -> bool {
        self.extractable
    }

    #[qjs(get)]
    pub fn algorithm<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.algorithm
            .as_object(&ctx, self.name.as_ref())
            .map(|a| a.into_value())
    }

    #[qjs(get)]
    pub fn usages(&self) -> Vec<String> {
        self.usages.iter().map(|u| u.to_string()).collect()
    }
}

impl CryptoKey {
    pub fn check_validity(&self, usage: &str) -> std::result::Result<(), String> {
        for key in self.usages.iter() {
            if key == usage {
                return Ok(());
            }
        }
        Err([
            "CryptoKey with '",
            self.name.as_ref(),
            "', doesn't support '",
            usage,
            "'",
        ]
        .concat())
    }
}
