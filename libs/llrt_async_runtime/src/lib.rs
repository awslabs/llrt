// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod microtask;
mod scheduler;
mod state;
mod timers;

use llrt_hooking::ProviderType;
use rquickjs::{
    prelude::{Func, Opt},
    Ctx, Result,
};

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let rt_ptr = state::init_state(ctx)?;
    if rt_ptr.is_none() {
        return Ok(());
    }

    let result = (|| {
        let globals = ctx.globals();
        globals.set(
            "setTimeout",
            Func::from(move |ctx, cb, delay: Opt<f64>| {
                let delay = delay.unwrap_or(0.).max(0.) as u64;
                timers::set_timeout_interval(&ctx, cb, delay, ProviderType::Timeout)
            }),
        )?;
        globals.set(
            "setInterval",
            Func::from(move |ctx, cb, delay: Opt<f64>| {
                let delay = delay.unwrap_or(0.).max(0.) as u64;
                timers::set_timeout_interval(&ctx, cb, delay, ProviderType::Interval)
            }),
        )?;
        globals.set("clearTimeout", Func::from(timers::clear_timeout_interval))?;
        globals.set("clearInterval", Func::from(timers::clear_timeout_interval))?;
        globals.set(
            "setImmediate",
            Func::from(move |ctx, cb| {
                timers::set_timeout_interval(&ctx, cb, 0, ProviderType::Immediate)
            }),
        )?;
        globals.set("queueMicrotask", Func::from(microtask::queue_microtask))?;
        Ok(())
    })();

    if let (Err(_), Some(rt_ptr)) = (&result, rt_ptr) {
        state::remove_state(rt_ptr);
    }
    result
}

pub use microtask::queue_microtask;
pub use scheduler::{cleanup, poll_timers, ExecutingTimer};
pub use timers::{clear_timeout_interval, set_timeout, set_timeout_interval};
