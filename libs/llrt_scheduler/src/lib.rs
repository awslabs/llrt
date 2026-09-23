// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
mod microtask;
mod scheduler;
mod state;
mod timers;

use rquickjs::{prelude::Func, Ctx, Result};

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals()
        .set("queueMicrotask", Func::from(microtask::queue_microtask))?;
    Ok(())
}

pub use scheduler::tick;
pub use state::{graceful_shutdown, initialize};
pub use timers::{cancel_timer, schedule_immediate, schedule_interval, schedule_timeout};
