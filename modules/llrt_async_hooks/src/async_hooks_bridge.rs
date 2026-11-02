// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_async_context::{
    state::register_async_resource as register_native_async_resource, AsyncResourceKind,
};
use rquickjs::{BigInt, Ctx, JsLifetime, Object, Result, Value};

use crate::finalization_registry::FinalizationRegistryHandle;

pub(crate) struct AsyncHooksBridge {
    finalization_registry: FinalizationRegistryHandle,
}

unsafe impl<'js> JsLifetime<'js> for AsyncHooksBridge {
    type Changed<'to> = AsyncHooksBridge;
}

impl AsyncHooksBridge {
    pub(crate) fn new(ctx: &Ctx<'_>) -> Result<Self> {
        Ok(Self {
            finalization_registry: FinalizationRegistryHandle::new(ctx)?,
        })
    }

    pub(crate) fn register<'js>(
        &self,
        ctx: &Ctx<'js>,
        target: Value<'js>,
        kind: AsyncResourceKind,
        async_id: u64,
        trigger_id: u64,
    ) -> Result<()> {
        let token = Object::new(ctx.clone())?;
        token.set("kind", kind as u8)?;
        token.set("asyncId", BigInt::from_u64(ctx.clone(), async_id)?)?;
        token.set("triggerId", BigInt::from_u64(ctx.clone(), trigger_id)?)?;
        self.finalization_registry
            .register(ctx, target.clone(), token.clone())?;
        if kind == AsyncResourceKind::Native {
            register_native_async_resource(ctx.clone(), token.into(), target)?;
        }
        Ok(())
    }
}

pub(crate) fn register_async_resource_target<'js>(
    ctx: &Ctx<'js>,
    target: Value<'js>,
    kind: AsyncResourceKind,
    async_id: u64,
    trigger_id: u64,
) -> Result<()> {
    let Some(bridge) = ctx.userdata::<AsyncHooksBridge>() else {
        return Ok(());
    };
    bridge.register(ctx, target, kind, async_id, trigger_id)
}
