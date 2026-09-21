// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_hooking::{
    invoke_async_hook, is_hooking_enabled, register_finalization_registry, AsyncTokenKind,
    HookType, ProviderType,
};
use rquickjs::{prelude::OnceFn, Ctx, Function, Object, Persistent, Result};

pub(crate) fn queue_microtask<'js>(ctx: Ctx<'js>, cb: Function<'js>) -> Result<()> {
    if !is_hooking_enabled() {
        cb.defer::<()>(())?;
        return Ok(());
    }

    let (async_id, trigger_id) =
        invoke_async_hook(&ctx, HookType::Init, ProviderType::Microtask, 0, 0)?;
    if async_id == 0 {
        cb.defer::<()>(())?;
        return Ok(());
    }

    let resource = Object::new(ctx.clone())?;
    register_finalization_registry(
        &ctx,
        resource.clone().into_value(),
        AsyncTokenKind::Native,
        async_id,
        trigger_id,
    )?;

    let resource = Persistent::save(&ctx, resource);
    let callback = Persistent::save(&ctx, cb);
    Function::new(
        ctx.clone(),
        OnceFn::new(move |ctx| {
            let _resource = resource;
            callback.restore(&ctx)?.call::<_, ()>(())
        }),
    )?
    .defer::<()>(())?;
    Ok(())
}
