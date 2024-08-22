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

use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt},
    qjs, CatchResultExt, Ctx, Function, Persistent, Result,
};
use tokio::{
    select,
    sync::Notify,
    time::{Instant, Sleep},
};

use crate::{module_builder::ModuleInfo, modules::module::export_default, vm::Vm};

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

fn set_immediate(cb: Function) -> Result<()> {
    cb.defer::<()>(())?;
    Ok(())
}

pub fn set_timeout_interval<'js>(
    ctx: &Ctx<'js>,
    cb: Function<'js>,
    delay: u64,
    repeating: bool,
) -> Result<usize> {
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
    let mut abort_timer_rx = None;
    let task_running = state.running;
    if task_running {
        if deadline < state.deadline {
            state.deadline = deadline;
            state.notify.notify_one();
        }
    } else {
        state.running = true;
        abort_timer_rx = Some(state.notify.clone());
    }
    drop(rt_timer);

    if !task_running {
        create_spawn_loop(
            rt_ptr,
            ctx,
            unsafe { abort_timer_rx.unwrap_unchecked() },
            deadline,
        )?;
    }

    Ok(id)
}

fn get_timer_state<'a>(
    state_ref: &'a mut MutexGuard<Vec<RuntimeTimerState>>,
    rt: *mut qjs::JSRuntime,
) -> &'a mut RuntimeTimerState {
    let rt_timers = state_ref.iter_mut().find(|state| state.rt == rt);

    (unsafe { rt_timers.unwrap_unchecked() }) as _
}

fn clear_timeout_interval(ctx: Ctx<'_>, id: usize) -> Result<()> {
    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let mut rt_timers = RT_TIMER_STATE.lock().unwrap();

    let state = get_timer_state(&mut rt_timers, rt);
    if let Some(timeout) = state.timers.iter_mut().find(|t| t.id == id) {
        if let Some(timeout) = timeout.callback.take() {
            timeout.restore(&ctx)?; //prevent memory leaks
        }
        timeout.repeating = false;
        timeout.deadline = Instant::now() - Duration::from_secs(1);
        state.notify.notify_one()
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

pub fn init_timers(ctx: &Ctx<'_>) -> Result<()> {
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
) -> Result<()> {
    ctx.spawn(async move {
        let mut sleep = pin!(tokio::time::sleep_until(deadline));

        let mut executing_timers: Vec<Option<ExecutingTimer>> = Default::default();

        loop {
            select! {
                _ = timer_abort.notified() => {}
                _ = sleep.as_mut() => {}
            }

            if !poll_timers(rt, &mut executing_timers, Some(&mut sleep), None) {
                break;
            }
        }
    });

    Ok(())
}

pub struct ExecutingTimer(NonNull<qjs::JSContext>, Persistent<Function<'static>>);

unsafe impl Send for ExecutingTimer {}

pub fn poll_timers(
    rt: *mut qjs::JSRuntime,
    call_vec: &mut Vec<Option<ExecutingTimer>>,
    sleep: Option<&mut Pin<&mut Sleep>>,
    deadline: Option<&mut Instant>,
) -> bool {
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
                    call_vec.push(Some(ExecutingTimer(ctx, cb)));
                    return false;
                }
                timeout.deadline = now + Duration::from_millis(timeout.interval);
                if timeout.deadline < lowest {
                    lowest = timeout.deadline;
                }
                call_vec.push(Some(ExecutingTimer(ctx, cb.clone())));
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

    if !has_items {
        let mut rt_timers = RT_TIMER_STATE.lock().unwrap();
        let state = get_timer_state(&mut rt_timers, rt);
        let is_empty = state.timers.is_empty();
        state.running = !is_empty;

        return !is_empty;
    }
    true
}
