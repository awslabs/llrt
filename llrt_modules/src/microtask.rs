// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_async_context::lifecycle::register_native_resource;
use rquickjs::{prelude::OnceFn, Ctx, Function, Persistent, Result};

fn queue_microtask<'js>(ctx: Ctx<'js>, cb: Function<'js>) -> Result<()> {
    let async_resource = register_native_resource(&ctx)?;
    let callback = Persistent::save(&ctx, cb);
    Function::new(
        ctx.clone(),
        OnceFn::new(move |ctx| async_resource.call(&ctx, &callback.restore(&ctx)?, ())),
    )?
    .defer::<()>(())?;
    Ok(())
}

pub(crate) fn init(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals().set(
        "queueMicrotask",
        Function::new(ctx.clone(), queue_microtask)?,
    )?;
    Ok(())
}
