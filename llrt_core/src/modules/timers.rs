// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    qjs, CatchResultExt, Ctx, Function, Persistent, Result,
};

use crate::{module_builder::ModuleInfo, modules::module::export_default, vm::Vm};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);

pub type TimerRefList = Arc<Mutex<Vec<TimeoutRef>>>;

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

fn set_timeout_interval<'js>(
    ctx: &Ctx<'js>,
    timeouts: &TimerRefList,
    cb: Function<'js>,
    delay: usize,
    repeating: bool,
) -> Result<usize> {
    let expires = get_current_time_millis() + delay;
    let id = TIMER_ID.fetch_add(1, Ordering::Relaxed);

    let callback = Persistent::<Function>::save(ctx, cb);

    timeouts.lock().unwrap().push(TimeoutRef {
        expires,
        callback: Some(callback),
        ctx: ctx.as_raw(),
        id,
        repeating,
        delay,
    });

    Ok(id)
}

fn clear_timeout_interval(ctx: &Ctx<'_>, timeouts: &TimerRefList, id: usize) -> Result<()> {
    let mut timeouts = timeouts.lock().unwrap();
    if let Some(timeout) = timeouts.iter_mut().find(|t| t.id == id) {
        if let Some(timeout) = timeout.callback.take() {
            timeout.restore(ctx)?; //prevent memory leaks
        }
        timeout.expires = 0;
        timeout.repeating = false;
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

pub fn init_timers(ctx: &Ctx<'_>, timeout_refs: &TimerRefList) -> Result<()> {
    let globals = ctx.globals();

    let timeout_refs_1 = timeout_refs.clone();
    let timeout_refs_2 = timeout_refs.clone();
    let timeout_refs_3 = timeout_refs.clone();
    let timeout_refs_4 = timeout_refs.clone();

    globals.set(
        "setTimeout",
        Func::from(move |ctx, cb, delay| {
            set_timeout_interval(&ctx, &timeout_refs_1, cb, delay, false)
        }),
    )?;

    globals.set(
        "setInterval",
        Func::from(move |ctx, cb, delay| {
            set_timeout_interval(&ctx, &timeout_refs_2, cb, delay, true)
        }),
    )?;

    globals.set(
        "clearTimeout",
        Func::from(move |ctx: Ctx, id: usize| clear_timeout_interval(&ctx, &timeout_refs_3, id)),
    )?;

    globals.set(
        "clearInterval",
        Func::from(move |ctx: Ctx, id: usize| clear_timeout_interval(&ctx, &timeout_refs_4, id)),
    )?;

    globals.set("setImmediate", Func::from(set_immediate))?;

    Ok(())
}

pub(crate) fn poll_timers(
    timeouts: &Arc<Mutex<Vec<TimeoutRef>>>,
    timeout_callbacks: &mut Vec<Option<(Persistent<Function<'static>>, NonNull<qjs::JSContext>)>>,
    last_time: &mut usize,
) -> bool {
    let mut timeouts = timeouts.lock().unwrap();

    let current_time = get_current_time_millis();
    if current_time - *last_time >= 1 {
        timeouts.retain_mut(|timeout| {
            if timeout.expires < current_time {
                let ctx = timeout.ctx;
                if let Some(cb) = timeout.callback.take() {
                    if !timeout.repeating {
                        timeout_callbacks.push(Some((cb, ctx)));
                        return false;
                    }
                    timeout.expires = current_time + timeout.delay;
                    timeout_callbacks.push(Some((cb.clone(), ctx)));
                    timeout.callback.replace(cb);
                } else {
                    return false;
                }
            }
            true
        });
    }

    let has_pending_timeouts = !timeouts.is_empty();

    drop(timeouts);

    if !timeout_callbacks.is_empty() {
        for item in timeout_callbacks.iter_mut() {
            if let Some((timeout, ctx)) = item.take() {
                let ctx2 = unsafe { Ctx::from_raw(ctx) };
                if let Ok(timeout) = timeout.restore(&ctx2) {
                    if let Err(err) = timeout.call::<_, ()>(()).catch(&ctx2) {
                        Vm::print_error_and_exit(&ctx2, err);
                    }
                }
            }
        }
        timeout_callbacks.clear();
    }

    *last_time = current_time;
    has_pending_timeouts
}
