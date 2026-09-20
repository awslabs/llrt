// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use llrt_async_context::{
    lifecycle::promise_hook_tracker,
    state::{
        cleanup_async_context, get_current_id, get_current_resource, init_async_context_state,
    },
    AsyncContextBridge, PromiseHookController,
};
use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Function, Object, Result, Value,
};

mod async_hooks_bridge;
mod async_local_storage;
mod binding;
mod finalization_registry;

use self::async_hooks_bridge::{register_async_resource_target, AsyncHooksBridge};
use self::async_local_storage::{
    cleanup as cleanup_async_local_storage, register_async_context_observer, AsyncLocalStorage,
    AsyncLocalStorageRegistry,
};
use self::binding::{bind, snapshot};

pub(crate) fn create_hook<'js>(ctx: Ctx<'js>, _hooks_obj: Object<'js>) -> Result<Value<'js>> {
    let obj = Object::new(ctx.clone())?;
    obj.set("enable", Function::new(ctx.clone(), || {}))?;
    obj.set("disable", Function::new(ctx.clone(), || {}))?;

    Ok(obj.into())
}

pub(crate) fn current_id() -> u64 {
    // NOTE: This method is now obsolete. Therefore, it does not return a valid value.
    // But we will define it because it is used by cls-hooked.
    0
}

pub(crate) fn execution_async_id(ctx: Ctx<'_>) -> Result<u64> {
    Ok(get_current_id(&ctx)?.0)
}

pub(crate) fn trigger_async_id(ctx: Ctx<'_>) -> Result<u64> {
    Ok(get_current_id(&ctx)?.1)
}

pub(crate) fn execution_async_resource(ctx: Ctx<'_>) -> Result<Object<'_>> {
    get_current_resource(ctx)
}

pub struct AsyncHooksModule;

impl ModuleDef for AsyncHooksModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createHook")?;
        declare.declare("currentId")?;
        declare.declare("executionAsyncId")?;
        declare.declare("executionAsyncResource")?;
        declare.declare("triggerAsyncId")?;
        declare.declare("AsyncLocalStorage")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("createHook", Func::from(create_hook))?;
            default.set("currentId", Func::from(current_id))?;
            default.set("executionAsyncId", Func::from(execution_async_id))?;
            default.set(
                "executionAsyncResource",
                Func::from(execution_async_resource),
            )?;
            default.set("triggerAsyncId", Func::from(trigger_async_id))?;

            Class::<AsyncLocalStorage>::define(default)?;
            let constructor: Function = default.get("AsyncLocalStorage")?;
            constructor.set("bind", Func::from(bind))?;
            constructor.set("snapshot", Func::from(snapshot))?;

            Ok(())
        })?;
        Ok(())
    }
}

impl From<AsyncHooksModule> for ModuleInfo<AsyncHooksModule> {
    fn from(val: AsyncHooksModule) -> Self {
        ModuleInfo {
            name: "async_hooks",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    ctx.store_userdata(RefCell::new(AsyncLocalStorageRegistry::default()))
        .or_throw(ctx)?;
    init_async_context_state(ctx)?;

    ctx.store_userdata(AsyncHooksBridge::new(ctx)?)
        .or_throw(ctx)?;
    ctx.store_userdata(AsyncContextBridge::new(
        Rc::new(PromiseHookController::new(promise_hook_tracker())),
        register_async_resource_target,
    ))
    .or_throw(ctx)?;
    register_async_context_observer(ctx)?;

    Ok(())
}

pub fn cleanup(ctx: &Ctx<'_>) -> Result<()> {
    shutdown_promise_hook(ctx);
    cleanup_async_context(ctx);
    if let Some(registry) = ctx.userdata::<RefCell<AsyncLocalStorageRegistry>>() {
        cleanup_async_local_storage(&registry.borrow().storages);
    }
    Ok(())
}

pub fn shutdown_promise_hook(ctx: &Ctx<'_>) {
    if let Some(controller) = llrt_async_context::promise_hook_controller(ctx) {
        controller.shutdown(ctx);
    }
}
