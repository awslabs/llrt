// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::{hash_map::Entry, HashMap},
    ptr::NonNull,
    rc::Rc,
    sync::Mutex,
    time::Duration,
};

use once_cell::sync::Lazy;
use rquickjs::{qjs, Ctx, Exception, Function, Object, Persistent, Result};
use tokio::{sync::Notify, time::Instant};

static RT_TIMER_STATE: Lazy<Mutex<HashMap<usize, RuntimeTimerState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub(crate) fn timer_state() -> std::sync::MutexGuard<'static, HashMap<usize, RuntimeTimerState>> {
    RT_TIMER_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) struct RuntimeTimerState {
    pub(crate) next_timer_id: usize,
    pub(crate) timers: Vec<Timeout>,
    pub(crate) running: bool,
    pub(crate) shutting_down: bool,
    pub(crate) deadline: Instant,
    pub(crate) notify: Rc<Notify>,
}

impl RuntimeTimerState {
    pub(crate) fn new() -> Self {
        let deadline = Instant::now() + Duration::from_secs(86400 * 365 * 30);
        Self {
            next_timer_id: 0,
            timers: Default::default(),
            deadline,
            running: false,
            shutting_down: false,
            notify: Default::default(),
        }
    }
}

unsafe impl Send for RuntimeTimerState {}

#[derive(Clone)]
pub(crate) struct AsyncResource {
    pub(crate) resource: Persistent<Object<'static>>,
    pub(crate) async_id: u64,
    pub(crate) trigger_id: u64,
}

pub(crate) struct Timeout {
    pub(crate) callback: Option<Persistent<Function<'static>>>,
    pub(crate) async_resource: Option<AsyncResource>,
    pub(crate) deadline: Instant,
    pub(crate) raw_ctx: NonNull<qjs::JSContext>,
    pub(crate) id: usize,
    pub(crate) repeating: bool,
    pub(crate) interval: u64,
}

pub fn initialize(ctx: &Ctx<'_>) -> Result<Option<usize>> {
    let rt_ptr = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let mut rt_timers = timer_state();
    match rt_timers.entry(rt_ptr as usize) {
        Entry::Vacant(entry) => {
            entry.insert(RuntimeTimerState::new());
        },
        Entry::Occupied(entry) if entry.get().shutting_down => {
            return Err(Exception::throw_internal(
                ctx,
                "Async runtime is still shutting down",
            ));
        },
        Entry::Occupied(_) => return Ok(None),
    }
    drop(rt_timers);
    Ok(Some(rt_ptr as usize))
}

/// Requests timer scheduler shutdown for the current QuickJS runtime.
///
/// Shutdown is cooperative: the scheduler removes its state on its next
/// poll. Callers must not reinitialize the runtime until that shutdown has
/// completed.
pub fn graceful_shutdown(ctx: &Ctx<'_>) -> Result<()> {
    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    let mut rt_timers = timer_state();
    let Some(state) = rt_timers.get_mut(&(rt as usize)) else {
        return Ok(());
    };

    state.shutting_down = true;
    state.timers.clear();
    state.notify.notify_one();
    if !state.running {
        rt_timers.remove(&(rt as usize));
    }
    Ok(())
}

pub(crate) fn finish_shutdown(rt: usize) {
    timer_state().remove(&rt);
}
