// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{Ctx, Result, Value};

mod bridge;
pub mod lifecycle;
mod promise_hook;
pub mod state;

pub use bridge::{
    add_async_context_observer, is_tracking_active, notify_async_context_observers,
    promise_hook_controller, AsyncContextBridge, AsyncContextEvent, AsyncContextObserver,
};
pub use promise_hook::PromiseHookController;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsyncResourceKind {
    Native = 0,
    Promise = 1,
}

pub fn register_finalization_registry<'js>(
    ctx: &Ctx<'js>,
    target: Value<'js>,
    kind: AsyncResourceKind,
    async_id: u64,
    trigger_id: u64,
) -> Result<()> {
    if async_id == 0 {
        return Ok(());
    }
    let Some(bridge) = ctx.userdata::<AsyncContextBridge>() else {
        return Ok(());
    };
    if !bridge.promise_hook_controller.is_active() {
        return Ok(());
    }
    (bridge.async_resource_register)(ctx, target, kind, async_id, trigger_id)
}
