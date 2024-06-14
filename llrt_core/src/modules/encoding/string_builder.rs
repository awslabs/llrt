// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{function::Opt, prelude::This, Class, Ctx, Result, Value};

#[rquickjs::class]
#[derive(rquickjs::class::Trace, Debug)]
pub struct StringBuilder {
    #[qjs(skip_trace)]
    value: String,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl StringBuilder {
    #[qjs(constructor)]
    fn new(capacity: Opt<usize>) -> Self {
        Self {
            value: String::with_capacity(capacity.0.unwrap_or(256)),
        }
    }

    fn append<'js>(
        this: This<Class<'js, Self>>,
        _ctx: Ctx<'js>,
        value: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        if value.is_string() {
            let string: String = value.get()?;
            this.borrow_mut().value.push_str(&string);
        } else if value.is_number() {
            let number: f64 = value.get()?;
            this.borrow_mut().value.push_str(&number.to_string());
        } else if value.is_bool() {
            let boolean: bool = value.get()?;
            this.0
                .borrow_mut()
                .value
                .push_str(if boolean { "true" } else { "false" });
        }
        Ok(this.0)
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_string(&mut self) -> String {
        self.value.clone()
    }
}
