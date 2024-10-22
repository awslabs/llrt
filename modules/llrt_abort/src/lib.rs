// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::new_without_default)]
#[cfg(any(
    all(feature = "sleep-timers", feature = "sleep-tokio"),
    all(not(feature = "sleep-timers"), not(feature = "sleep-tokio"))
))]
compile_error!("Either the `sleep-timers` or `sleep-tokio` feature must be enabled");

use llrt_events::Emitter;
use rquickjs::{Class, Ctx, Result};

pub use self::{abort_controller::AbortController, abort_signal::AbortSignal};

mod abort_controller;
mod abort_signal;

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<AbortController>::define(&globals)?;
    Class::<AbortSignal>::define(&globals)?;

    AbortSignal::add_event_emitter_prototype(ctx)?;

    Ok(())
}
