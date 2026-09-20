// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use llrt_utils::result::ResultExt;
use rquickjs::{prelude::Func, Constructor, Ctx, Function, Object, Persistent, Result, Value};
use smallvec::SmallVec;
use tracing::trace;

use llrt_hooking::AsyncTokenKind;

use super::async_context::{
    get_current_id, get_id_from_token, parse_async_token, remove_native_resource, update_current_id,
};
use super::async_local_storage::remove_async_local_storage;
use super::AsyncHookState;

pub(crate) fn create_finalization_registry<'js>(
    ctx: &Ctx<'js>,
) -> Result<(Persistent<Object<'static>>, Persistent<Function<'static>>)> {
    let global = ctx.globals();
    let constructor: Constructor = global.get("FinalizationRegistry")?;
    let registry: Object = constructor.construct((Func::from(invoke_finalization_hook),))?;
    let register: Function = registry.get("register")?;
    Ok((
        Persistent::save(ctx, registry),
        Persistent::save(ctx, register),
    ))
}

fn invoke_finalization_hook<'js>(ctx: Ctx<'js>, uid: Value<'js>) -> Result<()> {
    let (kind, token_id) = parse_async_token(&ctx, &uid)?;
    let current_id = match kind {
        AsyncTokenKind::Native => get_id_from_token(&uid)?,
        AsyncTokenKind::Promise => get_id_from_token(&uid)?,
    };

    if current_id.0 == 0 {
        return Ok(());
    }

    cleanup_finalized_resource(&ctx, kind, token_id, current_id.0)?;
    invoke_destroy_callbacks(&ctx, kind, token_id, current_id)
}

fn cleanup_finalized_resource(
    ctx: &Ctx<'_>,
    kind: AsyncTokenKind,
    token_id: u64,
    async_id: u64,
) -> Result<()> {
    if kind == AsyncTokenKind::Native {
        remove_native_resource(ctx, token_id)?;
    }
    remove_async_local_storage(ctx, async_id)
}

fn invoke_destroy_callbacks(
    ctx: &Ctx<'_>,
    kind: AsyncTokenKind,
    token_id: u64,
    current_id: (u64, u64),
) -> Result<()> {
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
    let previous_id = get_current_id(ctx)?;
    update_current_id(ctx, current_id)?;
    trace!(
        "Destroy[{:?}:{}](async_id, trigger_id): {:?}",
        kind,
        token_id,
        current_id
    );

    let callbacks = {
        let state = bind_state.borrow();
        state
            .hooks
            .iter()
            .filter_map(|hook| hook.callbacks.destroy.clone())
            .collect::<SmallVec<[_; 2]>>()
    };
    for callback in callbacks {
        if let Err(error) = callback.call::<_, ()>((current_id.0,)) {
            trace!("async_hooks destroy callback failed: {:?}", error);
        }
    }
    update_current_id(ctx, previous_id)?;
    Ok(())
}
