// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::time::Duration;

use llrt_hooking::{
    invoke_async_hook, is_hooking_enabled, register_finalization_registry, AsyncTokenKind,
    HookType, ProviderType,
};
use rquickjs::{prelude::Opt, qjs, Ctx, Exception, Function, Object, Persistent, Result, Value};
use tokio::time::Instant;

use crate::{
    scheduler::create_spawn_loop,
    state::{timer_state, AsyncResource, Timeout},
};

pub fn set_timeout_interval<'js>(
    ctx: &Ctx<'js>,
    cb: Function<'js>,
    delay: u64,
    provider_type: ProviderType,
) -> Result<usize> {
    let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    {
        let rt_timers = timer_state();
        let Some(state) = rt_timers.get(&(rt_ptr as usize)) else {
            return Err(Exception::throw_internal(
                ctx,
                "Async runtime is not initialized",
            ));
        };
        if state.shutting_down {
            return Err(Exception::throw_internal(
                ctx,
                "Async runtime is shutting down",
            ));
        }
    }

    let hooks_enabled = is_hooking_enabled();
    let (repeating, deadline) = match provider_type {
        ProviderType::Immediate => (false, Instant::now() - Duration::from_secs(600)),
        ProviderType::Timeout => (false, Instant::now() + Duration::from_millis(delay)),
        ProviderType::Interval => (true, Instant::now() + Duration::from_millis(delay)),
        _ => {
            return Err(Exception::throw_type(
                ctx,
                "The specified provider type is not supported.",
            ))
        },
    };

    let async_resource = if hooks_enabled {
        let (async_id, trigger_id) = invoke_async_hook(ctx, HookType::Init, provider_type, 0, 0)?;
        if async_id == 0 {
            None
        } else {
            let resource = Object::new(ctx.clone())?;
            register_finalization_registry(
                ctx,
                resource.clone().into_value(),
                AsyncTokenKind::Native,
                async_id,
                trigger_id,
            )?;
            Some(AsyncResource {
                resource: Persistent::save(ctx, resource),
                async_id,
                trigger_id,
            })
        }
    } else {
        None
    };

    let mut rt_timer = timer_state();
    let Some(state) = rt_timer.get_mut(&(rt_ptr as usize)) else {
        return Err(Exception::throw_internal(
            ctx,
            "Async runtime is not initialized",
        ));
    };
    if state.shutting_down {
        return Err(Exception::throw_internal(
            ctx,
            "Async runtime is shutting down",
        ));
    }
    let id = state.next_timer_id;
    state.next_timer_id = state
        .next_timer_id
        .checked_add(1)
        .ok_or_else(|| Exception::throw_internal(ctx, "Timer ID overflow"))?;
    state.timers.push(Timeout {
        deadline,
        callback: Some(Persistent::save(ctx, cb)),
        async_resource,
        raw_ctx: ctx.as_raw(),
        id,
        repeating,
        interval: delay,
    });
    let task_running = state.running;
    if task_running {
        if deadline < state.deadline {
            state.deadline = deadline;
            state.notify.notify_one();
        }
    } else {
        state.running = true;
        let timer_abort = state.notify.clone();
        drop(rt_timer);
        create_spawn_loop(rt_ptr, ctx, timer_abort, deadline)?;
    }

    Ok(id)
}

pub fn set_timeout<'js>(ctx: &Ctx<'js>, cb: Function<'js>, delay: u64) -> Result<usize> {
    set_timeout_interval(ctx, cb, delay, ProviderType::Timeout)
}

pub fn clear_timeout_interval(ctx: Ctx<'_>, id: Opt<Value>) -> Result<()> {
    if let Some(id) = id.0.and_then(|v| v.as_number()) {
        if !id.is_finite() || id < 0. || id.fract() != 0. {
            return Ok(());
        }
        let id = id as usize;
        let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
        let mut rt_timers = timer_state();
        let Some(state) = rt_timers.get_mut(&(rt as usize)) else {
            return Ok(());
        };
        if state.shutting_down {
            return Ok(());
        }
        if let Some(timeout) = state.timers.iter_mut().find(|t| t.id == id) {
            let _ = timeout.callback.take();
            timeout.repeating = false;
            timeout.deadline = Instant::now() - Duration::from_secs(1);
            state.notify.notify_one();
        }
    }
    Ok(())
}
