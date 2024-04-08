// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{function::Opt, Class, Ctx, Result};
use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug, Default)]
#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct DOMException {
    message: String,
    name: String,
}

impl Display for DOMException {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        todo!()
        /*
        if self.message.is_empty() {
            return self.name.clone();
        }

        format!("{}: {}", &self.name, &self.message)
        */
    }
}

impl Error for DOMException {}

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
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<DOMException>::define(&globals)?;

    Ok(())
}
