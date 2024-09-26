// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, IntoJsFunc},
    prelude::Func,
    Ctx, FromJs, Function, IntoAtom, IntoJs, Object, Result, Symbol, Undefined, Value,
};

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
