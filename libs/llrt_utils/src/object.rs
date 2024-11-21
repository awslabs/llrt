// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, IntoJsFunc},
    prelude::Func,
    Array, Ctx, FromJs, IntoAtom, IntoJs, Object, Result, Symbol, Undefined, Value,
};

use crate::primordials::{BasePrimordials, Primordial};

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
    fn for_description(ctx: &Ctx<'js>, description: &str) -> Result<Symbol<'js>>;
}

impl<'js> CreateSymbol<'js> for Symbol<'js> {
    fn for_description(ctx: &Ctx<'js>, description: &str) -> Result<Symbol<'js>> {
        BasePrimordials::get(ctx)?
            .function_symbol_for
            .call((description,))
    }
}

pub struct Proxy<'js> {
    target: Value<'js>,
    options: Object<'js>,
}

impl<'js> IntoJs<'js> for Proxy<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let proxy_ctor = ctx.globals().get::<_, Constructor>(PredefinedAtom::Proxy)?;

        proxy_ctor.construct::<_, Value>((self.target, self.options))
    }
}

impl<'js> Proxy<'js> {
    pub fn new(ctx: Ctx<'js>) -> Result<Self> {
        let options = Object::new(ctx.clone())?;
        Ok(Self {
            target: Undefined.into_value(ctx),
            options,
        })
    }

    pub fn with_target(ctx: Ctx<'js>, target: Value<'js>) -> Result<Self> {
        let options = Object::new(ctx)?;
        Ok(Self { target, options })
    }

    pub fn setter<T, P>(&self, setter: Func<T, P>) -> Result<()>
    where
        T: IntoJsFunc<'js, P> + 'js,
    {
        self.options.set(PredefinedAtom::Setter, setter)?;
        Ok(())
    }

    pub fn getter<T, P>(&self, getter: Func<T, P>) -> Result<()>
    where
        T: IntoJsFunc<'js, P> + 'js,
    {
        self.options.set(PredefinedAtom::Getter, getter)?;
        Ok(())
    }
}

pub fn map_to_entries<'js, K, V, M>(ctx: &Ctx<'js>, map: M) -> Result<Array<'js>>
where
    M: IntoIterator<Item = (K, V)>,
    K: IntoJs<'js>,
    V: IntoJs<'js>,
{
    let array = Array::new(ctx.clone())?;
    for (idx, (key, value)) in map.into_iter().enumerate() {
        let entry = Array::new(ctx.clone())?;
        entry.set(0, key)?;
        entry.set(1, value)?;
        array.set(idx, entry)?;
    }

    Ok(array)
}
