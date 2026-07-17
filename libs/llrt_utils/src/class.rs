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

/// Which view an iterator yields: `keys()`, `values()`, or `entries()`.
#[derive(Clone, Copy)]
pub enum IterKind {
    Keys,
    Values,
    Entries,
}

/// Wrap an entry into a `{ value, done }` iterator result. `None` means done.
pub fn iterator_result<'js>(
    ctx: &Ctx<'js>,
    kind: IterKind,
    entry: Option<(Value<'js>, Value<'js>)>,
) -> Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    match entry {
        Some((key, value)) => {
            obj.set(PredefinedAtom::Done, false)?;
            match kind {
                IterKind::Keys => obj.set(PredefinedAtom::Value, key)?,
                IterKind::Values => obj.set(PredefinedAtom::Value, value)?,
                IterKind::Entries => {
                    let entry = Array::new(ctx.clone())?;
                    entry.set(0, key)?;
                    entry.set(1, value)?;
                    obj.set(PredefinedAtom::Value, entry)?;
                },
            }
        },
        None => obj.set(PredefinedAtom::Done, true)?,
    }
    Ok(obj)
}

/// Create a WebIDL iterator instance, wiring its prototype the first time:
/// the prototype inherits `%IteratorPrototype%` (so it's tagged
/// `[object Iterator]`) and `next` becomes enumerable. Idempotent — later
/// calls skip the setup — so callers just build iterators and never register
/// anything separately.
pub fn live_iterator<'js, C>(ctx: &Ctx<'js>, iter: C) -> Result<Class<'js, C>>
where
    C: JsClass<'js> + 'js,
{
    let instance = Class::<C>::instance(ctx.clone(), iter)?;
    if let Some(proto) = Class::<C>::prototype(ctx)? {
        let iterator_proto = &BasePrimordials::get(ctx)?.prototype_iterator;
        if proto.get_prototype().as_ref() != Some(iterator_proto) {
            proto.set_prototype(Some(iterator_proto))?;
            let next_fn: Function = proto.get(PredefinedAtom::Next)?;
            proto.prop(
                PredefinedAtom::Next,
                Property::from(next_fn)
                    .writable()
                    .enumerable()
                    .configurable(),
            )?;
        }
    }
    Ok(instance)
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
