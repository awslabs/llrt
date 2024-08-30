// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{atom::PredefinedAtom, FromJs, Function, IntoAtom, Object, Result, Symbol, Value};

pub trait ObjectExt<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>>;
}

impl<'js> ObjectExt<'js> for Object<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js> + Sized>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        self.get::<K, Option<V>>(k)
    }
}

impl<'js> ObjectExt<'js> for Value<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>> {
        if let Some(obj) = self.as_object() {
            return obj.get_optional(k);
        }
        Ok(None)
    }
}

pub trait CreateSymbol<'js> {
    fn for_description(globals: &Object<'js>, description: &'static str) -> Result<Symbol<'js>>;
}

impl<'js> CreateSymbol<'js> for Symbol<'js> {
    fn for_description(globals: &Object<'js>, description: &'static str) -> Result<Symbol<'js>> {
        let symbol_function: Function = globals.get(PredefinedAtom::Symbol)?;
        let for_function: Function = symbol_function.get(PredefinedAtom::For)?;
        for_function.call((description,))
    }
}
