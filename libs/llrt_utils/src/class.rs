// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom, class::JsClass, object::Accessor, prelude::This, Array, Class, Ctx,
    Function, Object, Result, Value,
};

use crate::primordials::{BasePrimordials, Primordial};

use super::{object::ObjectExt, result::OptionExt};

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
        let custom_inspect_symbol = BasePrimordials::get(globals.ctx())?
            .symbol_custom_inspect
            .clone();
        if let Some(proto) = Class::<C>::prototype(globals.ctx())? {
            proto.prop(
                custom_inspect_symbol,
                Accessor::from(|this: This<Class<'js, C>>, ctx| this.borrow().custom_inspect(ctx)),
            )?;
        }
        Ok(())
    }
}
