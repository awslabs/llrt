// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    function::{Constructor, Opt},
    Class, Ctx, Object, Result,
};

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
struct DOMException {
    message: String,
    name: String,
    stack: String,
}

#[rquickjs::methods]
impl DOMException {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'_>, message: Opt<String>, name: Opt<String>) -> Result<Self> {
        let error_ctor: Constructor = ctx.globals().get(PredefinedAtom::Error)?;
        let new: Object = error_ctor.construct((message.clone(),))?;

        let var_message = message.0.unwrap_or(String::from(""));
        let var_name = name.0.unwrap_or(String::from("Error"));

        Ok(Self {
            message: var_message,
            name: var_name,
            stack: new.get::<_, String>(PredefinedAtom::Stack)?,
        })
    }

    #[qjs(get)]
    fn message(&self) -> String {
        self.message.clone()
    }

    #[qjs(get)]
    fn name(&self) -> String {
        self.name.clone()
    }

    #[qjs(get)]
    fn stack(&self) -> String {
        self.stack.clone()
    }

    #[qjs(rename = PredefinedAtom::ToString)]
    pub fn to_string(&self) -> String {
        if self.message.is_empty() {
            return self.name.clone();
        }

        format!("{}: {}", &self.name, &self.message)
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    Class::<DOMException>::define(&globals)?;

    let dom_ex_proto = Class::<DOMException>::prototype(ctx.clone()).unwrap();
    let error_ctor: Object = globals.get(PredefinedAtom::Error)?;
    let error_proto = error_ctor.get_prototype();
    dom_ex_proto.set_prototype(error_proto.as_ref())?;

    Ok(())
}
