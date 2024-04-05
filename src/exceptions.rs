// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    atom::PredefinedAtom,
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Result,
};

use crate::module::export_default;

#[derive(Clone, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct DOMException {
    message: String,
    name: String,
}

#[rquickjs::methods]
impl<'js> DOMException {
    #[qjs(constructor)]
    pub fn new(_ctx: Ctx<'js>, message: Opt<String>, name: Opt<String>) -> Result<Self> {
        Ok(Self {
            message: message.0.unwrap_or(String::from("")),
            name: name.0.unwrap_or(String::from("Error")),
        })
    }

    #[qjs(get, rename = "message")]
    fn get_message(&self) -> String {
        self.message.clone()
    }

    #[qjs(get, rename = "name")]
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    #[qjs(rename = PredefinedAtom::ToString)]
    pub fn to_string(&self) -> String {
        let mut message = String::new();

        if !self.message.is_empty() {
            message = format!(": {}", &self.message)
        }

        format!("{}{}", &self.name, message)
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<DOMException>::define(&globals)?;

    Ok(())
}

pub struct ExceptionsModule;

impl ModuleDef for ExceptionsModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        Class::<DOMException>::register(ctx)?;

        export_default(ctx, exports, |default| {
            Class::<DOMException>::define(default)?;

            Ok(())
        })
    }
}
