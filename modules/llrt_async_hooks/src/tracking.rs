// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use rquickjs::{Ctx, Exception, JsLifetime, Result};

pub(crate) const TRACK_ALS: u8 = 1 << 5;

#[derive(Default, JsLifetime)]
pub(crate) struct AsyncTrackingState {
    pub(crate) mask: u8,
}

pub(crate) fn set_bit(ctx: &Ctx<'_>, bit: u8, enabled: bool) -> Result<()> {
    let state = ctx
        .userdata::<RefCell<AsyncTrackingState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncTrackingState is not initialized"))?;
    let mut state = state.borrow_mut();
    if enabled {
        state.mask |= bit;
    } else {
        state.mask &= !bit;
    }
    Ok(())
}

pub(crate) fn has(mask: u8, bit: u8) -> bool {
    mask & bit != 0
}

pub(crate) fn tracking_mask(ctx: &Ctx<'_>) -> Result<u8> {
    let state = ctx
        .userdata::<RefCell<AsyncTrackingState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncTrackingState is not initialized"))?;
    let mask = state.borrow().mask;
    Ok(mask)
}
