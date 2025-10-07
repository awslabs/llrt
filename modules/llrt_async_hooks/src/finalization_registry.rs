// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use llrt_utils::result::ResultExt;
use rquickjs::{prelude::Func, Ctx, Result, Value};
use tracing::trace;

use super::{remove_id_map, update_current_id, AsyncHookState};

pub(crate) fn init_finalization_registry(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    global.set(
        "__invokeFinalizationHook",
        Func::from(invoke_finalization_hook),
    )?;

    let _: () = ctx.eval(
        r#"
        globalThis.asyncFinalizationRegistry = (() => {
            const registry = new FinalizationRegistry(__invokeFinalizationHook);
            return {
                register(target, heldValue) {
                    registry.register(target, heldValue);
                }
            };
        })();
        "#,
    )?;

    global.remove("__invokeFinalizationHook")?;

    Ok(())
}

fn invoke_finalization_hook<'js>(ctx: Ctx<'js>, uid: Value<'js>) -> Result<()> {
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(&ctx)?;
    let state = bind_state.borrow();
    if state.hooks.is_empty() {
        return Ok(());
    }

    let uid = uid.as_number().unwrap() as usize;

    let current_id = remove_id_map(&ctx, uid)?;
    if current_id.0 == 0 {
        return Ok(());
    }

    update_current_id(&ctx, current_id)?;
    trace!("Destroy[{}](async_id, trigger_id): {:?}", uid, current_id);

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
