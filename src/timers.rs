// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Function, Result,
};

use crate::{module::export_default, vm::CtxExtension};

static TIMER_ID: AtomicUsize = AtomicUsize::new(0);
static TIME_POLL_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct Timeout<'js> {
    cb: Option<Function<'js>>,
    timeout: usize,
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

fn set_timeout_interval<'js>(
    ctx: &Ctx<'js>,
    timeouts: &Arc<Mutex<Vec<Timeout<'js>>>>,
    cb: Function<'js>,
    delay: usize,
    repeating: bool,
) -> Result<usize> {
    let timeout = get_current_time_millis() + delay;
    let id = TIMER_ID.fetch_add(1, Ordering::Relaxed);
    timeouts.lock().unwrap().push(Timeout {
        cb: Some(cb.clone()),
        timeout,
        id,
        repeating,
        delay,
    });
    if !TIME_POLL_ACTIVE.load(Ordering::Relaxed) {
        poll_timers(ctx, timeouts.clone())?
    }

    Ok(id)
}

fn clear_timeout_interval(timeouts: &Arc<Mutex<Vec<Timeout>>>, id: usize) {
    let mut timeouts = timeouts.lock().unwrap();
    if let Some(timeout) = timeouts.iter_mut().find(|t| t.id == id) {
        timeout.cb.take();
        timeout.timeout = 0;
        timeout.repeating = false;
    }
}

pub struct TimersModule;

impl ModuleDef for TimersModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("setTimeout")?;
        declare.declare("clearTimeout")?;
        declare.declare("setInterval")?;
        declare.declare("clearInterval")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
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

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    #[allow(clippy::arc_with_non_send_sync)]
    let timeouts = Arc::new(Mutex::new(Vec::<Timeout>::new()));
    let timeouts2 = timeouts.clone();
    let timeouts3 = timeouts.clone();
    let timeouts4 = timeouts.clone();

    globals.set(
        "setTimeout",
        Func::from(move |ctx, cb, delay| set_timeout_interval(&ctx, &timeouts, cb, delay, false)),
    )?;

    globals.set(
        "setInterval",
        Func::from(move |ctx, cb, delay| set_timeout_interval(&ctx, &timeouts2, cb, delay, true)),
    )?;

    globals.set(
        "clearTimeout",
        Func::from(move |id: usize| clear_timeout_interval(&timeouts3, id)),
    )?;

    globals.set(
        "clearInterval",
        Func::from(move |id: usize| clear_timeout_interval(&timeouts4, id)),
    )?;

    globals.set("setImmediate", Func::from(set_immediate))?;

    Ok(())
}

#[inline(always)]
fn poll_timers<'js>(ctx: &Ctx<'js>, timeouts: Arc<Mutex<Vec<Timeout<'js>>>>) -> Result<()> {
    TIME_POLL_ACTIVE.store(true, Ordering::Relaxed);

    ctx.spawn_exit(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(1));
        let mut to_call = Some(Vec::new());
        let mut exit_after_next_tick = false;
        loop {
            interval.tick().await;

            let mut call_vec = to_call.take().unwrap(); //avoid creating a new vec
            let current_time = get_current_time_millis();
            let mut had_items = false;

            timeouts.lock().unwrap().retain_mut(|timeout| {
                had_items = true;
                exit_after_next_tick = false;
                if current_time > timeout.timeout {
                    if !timeout.repeating {
                        //do not clone if not not repeating
                        call_vec.push(timeout.cb.take());
                        return false;
                    }
                    timeout.timeout = current_time + timeout.delay;
                    call_vec.push(timeout.cb.clone());
                }
                true
            });

            for cb in call_vec.iter_mut() {
                if let Some(cb) = cb.take() {
                    cb.call::<(), ()>(())?;
                };
            }

            call_vec.clear();
            to_call.replace(call_vec);

            if !had_items {
                if exit_after_next_tick {
                    break;
                }
                exit_after_next_tick = true;
            }
        }
        TIME_POLL_ACTIVE.store(false, Ordering::Relaxed);

        Ok(())
    })?;
    Ok(())
}
