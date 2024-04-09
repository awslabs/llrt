// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    function::Constructor,
    prelude::{Func, This},
    Ctx, IntoJs, Object, Result, Value,
};

pub struct DOMException {
    message: String,
    name: String,
    stack: Result<String>,
}

impl<'js> IntoJs<'js> for DOMException {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        //TODO: It seems that the parameters are not being pulled in properly from the JavaScript side.
        let message = self.message;
        let name = self.name;
        let val_message = message.into_js(ctx)?;
        let val_name = name.into_js(ctx)?;
        let constructor: Constructor = ctx.globals().get(stringify!(DOMException))?;
        constructor.construct((val_message, val_name))
    }
}

//impl<'js> DOMException {}

fn message<'js>(_this: This<Object<'js>>, _ctx: Ctx<'js>) -> Result<String> {
    // TODO: I don't know how to get the message property out of this object.
    Ok("".to_string())
}

fn name<'js>(_this: This<Object<'js>>, _ctx: Ctx<'js>) -> Result<String> {
    // TODO: I don't know how to get the name property out of this object.
    Ok("".to_string())
}

fn to_string(_this: This<Object<'_>>, _ctx: Ctx) -> Result<String> {
    // TODO: I don't know how to get the message and name property out of this object.
    Ok("".to_string())
}

fn set_prototype<'js>(ctx: &Ctx<'js>, constructor: Object<'js>) -> Result<()> {
    let prototype: &Object = &constructor.get(PredefinedAtom::Prototype)?;
    // TODO: It accesses struct directly with or without the next 2 lines.
    prototype.set(PredefinedAtom::Getter, Func::from(message))?;
    prototype.set(PredefinedAtom::Getter, Func::from(name))?;

    prototype.set(PredefinedAtom::ToString, Func::from(to_string))?;

    ctx.globals().set(stringify!(DOMException), constructor)?;

    Ok(())
}

pub fn init<'js>(ctx: &Ctx<'js>) -> Result<()> {
    let dom_exception = ctx.eval::<Object<'js>, &str>(&format!(
        "class {0} extends Error {{}}\n{0}",
        stringify!(DOMException)
    ))?;

    // TODO: Trying to register with globalThis, but IntoJs conflicts with #[rquickjs::class].
    //let globals = ctx.globals();
    //Class::<DOMException>::define(&globals)?;

    set_prototype(ctx, dom_exception)
}
