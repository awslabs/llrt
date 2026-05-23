// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom, class::JsClass, object::Accessor, object::Property, prelude::This, Array,
    Class, Ctx, Function, Object, Result, Symbol, Value,
};

use super::{
    object::ObjectExt,
    primordials::{BasePrimordials, Primordial},
    result::OptionExt,
};

pub static CUSTOM_INSPECT_SYMBOL_DESCRIPTION: &str = "llrt.inspect.custom";

pub trait IteratorDef<'js>
where
    Self: 'js + JsClass<'js> + Sized,
{
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>>;

    fn js_iterator(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let value = self.js_entries(ctx)?;
        let obj = value.as_object();
        let values_fn: Function = obj.get(PredefinedAtom::Values)?;
        values_fn.call((This(value),))
    }
}

pub fn get_class_name(value: &Value) -> Result<Option<String>> {
    value
        .get_optional::<_, Object>(PredefinedAtom::Constructor)?
        .and_then_ok(|ctor| ctor.get_optional::<_, String>(PredefinedAtom::Name))
}

#[inline(always)]
pub fn get_class<'js, C>(provided: &Value<'js>) -> Result<Option<Class<'js, C>>>
where
    C: JsClass<'js>,
{
    if provided
        .as_object()
        .map(|p| p.instance_of::<C>())
        .unwrap_or_default()
    {
        return Ok(Some(Class::<C>::from_value(provided)?));
    }
    Ok(None)
}

pub trait CustomInspectExtension<'js> {
    fn define_with_custom_inspect(globals: &Object<'js>) -> Result<()>;
}

pub trait CustomInspect<'js>
where
    Self: JsClass<'js>,
{
    fn custom_inspect(&self, ctx: Ctx<'js>) -> Result<Object<'js>>;
}

impl<'js, C> CustomInspectExtension<'js> for Class<'js, C>
where
    C: JsClass<'js> + CustomInspect<'js> + 'js,
{
    fn define_with_custom_inspect(globals: &Object<'js>) -> Result<()> {
        Self::define(globals)?;
        let custom_inspect_symbol =
            Symbol::new_global(globals.ctx().clone(), CUSTOM_INSPECT_SYMBOL_DESCRIPTION)?;
        if let Some(proto) = Class::<C>::prototype(globals.ctx())? {
            proto.prop(
                custom_inspect_symbol,
                Accessor::from(|this: This<Class<'js, C>>, ctx| this.borrow().custom_inspect(ctx)),
            )?;
        }
        Ok(())
    }
}

/// Register a class as a WebIDL pair iterator: registers it on the globals,
/// removes the constructor (it's not exposed to JS), wires its prototype to
/// inherit from `%IteratorPrototype%` (so it's iterable and stringifies as
/// `[object Iterator]`), and re-defines `next` as enumerable per WebIDL.
///
/// The class must declare a `next(&mut self, ctx) -> Result<Object>` method
/// via `#[rquickjs::methods]`.
pub trait WebIdlIteratorExtension<'js> {
    fn define_as_webidl_iterator(globals: &Object<'js>, name: &str) -> Result<()>;
}

impl<'js, C> WebIdlIteratorExtension<'js> for Class<'js, C>
where
    C: JsClass<'js> + 'js,
{
    fn define_as_webidl_iterator(globals: &Object<'js>, name: &str) -> Result<()> {
        let ctx = globals.ctx();
        Self::define(globals)?;
        // Iterator class is not exposed to JS.
        globals.remove(name)?;
        if let Some(proto) = Class::<C>::prototype(ctx)? {
            let iterator_proto = BasePrimordials::get(ctx)?.prototype_iterator.clone();
            proto.set_prototype(Some(&iterator_proto))?;
            let next_fn: Function = proto.get("next")?;
            proto.prop(
                "next",
                Property::from(next_fn)
                    .writable()
                    .enumerable()
                    .configurable(),
            )?;
        }
        Ok(())
    }
}
