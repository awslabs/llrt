// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use rquickjs::{prelude::Func, Ctx, Function, Object, Result, Value};
use tracing::trace;

use super::{remove_id_map, update_current_id, AsyncHookState};

pub(crate) fn init_finalization_registry(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    global.set(
        "__invokeFinalizationHook",
        Func::from(invoke_finalization_hook),
    )?;

    let _: () = ctx.eval(
        "globalThis.asyncFinalizationRegistry = new FinalizationRegistry(__invokeFinalizationHook)",
    )?;

    global.remove("__invokeFinalizationHook")?;

    Ok(())
}

pub(crate) fn register_finalization_registry<'js>(
    ctx: &Ctx<'js>,
    target: Value<'js>,
    uid: usize,
) -> Result<()> {
    let global = ctx.globals();
    let finalization_registry: Object = global.get("asyncFinalizationRegistry")?;
    let register: Function = finalization_registry.get("register")?;
    if let Err(e) = register.call::<_, ()>((target, uid)) {
        trace!("register_finalization_registry::Error: {}", &e.to_string());
        let exception_value = ctx.catch();
        trace!("register_finalization_registry {:?}", exception_value);
    }
    Ok(())
}

fn invoke_finalization_hook<'js>(ctx: Ctx<'js>, uid: Value<'js>) -> Result<()> {
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().unwrap();
    let state = bind_state.borrow();

    if state.hooks.is_empty() {
        return Ok(());
    }

    let uid = uid.as_number().unwrap() as usize;

    let current_id = remove_id_map(&ctx, uid);
    update_current_id(&ctx, current_id);
    trace!("Destroy(async_id, trigger_id): {:?}", current_id);

    for hook in &state.hooks {
        if *hook.enabled.as_ref().borrow() {
            if let Some(func) = &hook.destroy {
                let _ = func
                    .call::<_, ()>((current_id.0,))
                    .or_else(|_| func.call::<_, ()>(()));
            }
        }
    }
    Ok(())
}
