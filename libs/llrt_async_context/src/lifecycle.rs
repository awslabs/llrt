// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    function::IntoArgs, promise::PromiseHookType, runtime::PromiseHook, Ctx, Function, Object,
    Persistent, Result,
};

use crate::{
    bridge::{is_tracking_active, notify_async_context_observers, AsyncContextEvent},
    register_finalization_registry,
    state::{
        enter_async_scope, exit_async_scope, get_promise_id, insert_promise_id, next_native_id,
        remove_native_resource,
    },
    AsyncResourceKind,
};

#[derive(Clone)]
pub struct AsyncResourceHandle {
    async_id: u64,
    trigger_id: u64,
    _resource: Option<Persistent<Object<'static>>>,
}

impl AsyncResourceHandle {
    pub fn new(ctx: &Ctx<'_>, trigger_id: Option<u64>) -> Result<Self> {
        let (async_id, current_trigger_id) = next_native_id(ctx)?;
        Ok(Self {
            async_id,
            trigger_id: trigger_id.unwrap_or(current_trigger_id),
            _resource: None,
        })
    }

    pub fn async_id(&self) -> u64 {
        self.async_id
    }

    pub fn trigger_id(&self) -> u64 {
        self.trigger_id
    }

    #[inline]
    pub fn call<'js, A>(&self, ctx: &Ctx<'js>, callback: &Function<'js>, args: A) -> Result<()>
    where
        A: IntoArgs<'js>,
    {
        call_async_callback(ctx, callback, args, self.async_id, self.trigger_id)
    }

    #[inline]
    pub fn run<T, F>(&self, ctx: &Ctx<'_>, callback: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        call_async_callback_with(ctx, self.async_id, self.trigger_id, callback)
    }
}

pub fn register_native_resource(ctx: &Ctx<'_>) -> Result<AsyncResourceHandle> {
    if !is_tracking_active(ctx) {
        return Ok(AsyncResourceHandle {
            async_id: 0,
            trigger_id: 0,
            _resource: None,
        });
    }

    let mut handle = AsyncResourceHandle::new(ctx, None)?;
    notify_async_context_observers(
        ctx,
        AsyncContextEvent::Init {
            async_id: handle.async_id,
            trigger_id: handle.trigger_id,
        },
    )?;

    let target = Object::new(ctx.clone())?;
    register_finalization_registry(
        ctx,
        target.clone().into_value(),
        AsyncResourceKind::Native,
        handle.async_id,
        handle.trigger_id,
    )?;
    handle._resource = Some(Persistent::save(ctx, target));
    Ok(handle)
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
fn call_async_callback<'js, A>(
    ctx: &Ctx<'js>,
    callback: &Function<'js>,
    args: A,
    async_id: u64,
    trigger_id: u64,
) -> Result<()>
where
    A: IntoArgs<'js>,
{
    call_async_callback_with(ctx, async_id, trigger_id, || callback.call::<_, ()>(args))
}

#[inline]
fn call_async_callback_with<T, F>(
    ctx: &Ctx<'_>,
    async_id: u64,
    trigger_id: u64,
    callback: F,
) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    let entered_scope = async_id != 0;
    if entered_scope {
        enter_async_scope(ctx, (async_id, trigger_id))?;
    }
    let result = callback();
    let after_result = if entered_scope {
        exit_async_scope(ctx, async_id)
    } else {
        Ok(())
    };
    let value = result?;
    after_result?;
    Ok(value)
}

pub fn promise_hook_tracker() -> PromiseHook {
    Box::new(|ctx, type_, promise, parent| {
        let _ = (|| -> Result<()> {
            match type_ {
                PromiseHookType::Init => {
                    let (async_id, trigger_id) = insert_promise_id(&ctx, &promise, &parent)?;
                    let _ = register_finalization_registry(
                        &ctx,
                        promise,
                        AsyncResourceKind::Promise,
                        async_id,
                        trigger_id,
                    );
                    notify_async_context_observers(
                        &ctx,
                        AsyncContextEvent::Init {
                            async_id,
                            trigger_id,
                        },
                    )?;
                },
                PromiseHookType::Before | PromiseHookType::After => {
                    let (async_id, trigger_id) = get_promise_id(&ctx, &promise)?;
                    if async_id == 0 {
                        return Ok(());
                    }
                    match type_ {
                        PromiseHookType::Before => enter_async_scope(&ctx, (async_id, trigger_id))?,
                        PromiseHookType::After => exit_async_scope(&ctx, async_id)?,
                        _ => unreachable!(),
                    }
                },
                PromiseHookType::Resolve => {},
            }
            Ok(())
        })();
    })
}
