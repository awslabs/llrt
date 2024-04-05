// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use rquickjs::{atom::PredefinedAtom, function::Opt, Class, Ctx, Result};

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
