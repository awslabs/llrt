// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    qjs, CatchResultExt, Ctx, Function, Persistent, Result,
};

use crate::{module_builder::ModuleInfo, modules::module::export_default, vm::Vm};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);
static TIME_POLL_ACTIVE: AtomicBool = AtomicBool::new(false);
pub struct TimeoutRef {
    callback: Option<Persistent<Function<'static>>>,
    pub expires: usize,
    ctx: NonNull<qjs::JSContext>,
    id: usize,
    repeating: bool,
    delay: usize,
}

fn set_immediate(cb: Function) -> Result<()> {
    cb.defer::<()>(())?;
    Ok(())
}

fn get_current_time_millis() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| t.as_millis() as usize)
        .unwrap_or(0)
}

pub fn set_timeout_interval<'js>(
    ctx: &Ctx<'js>,
    cb: Function<'js>,
    delay: usize,
    repeating: bool,
) -> Result<usize> {
    let expires = get_current_time_millis() + delay;
    let id = TIMER_ID.fetch_add(1, Ordering::Relaxed);

    let callback = Persistent::<Function>::save(ctx, cb);

    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let mut rt_timers = RUNTIME_TIMERS.lock().unwrap();

    let timeout_ref = TimeoutRef {
        expires,
        callback: Some(callback),
        ctx: ctx.as_raw(),
        id,
        repeating,
        delay,
    };

    if let Some(entry) = rt_timers.iter_mut().find(|state| state.rt == rt) {
        entry.timers.push(timeout_ref);
    } else {
        let mut entry = RuntimeTimerState::new(rt);
        entry.timers.push(timeout_ref);
        rt_timers.push(entry);
    }
    drop(rt_timers);
    create_spawn_loop(ctx, rt)?;

    Ok(id)
}

fn clear_timeout_interval(ctx: Ctx<'_>, id: usize) -> Result<()> {
    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let mut rt_timers = RUNTIME_TIMERS.lock().unwrap();

    if let Some(state) = rt_timers.iter_mut().find(|t| t.rt == rt) {
        if let Some(timeout) = state.timers.iter_mut().find(|t| t.id == id) {
            if let Some(timeout) = timeout.callback.take() {
                timeout.restore(&ctx)?; //prevent memory leaks
            }
            timeout.expires = 0;
            timeout.repeating = false;
        }
    }

    Ok(())
}

unsafe impl Send for RuntimeTimerState {}

pub(crate) static RUNTIME_TIMERS: Lazy<Mutex<Vec<RuntimeTimerState>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub(crate) struct RuntimeTimerState {
    timers: Vec<TimeoutRef>,
    rt: *mut qjs::JSRuntime,
}
impl RuntimeTimerState {
    fn new(rt: *mut qjs::JSRuntime) -> Self {
        Self {
            timers: Vec::new(),
            rt,
        }
    }
}

pub struct TimersModule;

impl ModuleDef for TimersModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("setTimeout")?;
        declare.declare("clearTimeout")?;
        declare.declare("setInterval")?;
        declare.declare("clearInterval")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();

        export_default(ctx, exports, |default| {
            let functions = ["setTimeout", "clearTimeout", "setInterval", "clearInterval"];
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

pub fn init_timers(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    globals.set(
        "setTimeout",
        Func::from(move |ctx, cb, delay: Opt<f64>| {
            let delay = delay.unwrap_or(0.).max(0.) as usize;
            set_timeout_interval(&ctx, cb, delay, false)
        }),
    )?;

    globals.set(
        "setInterval",
        Func::from(move |ctx, cb, delay: Opt<f64>| {
            let delay = delay.unwrap_or(0.).max(0.) as usize;
            set_timeout_interval(&ctx, cb, delay, true)
        }),
    )?;

    globals.set("clearTimeout", Func::from(clear_timeout_interval))?;

    globals.set("clearInterval", Func::from(clear_timeout_interval))?;

    globals.set("setImmediate", Func::from(set_immediate))?;

    Ok(())
}

pub struct ExecutingTimer(NonNull<qjs::JSContext>, Persistent<Function<'static>>);

unsafe impl Send for ExecutingTimer {}

#[inline(always)]
fn create_spawn_loop(ctx: &Ctx<'_>, rt: *mut qjs::JSRuntime) -> Result<()> {
    if !TIME_POLL_ACTIVE.swap(true, Ordering::Relaxed) {
        ctx.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(4));

            let mut executing_timers: Option<Vec<Option<ExecutingTimer>>> = Some(Vec::new());
            let mut exit_after_next_tick = false;
            loop {
                interval.tick().await;

                if !poll_timers(rt, &mut executing_timers, &mut exit_after_next_tick) {
                    break;
                }
            }
            TIME_POLL_ACTIVE.store(false, Ordering::Relaxed);
        });
    }
    Ok(())
}

pub fn poll_timers(
    rt: *mut qjs::JSRuntime,
    executing_timers: &mut Option<Vec<Option<ExecutingTimer>>>,
    exit_after_next_tick: &mut bool,
) -> bool {
    let mut rt_timers = RUNTIME_TIMERS.lock().unwrap();
    if let Some(state) = rt_timers.iter_mut().find(|t| t.rt == rt) {
        let mut call_vec = executing_timers.take().unwrap(); //avoid creating a new vec
        let current_time = get_current_time_millis();
        let mut had_items = false;

        state.timers.retain_mut(|timeout| {
            had_items = true;
            *exit_after_next_tick = false;
            if timeout.expires < current_time {
                let ctx = timeout.ctx;
                if let Some(cb) = timeout.callback.take() {
                    if !timeout.repeating {
                        call_vec.push(Some(ExecutingTimer(ctx, cb)));
                        return false;
                    }
                    timeout.expires = current_time + timeout.delay;
                    call_vec.push(Some(ExecutingTimer(ctx, cb.clone())));
                    timeout.callback.replace(cb);
                } else {
                    return false;
                }
            }
            true
        });

        drop(rt_timers);

        if !call_vec.is_empty() {
            for item in call_vec.iter_mut() {
                if let Some(ExecutingTimer(ctx, timeout)) = item.take() {
                    let ctx2 = unsafe { Ctx::from_raw(ctx) };
                    if let Ok(timeout) = timeout.restore(&ctx2) {
                        if let Err(err) = timeout.call::<_, ()>(()).catch(&ctx2) {
                            Vm::print_error_and_exit(&ctx2, err);
                        }
                    }
                }
            }
            call_vec.clear();
        }

        executing_timers.replace(call_vec);

        if !had_items {
            if *exit_after_next_tick {
                return false;
            }
            *exit_after_next_tick = true;
        }
    }
    true
}
