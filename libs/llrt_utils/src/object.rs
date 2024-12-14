// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom, function::IntoJsFunc, prelude::Func, Array, Ctx, Error, Exception,
    FromJs, IntoAtom, IntoJs, Object, Result, Symbol, Undefined, Value,
};

use crate::primordials::{BasePrimordials, Primordial};

pub trait ObjectExt<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>>;
    fn get_required<K: AsRef<str>, V: FromJs<'js>>(
        &self,
        k: K,
        object_name: &'static str,
    ) -> Result<V>;
    fn into_object_or_throw(self, ctx: &Ctx<'js>, object_name: &'static str)
        -> Result<Object<'js>>;
}

impl<'js> ObjectExt<'js> for Object<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js> + Sized>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        self.get::<K, Option<V>>(k)
    }

    fn get_required<K: AsRef<str>, V: FromJs<'js>>(
        &self,
        k: K,
        object_name: &'static str,
    ) -> Result<V> {
        let k = k.as_ref();
        self.get::<&str, Option<V>>(k)?.ok_or_else(|| {
            Exception::throw_type(
                self.ctx(),
                &[object_name, " '", k, "' property required"].concat(),
            )
        })
    }

    fn into_object_or_throw(self, _: &Ctx<'js>, _: &'static str) -> Result<Object<'js>> {
        Ok(self)
    }
}

impl<'js> ObjectExt<'js> for Value<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>> {
        if let Some(obj) = self.as_object() {
            return obj.get_optional(k);
        }
        Ok(None)
    }

    fn get_required<K: AsRef<str>, V: FromJs<'js>>(
        &self,
        k: K,
        object_name: &'static str,
    ) -> Result<V> {
        self.as_object()
            .ok_or_else(|| not_a_object_error(self.ctx(), object_name))?
            .get_required(k, object_name)
    }

    fn into_object_or_throw(
        self,
        ctx: &Ctx<'js>,
        object_name: &'static str,
    ) -> Result<Object<'js>> {
        self.into_object()
            .ok_or_else(|| not_a_object_error(ctx, object_name))
    }
}

pub fn not_a_object_error(ctx: &Ctx<'_>, object_name: &str) -> Error {
    Exception::throw_type(ctx, &[object_name, " is not an object"].concat())
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
        BasePrimordials::get(ctx)?
            .constructor_proxy
            .construct::<_, Value>((self.target, self.options))
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
