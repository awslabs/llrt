// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub use llrt_modules::events::{
    EmitError, Emitter, EventEmitter, EventKey, EventList, EventsModule,
};
use rquickjs::{Class, Ctx, Result};

use self::{abort_controller::AbortController, abort_signal::AbortSignal};

pub mod abort_controller;
pub mod abort_signal;

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    llrt_modules::events::init(ctx)?;

    let globals = ctx.globals();

    Class::<AbortController>::define(&globals)?;
    Class::<AbortSignal>::define(&globals)?;

    AbortSignal::add_event_emitter_prototype(ctx)?;

    Ok(())
}
