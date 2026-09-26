// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_hooking::AsyncTokenKind;
use rquickjs::{prelude::Func, Constructor, Ctx, Function, Object, Result, Value};

use crate::async_context::{get_id_from_token, parse_async_token, remove_native_resource};
use crate::async_local_storage::remove_async_local_storage;

pub(crate) fn create_finalization_registry<'a>(
    ctx: &Ctx<'a>,
) -> Result<(Object<'a>, Function<'a>)> {
    let global = ctx.globals();
    let constructor: Constructor = global.get("FinalizationRegistry")?;
    let registry: Object = constructor.construct((Func::from(invoke_finalization_hook),))?;
    let register: Function = registry.get("register")?;
    Ok((registry, register))
}

fn invoke_finalization_hook<'js>(ctx: Ctx<'js>, uid: Value<'js>) -> Result<()> {
    let (kind, token_id) = parse_async_token(&ctx, &uid)?;

    let current_id = get_id_from_token(&uid)?;
    if current_id.0 == 0 {
        return Ok(());
    }

    clear_finalized_resource(&ctx, kind, token_id, current_id.0)?;
    Ok(())
}

fn clear_finalized_resource(
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
