use llrt_utils::primordials::{BasePrimordials, Primordial};
// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{atom::PredefinedAtom, function::Opt, Class, Ctx, Object, Result};

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
pub struct DOMException {
    message: String,
    name: String,
    stack: String,
}

#[rquickjs::methods]
impl DOMException {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'_>, message: Opt<String>, name: Opt<String>) -> Result<Self> {
        let primordials = BasePrimordials::get(&ctx)?;

        let new: Object = primordials
            .constructor_error
            .construct((message.clone(),))?;

        let message = message.0.unwrap_or(String::from(""));
        let name = name.0.unwrap_or(String::from("Error"));

        Ok(Self {
            message,
            name,
            stack: new.get::<_, String>(PredefinedAtom::Stack)?,
        })
    }

    #[qjs(get)]
    fn message(&self) -> String {
        self.message.clone()
    }

    #[qjs(get)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[qjs(get)]
    fn stack(&self) -> String {
        self.stack.clone()
    }

    #[allow(clippy::inherent_to_string)]
    #[qjs(rename = PredefinedAtom::ToString)]
    pub fn to_string(&self) -> String {
        if self.message.is_empty() {
            return self.name.clone();
        }

        [self.name.as_str(), self.message.as_str()].join(": ")
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    Class::<DOMException>::define(&globals)?;

    let dom_ex_proto = Class::<DOMException>::prototype(ctx)?.unwrap();
    let error_prototype = &BasePrimordials::get(ctx)?.prototype_error;
    dom_ex_proto.set_prototype(Some(error_prototype))?;

    Ok(())
}
