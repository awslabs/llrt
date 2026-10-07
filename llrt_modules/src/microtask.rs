// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_async_context::{
    is_tracking_active,
    lifecycle::{async_resource_notify, call_async_callback, AsyncLifecycle},
    register_finalization_registry, AsyncResourceKind,
};
use rquickjs::{prelude::OnceFn, Ctx, Function, Object, Persistent, Result};

fn queue_microtask<'js>(ctx: Ctx<'js>, cb: Function<'js>) -> Result<()> {
    if !is_tracking_active(&ctx) {
        cb.defer::<()>(())?;
        return Ok(());
    }

    let (async_id, trigger_id) = async_resource_notify(&ctx, AsyncLifecycle::Init, 0, 0)?;
    if async_id == 0 {
        cb.defer::<()>(())?;
        return Ok(());
    }

    let resource = Object::new(ctx.clone())?;
    register_finalization_registry(
        &ctx,
        resource.clone().into_value(),
        AsyncResourceKind::Native,
        async_id,
        trigger_id,
    )?;

    let resource = Persistent::save(&ctx, resource);
    let callback = Persistent::save(&ctx, cb);
    Function::new(
        ctx.clone(),
        OnceFn::new(move |ctx| {
            let _resource = resource;
            call_async_callback(&ctx, &callback.restore(&ctx)?, (), async_id, trigger_id)
        }),
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
