// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use rquickjs::{Ctx, Exception, JsLifetime, Result};

pub(crate) const TRACK_INIT: u8 = 1 << 0;
pub(crate) const TRACK_BEFORE: u8 = 1 << 1;
pub(crate) const TRACK_AFTER: u8 = 1 << 2;
pub(crate) const TRACK_RESOLVE: u8 = 1 << 3;
pub(crate) const TRACK_DESTROY: u8 = 1 << 4;
pub(crate) const TRACK_ALS: u8 = 1 << 5;
pub(crate) const LEGACY_MASK: u8 =
    TRACK_INIT | TRACK_BEFORE | TRACK_AFTER | TRACK_RESOLVE | TRACK_DESTROY;
pub(crate) const FINALIZATION_MASK: u8 = TRACK_DESTROY | TRACK_ALS;
pub(crate) const BEFORE_OR_ALS_MASK: u8 = TRACK_BEFORE | TRACK_AFTER | TRACK_ALS;

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

pub(crate) fn add(mask: &mut u8, bits: u8) {
    *mask |= bits;
}

pub(crate) fn callback_mask(
    init: bool,
    before: bool,
    after: bool,
    resolve: bool,
    destroy: bool,
) -> u8 {
    let mut mask = 0;
    if init {
        mask |= TRACK_INIT;
    }
    if before {
        mask |= TRACK_BEFORE;
    }
    if after {
        mask |= TRACK_AFTER;
    }
    if resolve {
        mask |= TRACK_RESOLVE;
    }
    if destroy {
        mask |= TRACK_DESTROY;
    }
    mask
}

pub(crate) fn set_legacy_mask(ctx: &Ctx<'_>, mask: u8) -> Result<()> {
    let state = ctx
        .userdata::<RefCell<AsyncTrackingState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncTrackingState is not initialized"))?;
    let mut state = state.borrow_mut();
    state.mask = (state.mask & TRACK_ALS) | mask;
    Ok(())
}

pub(crate) fn tracking_mask(ctx: &Ctx<'_>) -> Result<u8> {
    let state = ctx
        .userdata::<RefCell<AsyncTrackingState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncTrackingState is not initialized"))?;
    let mask = state.borrow().mask;
    Ok(mask)
}
