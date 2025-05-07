// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::HashMap, rc::Rc};

use llrt_utils::str_enum;
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    Ctx, Exception, IntoJs, JsLifetime, Result, Value,
};

use super::key_algorithm::KeyAlgorithm;

#[derive(PartialEq)]
pub enum KeyKind {
    Secret,
    Private,
    Public,
}

str_enum!(KeyKind,Secret => "secret", Private => "private", Public => "public");

#[derive(Eq, Hash, PartialEq)]
pub enum CacheKey {
    Usages,
    Algorithm,
}

#[rquickjs::class]
// #[derive(rquickjs::JsLifetime)]
pub struct CryptoKey<'js> {
    pub kind: KeyKind,
    pub extractable: bool,
    pub algorithm: KeyAlgorithm,
    pub name: Box<str>,
    pub usages: Vec<String>,
    pub handle: Rc<[u8]>,
    pub cache: HashMap<CacheKey, Value<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for CryptoKey<'js> {
    type Changed<'to> = CryptoKey<'to>;
}

impl<'js> Trace<'js> for CryptoKey<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        for value in self.cache.values() {
            value.trace(tracer);
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

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(CryptoKey)
    }

    #[qjs(get)]
    pub fn algorithm(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let cache_key = CacheKey::Algorithm;
        if let Some(value) = self.cache.get(&cache_key) {
            return Ok(value.clone());
        }
        let obj = self
            .algorithm
            .as_object(&ctx, self.name.as_ref())?
            .into_value();
        self.cache.insert(cache_key, obj.clone());
        Ok(obj)
    }

    #[qjs(get)]
    pub fn usages(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let cache_key = CacheKey::Usages;
        if let Some(value) = self.cache.get(&cache_key) {
            return Ok(value.clone());
        }
        let usages = self.usages.clone().into_js(&ctx)?;
        self.cache.insert(cache_key, usages.clone());
        Ok(usages)
    }
}

impl CryptoKey<'_> {
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
            cache: Default::default(),
            name: name.into(),
            usages,
            handle: handle.into(),
        }
    }

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
