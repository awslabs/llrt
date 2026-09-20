// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    function::IntoArgs, promise::PromiseHookType, runtime::PromiseHook, Ctx, Function, Result,
    Value,
};

use crate::{
    bridge::{is_tracking_active, notify_async_context_observers, AsyncContextEvent},
    register_finalization_registry,
    state::{
        enter_async_scope, exit_async_scope, get_current_id, get_promise_id, insert_promise_id,
        next_native_id, remove_native_resource,
    },
    AsyncResourceKind,
};

#[derive(Clone, Copy, PartialEq)]
pub enum AsyncLifecycle {
    Init,
    Before,
    After,
}

pub fn async_resource_notify(
    ctx: &Ctx<'_>,
    lifecycle: AsyncLifecycle,
    async_id: u64,
    trigger_id: u64,
) -> Result<(u64, u64)> {
    if !is_tracking_active(ctx) {
        return Ok((0, 0));
    }
    match lifecycle {
        AsyncLifecycle::Init => {
            let id = if async_id == 0 {
                next_native_id(ctx)?
            } else {
                (
                    async_id,
                    if trigger_id == 0 {
                        get_current_id(ctx)?.0
                    } else {
                        trigger_id
                    },
                )
            };
            notify_async_context_observers(
                ctx,
                AsyncContextEvent::Init {
                    async_id: id.0,
                    trigger_id: id.1,
                },
            )?;
            Ok(id)
        },
        AsyncLifecycle::Before => {
            if async_id != 0 {
                enter_async_scope(ctx, (async_id, trigger_id))?;
            }
            Ok((async_id, trigger_id))
        },
        AsyncLifecycle::After => {
            if async_id != 0 {
                exit_async_scope(ctx, async_id)?;
            }
            Ok((async_id, trigger_id))
        },
    }
}

pub fn promise_hook_tracker() -> PromiseHook {
    Box::new(|ctx, type_, promise, parent| {
        let _ = track_promise_hook(&ctx, type_, promise, parent);
    })
}

fn track_promise_hook<'js>(
    ctx: &Ctx<'js>,
    type_: PromiseHookType,
    promise: Value<'js>,
    parent: Value<'js>,
) -> Result<()> {
    match type_ {
        PromiseHookType::Init => {
            let (async_id, trigger_id) = insert_promise_id(ctx, &promise, &parent)?;
            let _ = register_finalization_registry(
                ctx,
                promise,
                AsyncResourceKind::Promise,
                async_id,
                trigger_id,
            );
            notify_async_context_observers(
                ctx,
                AsyncContextEvent::Init {
                    async_id,
                    trigger_id,
                },
            )?;
        },
        PromiseHookType::Before | PromiseHookType::After => {
            let (async_id, trigger_id) = get_promise_id(ctx, &promise)?;
            if async_id == 0 {
                return Ok(());
            }
            match type_ {
                PromiseHookType::Before => enter_async_scope(ctx, (async_id, trigger_id))?,
                PromiseHookType::After => exit_async_scope(ctx, async_id)?,
                _ => unreachable!(),
            }
        },
        PromiseHookType::Resolve => {},
    }
    Ok(())
}

pub fn finalize_async_resource(
    ctx: &Ctx<'_>,
    kind: AsyncResourceKind,
    token_id: u64,
    async_id: u64,
) -> Result<()> {
    if kind == AsyncResourceKind::Native {
        remove_native_resource(ctx, token_id)?;
    }
    notify_async_context_observers(
        ctx,
        AsyncContextEvent::Finalized {
            kind,
            token_id,
            async_id,
        },
    )
}

#[inline]
pub fn call_async_callback<'js, A>(
    ctx: &Ctx<'js>,
    callback: &Function<'js>,
    args: A,
    async_id: u64,
    trigger_id: u64,
) -> Result<()>
where
    A: IntoArgs<'js>,
{
    async_resource_notify(ctx, AsyncLifecycle::Before, async_id, trigger_id)?;
    let result = callback.call::<_, ()>(args);
    let after_result =
        async_resource_notify(ctx, AsyncLifecycle::After, async_id, trigger_id).map(|_| ());
    result?;
    after_result
}
