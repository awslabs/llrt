// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_async_context::{
    lifecycle::finalize_async_resource,
    state::{get_trigger_id_from_token, parse_async_token},
};
use rquickjs::{
    prelude::{Func, This},
    Constructor, Ctx, Function, Object, Persistent, Result, Value,
};

pub(super) struct FinalizationRegistryHandle {
    registry: Persistent<Object<'static>>,
    register: Persistent<Function<'static>>,
}

impl FinalizationRegistryHandle {
    pub(super) fn new(ctx: &Ctx<'_>) -> Result<Self> {
        let global = ctx.globals();
        let constructor: Constructor = global.get("FinalizationRegistry")?;
        let registry: Object = constructor.construct((Func::from(invoke_finalization_hook),))?;
        let register: Function = registry.get("register")?;
        Ok(Self {
            registry: Persistent::save(ctx, registry),
            register: Persistent::save(ctx, register),
        })
    }

    pub(super) fn register<'js>(
        &self,
        ctx: &Ctx<'js>,
        target: Value<'js>,
        token: Object<'js>,
    ) -> Result<()> {
        let registry = self.registry.clone().restore(ctx)?;
        let register = self.register.clone().restore(ctx)?;
        register.call::<_, ()>((This(registry), target, token))
    }
}

fn invoke_finalization_hook<'js>(ctx: Ctx<'js>, uid: Value<'js>) -> Result<()> {
    let (kind, token_id) = parse_async_token(&ctx, &uid)?;
    let trigger_id = get_trigger_id_from_token(&uid)?;
    if token_id == 0 || trigger_id == 0 {
        return Ok(());
    }

    finalize_async_resource(&ctx, kind, token_id, token_id)?;
    Ok(())
}
