// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use llrt_utils::str_enum;
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    Ctx, Exception, IntoJs, Object, Result, Value,
};

use super::key_algorithm::KeyAlgorithm;

#[derive(PartialEq, Clone, Copy)]
pub enum KeyKind {
    Secret,
    Private,
    Public,
}

str_enum!(KeyKind,Secret => "secret", Private => "private", Public => "public");

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct CryptoKey<'js> {
    pub kind: KeyKind,
    pub extractable: bool,
    pub algorithm: KeyAlgorithm,
    pub name: Box<str>,
    pub usages: Vec<String>,
    pub handle: Rc<[u8]>,
    algorithm_cache: Option<Object<'js>>,
    usages_cache: Option<Value<'js>>,
}

impl<'js> CryptoKey<'js> {
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
            algorithm_cache: None,
            usages_cache: None,
        }
    }
}

impl<'js> Trace<'js> for CryptoKey<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Some(cached) = &self.algorithm_cache {
            cached.trace(tracer);
        }
        if let Some(cached) = &self.usages_cache {
            cached.trace(tracer);
        }
    }
}

#[rquickjs::methods]
impl<'js> CryptoKey<'js> {
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

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(CryptoKey)
    }

    #[qjs(get)]
    pub fn algorithm(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(cached) = &self.algorithm_cache {
            return Ok(cached.clone().into_value());
        }
        let obj = self.algorithm.as_object(&ctx, self.name.as_ref())?;
        self.algorithm_cache = Some(obj.clone());
        Ok(obj.into_value())
    }

    #[qjs(get)]
    pub fn usages(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(cached) = &self.usages_cache {
            return Ok(cached.clone());
        }
        let arr = self.usages.clone().into_js(&ctx)?;
        self.usages_cache = Some(arr.clone());
        Ok(arr)
    }
}

impl<'js> CryptoKey<'js> {
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
