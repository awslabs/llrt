// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    pin::{pin, Pin},
    ptr::NonNull,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, MutexGuard,
    },
    time::Duration,
};

use llrt_context::CtxExtension;
#[cfg(feature = "hooking")]
use llrt_hooking::{invoke_async_hook, register_finalization_registry, HookType, ProviderType};
use llrt_utils::module::{export_default, ModuleInfo};
use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    qjs, Ctx, Function, Persistent, Result, Value,
};
use tokio::{
    select,
    sync::Notify,
    time::{Instant, Sleep},
};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);
static RT_TIMER_STATE: Lazy<Mutex<Vec<RuntimeTimerState>>> = Lazy::new(|| Mutex::new(Vec::new()));

pub struct RuntimeTimerState {
    timers: Vec<Timeout>,
    rt: *mut qjs::JSRuntime,
    running: bool,
    deadline: Instant,
    notify: Rc<Notify>,
}
impl RuntimeTimerState {
    fn new(rt: *mut qjs::JSRuntime) -> Self {
        let deadline = Instant::now() + Duration::from_secs(86400 * 365 * 30);
        Self {
            timers: Default::default(),
            rt,
            deadline,
            running: false,
            notify: Default::default(),
        }
    }
}

unsafe impl Send for RuntimeTimerState {}

#[derive(Clone)]
pub struct Timeout {
    callback: Option<Persistent<Function<'static>>>,
    deadline: Instant,
    raw_ctx: NonNull<qjs::JSContext>,
    id: usize,
    repeating: bool,
    interval: u64,
}

impl Default for Timeout {
    fn default() -> Self {
        Self {
            callback: None,
            deadline: Instant::now(),
            raw_ctx: NonNull::dangling(),
            id: 0,
            repeating: false,
            interval: 0,
        }
    }
}

fn set_immediate<'js>(_ctx: Ctx<'js>, cb: Function<'js>) -> Result<()> {
    // SAFETY: Since it checks in advance whether it is an Function type, we can always get a pointer to the Function.
    let _uid = unsafe { cb.as_raw().u.ptr } as usize;
    #[cfg(feature = "hooking")]
    {
        register_finalization_registry(&_ctx, cb.clone().into_value(), _uid)?;
        invoke_async_hook(&_ctx, HookType::Init, ProviderType::Immediate, _uid)?;
        invoke_async_hook(&_ctx, HookType::Before, ProviderType::None, _uid)?;
    }
    cb.defer::<()>(())?;
    #[cfg(feature = "hooking")]
    {
        invoke_async_hook(&_ctx, HookType::After, ProviderType::None, _uid)?;
    }
    Ok(())
}

pub fn set_timeout_interval<'js>(
    ctx: &Ctx<'js>,
    cb: Function<'js>,
    delay: u64,
    repeating: bool,
) -> Result<usize> {
    // SAFETY: Since it checks in advance whether it is an Function type, we can always get a pointer to the Function.
    let uid = unsafe { cb.as_raw().u.ptr } as usize;
    #[cfg(feature = "hooking")]
    {
        let provider_type = if repeating {
            ProviderType::Interval
        } else {
            ProviderType::Timeout
        };
        register_finalization_registry(ctx, cb.clone().into_value(), uid)?;
        invoke_async_hook(ctx, HookType::Init, provider_type, uid)?;
    }
    let deadline = Instant::now() + Duration::from_millis(delay);
    let id = TIMER_ID.fetch_add(1, Ordering::Relaxed);

    let callback = Persistent::<Function>::save(ctx, cb);

    let timeout = Timeout {
        deadline,
        callback: Some(callback),
        raw_ctx: ctx.as_raw(),
        id,
        repeating,
        interval: delay,
    };

    let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

    let mut rt_timer = RT_TIMER_STATE.lock().unwrap();
    let state = get_timer_state(&mut rt_timer, rt_ptr);
    state.timers.push(timeout);
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
        create_spawn_loop(rt_ptr, ctx, timer_abort, deadline, uid)?;
    }

    Ok(id)
}

fn get_timer_state<'a>(
    state_ref: &'a mut MutexGuard<Vec<RuntimeTimerState>>,
    rt: *mut qjs::JSRuntime,
) -> &'a mut RuntimeTimerState {
    let rt_timers = state_ref.iter_mut().find(|state| state.rt == rt);

    //save a branch
    unsafe { rt_timers.unwrap_unchecked() }
}

fn clear_timeout_interval(ctx: Ctx<'_>, id: Opt<Value>) -> Result<()> {
    if let Some(id) = id.0.and_then(|v| v.as_number()) {
        let id = id as usize;
        let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
        let mut rt_timers = RT_TIMER_STATE.lock().unwrap();

        let state = get_timer_state(&mut rt_timers, rt);
        if let Some(timeout) = state.timers.iter_mut().find(|t| t.id == id) {
            let _ = timeout.callback.take();
            timeout.repeating = false;
            timeout.deadline = Instant::now() - Duration::from_secs(1);
            state.notify.notify_one()
        }
    }

    Ok(())
}

pub struct TimersModule;

impl ModuleDef for TimersModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("setTimeout")?;
        declare.declare("clearTimeout")?;
        declare.declare("setInterval")?;
        declare.declare("setImmediate")?;
        declare.declare("clearInterval")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();

        export_default(ctx, exports, |default| {
            let functions = [
                "setTimeout",
                "clearTimeout",
                "setInterval",
                "clearInterval",
                "setImmediate",
            ];
            for func_name in functions {
                let function: Function = globals.get(func_name)?;
                default.set(func_name, function)?;
            }
            Ok(())
        })?;

        Ok(())
    }
}

impl From<TimersModule> for ModuleInfo<TimersModule> {
    fn from(val: TimersModule) -> Self {
        ModuleInfo {
            name: "timers",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

    let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
    rt_timers.push(RuntimeTimerState::new(rt_ptr));

    let globals = ctx.globals();

    globals.set(
        "setTimeout",
        Func::from(move |ctx, cb, delay: Opt<f64>| {
            let delay = delay.unwrap_or(0.).max(0.) as u64;
            set_timeout_interval(&ctx, cb, delay, false)
        }),
    )?;

    globals.set(
        "setInterval",
        Func::from(move |ctx, cb, delay: Opt<f64>| {
            let delay = delay.unwrap_or(0.).max(0.) as u64;
            set_timeout_interval(&ctx, cb, delay, true)
        }),
    )?;

    globals.set("clearTimeout", Func::from(clear_timeout_interval))?;

    globals.set("clearInterval", Func::from(clear_timeout_interval))?;

    globals.set("setImmediate", Func::from(set_immediate))?;

    Ok(())
}

#[inline(always)]
fn create_spawn_loop(
    rt: *mut qjs::JSRuntime,
    ctx: &Ctx<'_>,
    timer_abort: Rc<Notify>,
    deadline: Instant,
    uid: usize,
) -> Result<()> {
    ctx.spawn_exit_simple(async move {
        let mut sleep = pin!(tokio::time::sleep_until(deadline));

        let mut executing_timers: Vec<Option<ExecutingTimer>> = Default::default();

        loop {
            select! {
                _ = timer_abort.notified() => {}
                _ = sleep.as_mut() => {}
            }

            if !poll_timers(rt, &mut executing_timers, Some(&mut sleep), None, uid)? {
                break;
            }
        }
        Ok(())
    });

    Ok(())
}

pub struct ExecutingTimer(
    Instant,
    NonNull<qjs::JSContext>,
    Persistent<Function<'static>>,
);

unsafe impl Send for ExecutingTimer {}

pub fn poll_timers(
    rt: *mut qjs::JSRuntime,
    call_vec: &mut Vec<Option<ExecutingTimer>>,
    sleep: Option<&mut Pin<&mut Sleep>>,
    deadline: Option<&mut Instant>,
    _uid: usize,
) -> Result<bool> {
    static MIN_SLEEP: Duration = Duration::from_millis(4);
    static FAR_FUTURE: Duration = Duration::from_secs(84200 * 365 * 30);

    let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
    let state = get_timer_state(&mut rt_timers, rt);
    let now = Instant::now();

    let mut had_items = false;
    let mut lowest = now + FAR_FUTURE;
    state.timers.retain_mut(|timeout| {
        had_items = true;
        if timeout.deadline < now {
            let ctx = timeout.raw_ctx;
            if let Some(cb) = timeout.callback.take() {
                if !timeout.repeating {
                    call_vec.push(Some(ExecutingTimer(timeout.deadline, ctx, cb)));
                    return false;
                }
                timeout.deadline = now + Duration::from_millis(timeout.interval);
                if timeout.deadline < lowest {
                    lowest = timeout.deadline;
                }
                call_vec.push(Some(ExecutingTimer(timeout.deadline, ctx, cb.clone())));
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
        if let Some(ExecutingTimer(_, ctx, timeout)) = item.take() {
            let ctx2 = unsafe { Ctx::from_raw(ctx) };

            if is_first_time {
                while ctx2.execute_pending_job() {}
                is_first_time = false;
            }

            if let Ok(timeout) = timeout.restore(&ctx2) {
                #[cfg(feature = "hooking")]
                invoke_async_hook(&ctx2, HookType::Before, ProviderType::None, _uid)?;
                timeout.call::<_, ()>(())?;
                #[cfg(feature = "hooking")]
                invoke_async_hook(&ctx2, HookType::After, ProviderType::None, _uid)?;
            }

            while ctx2.execute_pending_job() {}
        }
    }
    call_vec.clear();

    if !has_items {
        let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
        let state = get_timer_state(&mut rt_timers, rt);
        let is_empty = state.timers.is_empty();
        state.running = !is_empty;

        return Ok(!is_empty);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use super::*;

    #[tokio::test]
    async fn test_timers() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();

                // Assume we have a TimersModule that provides setTimeout, setImmediate, and setInterval
                ModuleEvaluator::eval_rust::<TimersModule>(ctx.clone(), "timers")
                    .await
                    .unwrap();

                // Test setTimeout
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setTimeout",
                    r#"
                        import { setTimeout } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setTimeout(() => resolve('timeout'), 100);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "timeout");

                // Test setImmediate
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setImmediate",
                    r#"
                        import { setImmediate } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setImmediate(() => resolve('immediate'));
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "immediate");

                // Test setInterval
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setInterval",
                    r#"
                        import { setInterval, clearInterval } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                let count = 0;
                                const intervalId = setInterval(() => {
                                    count++;
                                    if (count === 3) {
                                        clearInterval(intervalId);
                                        resolve(count);
                                    }
                                }, 10);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<i32, _>(&ctx, &module, ()).await;
                assert_eq!(result, 3);

                // Test nested timers
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_nestedTimers",
                    r#"
                        import { setTimeout, setImmediate } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setTimeout(() => {
                                    setImmediate(() => {
                                        setTimeout(() => {
                                            resolve('nested');
                                        }, 10);
                                    });
                                }, 10);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "nested");

                // Test canceling timeout
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_cancelTimeout",
                    r#"
                        import { setTimeout, clearTimeout } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                const timeoutId = setTimeout(() => {
                                    resolve('should not happen');
                                }, 10);
                                clearTimeout(timeoutId);
                                setTimeout(() => resolve('canceled'), 20);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "canceled");

                // Test multiple intervals
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_multipleIntervals",
                    r#"
                        import { setInterval, clearInterval } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                let count1 = 0, count2 = 0;
                                const id1 = setInterval(() => {
                                    count1++;
                                    if (count1 === 2) clearInterval(id1);
                                }, 10);
                                const id2 = setInterval(() => {
                                    count2++;
                                    if (count2 === 3) {
                                        clearInterval(id2);
                                        resolve([count1, count2]);
                                    }
                                }, 20);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<Vec<i32>, _>(&ctx, &module, ()).await;
                assert_eq!(result, vec![2, 3]);
            })
        })
        .await;
    }
}
