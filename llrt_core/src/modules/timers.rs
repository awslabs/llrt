// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    qjs, CatchResultExt, Ctx, Function, Persistent, Result, Value,
};

use crate::{module_builder::ModuleInfo, modules::module::export_default, vm::Vm};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct RuntimeTimerState {
    timers: Vec<TimeoutRef>,
    last_time: usize,
    rt: *mut qjs::JSRuntime,
}
impl RuntimeTimerState {
    fn new(rt: *mut qjs::JSRuntime) -> Self {
        Self {
            timers: Vec::new(),
            rt,
            last_time: 0,
        }
    }
}

unsafe impl Send for RuntimeTimerState {}

pub(crate) static RUNTIME_TIMERS: Lazy<Mutex<Vec<RuntimeTimerState>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

struct ExecutingTimer(NonNull<qjs::JSContext>, Persistent<Function<'static>>);

unsafe impl Send for ExecutingTimer {}

static EXECUTING_TIMERS: Lazy<Mutex<Vec<Option<ExecutingTimer>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub struct TimeoutRef {
    callback: Option<Persistent<Function<'static>>>,
    pub expires: usize,
    ctx: NonNull<qjs::JSContext>,
    id: usize,
    repeating: bool,
    delay: usize,
}

unsafe impl Send for TimeoutRef {}

fn set_immediate(cb: Function) -> Result<()> {
    cb.defer::<()>(())?;
    Ok(())
}

fn get_current_time_millis() -> usize {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as usize
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

    Ok(id)
}

fn clear_timeout_interval(ctx: &Ctx<'_>, id: usize) -> Result<()> {
    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let mut rt_timers = RUNTIME_TIMERS.lock().unwrap();

    if let Some(entry) = rt_timers.iter_mut().find(|t| t.rt == rt) {
        if let Some(timeout) = entry.timers.iter_mut().find(|t| t.id == id) {
            if let Some(timeout) = timeout.callback.take() {
                timeout.restore(ctx)?; //prevent memory leaks
            }
            timeout.expires = 0;
            timeout.repeating = false;
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

pub fn init(_ctx: &Ctx<'_>) -> Result<()> {
    //timers handled separately below
    Ok(())
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

    globals.set(
        "clearTimeout",
        Func::from(move |ctx: Ctx, id: Value| {
            if let Some(id) = id.as_number() {
                clear_timeout_interval(&ctx, id as _)
            } else {
                Ok(())
            }
        }),
    )?;

    globals.set(
        "clearInterval",
        Func::from(move |ctx: Ctx, id: Value| {
            if let Some(id) = id.as_number() {
                clear_timeout_interval(&ctx, id as _)
            } else {
                Ok(())
            }
        }),
    )?;

    globals.set("setImmediate", Func::from(set_immediate))?;

    Ok(())
}

pub trait TimerPoller {
    fn poll_timers(&self) -> bool;
}

impl<'js> TimerPoller for Ctx<'js> {
    fn poll_timers(&self) -> bool {
        let rt = unsafe { qjs::JS_GetRuntime(self.as_raw().as_ptr()) };

        poll_timers(rt)
    }
}

pub fn poll_timers(rt: *mut qjs::JSRuntime) -> bool {
    let mut has_pending_timeouts = false;

    let mut rt_timers = RUNTIME_TIMERS.lock().unwrap();

    let mut executing_timers = EXECUTING_TIMERS.lock().unwrap();

    if let Some(state) = rt_timers.iter_mut().find(|state| state.rt == rt) {
        let current_time = get_current_time_millis();
        if current_time - state.last_time >= 1 {
            state.timers.retain_mut(|timeout| {
                if timeout.expires < current_time {
                    let ctx = timeout.ctx;
                    if let Some(cb) = timeout.callback.take() {
                        if !timeout.repeating {
                            executing_timers.push(Some(ExecutingTimer(ctx, cb)));
                            return false;
                        }
                        timeout.expires = current_time + timeout.delay;
                        executing_timers.push(Some(ExecutingTimer(ctx, cb.clone())));
                        timeout.callback.replace(cb);
                    } else {
                        return false;
                    }
                }
                true
            });
        }

        has_pending_timeouts = !state.timers.is_empty();
        state.last_time = current_time;
        drop(rt_timers);
    }

    if !executing_timers.is_empty() {
        has_pending_timeouts = true;
        for item in executing_timers.iter_mut() {
            if let Some(ExecutingTimer(ctx, timeout)) = item.take() {
                let ctx2 = unsafe { Ctx::from_raw(ctx) };
                if let Ok(timeout) = timeout.restore(&ctx2) {
                    if let Err(err) = timeout.call::<_, ()>(()).catch(&ctx2) {
                        Vm::print_error_and_exit(&ctx2, err);
                    }
                }
            }
        }
        executing_timers.clear();
    }
    drop(executing_timers);

    has_pending_timeouts
}
