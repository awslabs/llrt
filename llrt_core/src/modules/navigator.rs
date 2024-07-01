// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{Ctx, Object, Result};

use crate::VERSION;

fn get_user_agent() -> String {
    format!("llrt {}", VERSION)
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let navigator = Object::new(ctx.clone())?;

    navigator.set("userAgent", get_user_agent())?;

    globals.set("navigator", navigator)?;

    Ok(())
}
