// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::libs::utils::primordials::{BasePrimordials, Primordial};
use rquickjs::{
    atom::PredefinedAtom, function::This, object::Accessor, Array, Ctx, Function, Object, Result,
    Value,
};

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    let primordials = BasePrimordials::get(ctx)?;
    let custom_inspect_symbol = primordials.symbol_custom_inspect.clone();

    // Map
    primordials
        .prototype_map
        .prop(custom_inspect_symbol.clone(), Accessor::from(map_inspect))?;

    // Set
    primordials
        .prototype_set
        .prop(custom_inspect_symbol.clone(), Accessor::from(set_inspect))?;

    // DataView
    let dataview_proto: Object = globals
        .get::<_, Function>(PredefinedAtom::DataView)?
        .get(PredefinedAtom::Prototype)?;
    dataview_proto.prop(
        custom_inspect_symbol.clone(),
        Accessor::from(dataview_inspect),
    )?;

    // ArrayBuffer
    let arraybuffer_proto: Object = globals
        .get::<_, Function>(PredefinedAtom::ArrayBuffer)?
        .get(PredefinedAtom::Prototype)?;
    arraybuffer_proto.prop(custom_inspect_symbol, Accessor::from(arraybuffer_inspect))?;

    Ok(())
}

fn map_inspect<'js>(ctx: Ctx<'js>, this: This<Object<'js>>) -> Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;

    let size: usize = this.get("size")?;
    obj.set("size", size)?;

    if size > 0 {
        let entries_fn: Function = this.get("entries")?;
        let iterator: Object = entries_fn.call((This(this.0.clone()),))?;
        let next_fn: Function = iterator.get(PredefinedAtom::Next)?;
        let entries = Array::new(ctx)?;

        for i in 0..size.min(100) {
            let next_result: Object = next_fn.call((This(iterator.clone()),))?;
            let done: bool = next_result.get(PredefinedAtom::Done)?;
            if done {
                break;
            }
            let entry: Array = next_result.get(PredefinedAtom::Value)?;
            entries.set(i, entry)?;
        }
        obj.set("entries", entries)?;
    }
    Ok(obj)
}

fn set_inspect<'js>(ctx: Ctx<'js>, this: This<Object<'js>>) -> Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;
    let size: usize = this.get("size")?;
    obj.set("size", size)?;

    if size > 0 {
        let values_fn: Function = this.get("values")?;
        let iterator: Object = values_fn.call((This(this.0.clone()),))?;
        let next_fn: Function = iterator.get("next")?;
        let values = Array::new(ctx)?;

        for i in 0..size.min(100) {
            let next_result: Object = next_fn.call((This(iterator.clone()),))?;
            let done: bool = next_result.get("done")?;
            if done {
                break;
            }
            let value: Value = next_result.get("value")?;
            values.set(i, value)?;
        }
        obj.set("values", values)?;
    }
    Ok(obj)
}

fn dataview_inspect<'js>(ctx: Ctx<'js>, this: This<Object<'js>>) -> Result<Object<'js>> {
    let obj = Object::new(ctx)?;
    obj.set("byteLength", this.get::<_, usize>("byteLength")?)?;
    obj.set("byteOffset", this.get::<_, usize>("byteOffset")?)?;
    obj.set("buffer", this.get::<_, Object>("buffer")?)?;
    Ok(obj)
}

fn arraybuffer_inspect<'js>(ctx: Ctx<'js>, this: This<Object<'js>>) -> Result<Object<'js>> {
    let primordials = BasePrimordials::get(&ctx)?;
    let obj = Object::new(ctx.clone())?;

    let byte_length: usize = this.get("byteLength")?;
    let uint8_view: Object = primordials
        .constructor_uint8array
        .construct((this.0.clone(),))?;

    let mut bytes = String::from("<");
    for i in 0..byte_length.min(8) {
        if i > 0 {
            bytes.push(' ');
        }
        let byte: u8 = uint8_view.get(i as u32)?;
        bytes.push_str(&format!("{:02x}", byte));
    }
    bytes.push('>');

    obj.set("uint8Contents", bytes)?;
    obj.set("byteLength", byte_length)?;
    Ok(obj)
}
