// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use llrt_utils::str_enum;
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    Ctx, Exception, Result, Value,
};

use super::key_algorithm::KeyAlgorithm;

#[derive(PartialEq)]
pub enum KeyKind {
    Secret,
    Private,
    Public,
}

str_enum!(KeyKind,Secret => "secret", Private => "private", Public => "public");

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct CryptoKey {
    pub kind: KeyKind,
    pub extractable: bool,
    pub algorithm: KeyAlgorithm,
    pub name: Box<str>,
    pub usages: Vec<String>,
    pub handle: Rc<[u8]>,
}

impl CryptoKey {
    pub fn new<N, H>(
        kind: KeyKind,
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
            kind,
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
    #[qjs(constructor)]
    fn constructor(ctx: Ctx<'_>) -> Result<Self> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    #[qjs(get, rename = "type")]
    pub fn get_type(&self) -> &str {
        self.kind.as_str()
    }

    #[qjs(get)]
    pub fn extractable(&self) -> bool {
        self.extractable
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(CryptoKey)
    }

    #[qjs(get)]
    pub fn algorithm<'js>(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.algorithm
            .as_object(&ctx, self.name.as_ref())
            .map(|a| a.into_value())
    }

    #[qjs(get)]
    pub fn usages(&self) -> Vec<String> {
        self.usages.clone()
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
