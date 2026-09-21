// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    pin::{pin, Pin},
    ptr::NonNull,
    rc::Rc,
    time::Duration,
};

use llrt_context::CtxExtension;
use llrt_hooking::{invoke_async_hook, HookType, ProviderType};
use rquickjs::{qjs, Ctx, Function, Persistent, Result};
use tokio::{
    select,
    sync::Notify,
    time::{Instant, Sleep},
};

use crate::state::{finish_shutdown, timer_state, AsyncResource};

pub(crate) fn create_spawn_loop(
    rt: *mut qjs::JSRuntime,
    ctx: &Ctx<'_>,
    timer_abort: Rc<Notify>,
    deadline: Instant,
) -> Result<()> {
    ctx.spawn_exit_simple(async move {
        let mut sleep = pin!(tokio::time::sleep_until(deadline));
        let mut executing_timers: Vec<Option<ExecutingTimer>> = Default::default();
        loop {
            select! {
                _ = timer_abort.notified() => {}
                _ = sleep.as_mut() => {}
            }
            match poll_timers(rt, &mut executing_timers, Some(&mut sleep), None) {
                Ok(true) => {},
                Ok(false) => break,
                Err(error) => {
                    finish_shutdown(rt as usize);
                    return Err(error);
                },
            }
        }
        Ok(())
    });
    Ok(())
}

struct ExecutingTimer(
    Instant,
    NonNull<qjs::JSContext>,
    Option<AsyncResource>,
    Persistent<Function<'static>>,
);

unsafe impl Send for ExecutingTimer {}

fn poll_timers(
    rt: *mut qjs::JSRuntime,
    call_vec: &mut Vec<Option<ExecutingTimer>>,
    sleep: Option<&mut Pin<&mut Sleep>>,
    deadline: Option<&mut Instant>,
) -> Result<bool> {
    static MIN_SLEEP: Duration = Duration::from_millis(4);
    static FAR_FUTURE: Duration = Duration::from_secs(84200 * 365 * 30);

    let mut rt_timers = timer_state();
    let Some(state) = rt_timers.get_mut(&(rt as usize)) else {
        return Ok(false);
    };
    if state.shutting_down {
        state.running = false;
        drop(rt_timers);
        finish_shutdown(rt as usize);
        return Ok(false);
    }
    let now = Instant::now();
    let mut had_items = false;
    let mut lowest = now + FAR_FUTURE;

    state.timers.retain_mut(|timeout| {
        had_items = true;
        if timeout.deadline < now {
            let ctx = timeout.raw_ctx;
            if let Some(cb) = timeout.callback.take() {
                if !timeout.repeating {
                    call_vec.push(Some(ExecutingTimer(
                        timeout.deadline,
                        ctx,
                        timeout.async_resource.take(),
                        cb,
                    )));
                    return false;
                }
                timeout.deadline = now + Duration::from_millis(timeout.interval);
                if timeout.deadline < lowest {
                    lowest = timeout.deadline;
                }
                call_vec.push(Some(ExecutingTimer(
                    timeout.deadline,
                    ctx,
                    timeout.async_resource.clone(),
                    cb.clone(),
                )));
                timeout.callback.replace(cb);
            } else {
                return false;
            }
        } else if timeout.deadline < lowest {
            lowest = timeout.deadline;
        }
        true
    });

    let has_items = !state.timers.is_empty();
    if had_items {
        if lowest - now < MIN_SLEEP {
            lowest = now + MIN_SLEEP;
        }
        if let Some(sleep) = sleep {
            sleep.as_mut().reset(lowest);
        }
        if let Some(deadline) = deadline {
            *deadline = lowest;
        }
        state.deadline = lowest;
    }
    drop(rt_timers);

    call_vec.sort_unstable_by_key(|v| v.as_ref().map(|v| v.0));
    let mut is_first_time = true;
    for item in call_vec.iter_mut() {
        if let Some(ExecutingTimer(_, ctx, async_resource, timeout)) = item.take() {
            let ctx2 = unsafe { Ctx::from_raw(ctx) };
            if is_first_time {
                while ctx2.execute_pending_job() {}
                is_first_time = false;
            }
            if let Ok(callback) = timeout.restore(&ctx2) {
                if let Some(async_resource) = async_resource {
                    let _resource = async_resource.resource;
                    invoke_async_hook(
                        &ctx2,
                        HookType::Before,
                        ProviderType::None,
                        async_resource.async_id,
                        async_resource.trigger_id,
                    )?;
                    let callback_result = callback.call::<_, ()>(());
                    while ctx2.execute_pending_job() {}
                    let after_result = invoke_async_hook(
                        &ctx2,
                        HookType::After,
                        ProviderType::None,
                        async_resource.async_id,
                        async_resource.trigger_id,
                    );
                    callback_result?;
                    after_result?;
                } else {
                    callback.call::<_, ()>(())?;
                }
            }
            while ctx2.execute_pending_job() {}
        }
    }
    call_vec.clear();

    if !has_items {
        let mut rt_timers = timer_state();
        let Some(state) = rt_timers.get_mut(&(rt as usize)) else {
            return Ok(false);
        };
        if state.shutting_down {
            state.running = false;
            drop(rt_timers);
            finish_shutdown(rt as usize);
            return Ok(false);
        }
        let is_empty = state.timers.is_empty();
        state.running = !is_empty;
        return Ok(!is_empty);
    }
    Ok(true)
}

pub fn run_pending_jobs(ctx: &Ctx<'_>) -> Result<()> {
    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let should_poll = timer_state()
        .get(&(rt as usize))
        .is_some_and(|state| Instant::now() >= state.deadline);
    if should_poll {
        let mut executing_timers = Vec::new();
        poll_timers(rt, &mut executing_timers, None, None)?;
    }
    ctx.execute_pending_job();
    Ok(())
}
