// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use llrt_hooking::{
    is_hooking_enabled, register_finalization_registry, AsyncHookBridge, AsyncTokenKind,
};
use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    promise::PromiseHookType,
    runtime::PromiseHook,
    BigInt, Class, Constructor, Ctx, Function, Object, Persistent, Result, Value,
};
use tracing::trace;

mod async_context;
mod async_hooks;
mod async_local_storage;
mod finalization_registry;
mod tracking;

use self::async_context::{
    cleanup as cleanup_async_context, enter_async_scope, exit_async_scope, get_promise_id,
    insert_promise_id, next_native_id, register_async_resource, AsyncResourceState,
};
use self::async_hooks::{
    create_hook, current_id, dispatch_callback, dispatch_init, execution_async_id,
    execution_async_resource, trigger_async_id, AsyncHookRegistry,
};
use self::async_local_storage::{
    bind, cleanup as cleanup_async_local_storage, propagate_async_local_storage, snapshot,
    AsyncLocalStorage, AsyncLocalStorageRegistry,
};
use self::finalization_registry::create_finalization_registry;
use self::tracking::{
    has, tracking_mask, AsyncTrackingState, BEFORE_OR_ALS_MASK, FINALIZATION_MASK, LEGACY_MASK,
    TRACK_ALS, TRACK_RESOLVE,
};

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
    let global = ctx.globals();

    ctx.store_userdata(RefCell::new(AsyncTrackingState::default()))
        .or_throw(ctx)?;
    ctx.store_userdata(RefCell::new(AsyncHookRegistry::default()))
        .or_throw(ctx)?;
    ctx.store_userdata(RefCell::new(AsyncLocalStorageRegistry::default()))
        .or_throw(ctx)?;

    let weak_map: Constructor = global.get(PredefinedAtom::WeakMap)?;
    let promise_map: Object = weak_map.construct(())?;
    let promise_map = Persistent::save(ctx, promise_map);
    ctx.store_userdata(RefCell::new(AsyncResourceState::new(promise_map)))
        .or_throw(ctx)?;

    let (registory, register) = create_finalization_registry(ctx)?;
    let invoke_async_hook = Function::new(ctx.clone(), invoke_native_async_hook)?;
    let register_async_resource = Function::new(ctx.clone(), register_async_resource)?;
    ctx.store_userdata(AsyncHookBridge {
        finalization_registry: Persistent::save(ctx, registory),
        finalization_register: Persistent::save(ctx, register),
        async_resource_register: Persistent::save(ctx, register_async_resource),
        async_hook_invoker: Persistent::save(ctx, invoke_async_hook),
    })
    .or_throw(ctx)?;

    Ok(())
}

pub fn cleanup(ctx: &Ctx<'_>) -> Result<()> {
    cleanup_async_context(ctx);
    if let Some(registry) = ctx.userdata::<RefCell<AsyncLocalStorageRegistry>>() {
        cleanup_async_local_storage(&registry.borrow().storages);
    }
    if let Some(registry) = ctx.userdata::<RefCell<AsyncHookRegistry>>() {
        registry.borrow_mut().cleanup();
    }
    Ok(())
}

pub(crate) enum AsyncTarget<'js> {
    Native {
        id: u64,
        trigger_id: u64,
    },
    Promise {
        promise: Value<'js>,
        parent: Value<'js>,
    },
}

fn invoke_native_async_hook(
    ctx: Ctx<'_>,
    type_: String,
    async_type: String,
    async_id: u64,
    trigger_id: u64,
) -> Result<Object<'_>> {
    let type_ = match type_.as_ref() {
        "init" => PromiseHookType::Init,
        "before" => PromiseHookType::Before,
        "after" => PromiseHookType::After,
        "resolve" => PromiseHookType::Resolve,
        _ => return Object::new(ctx),
    };
    let (async_id, trigger_id) = invoke_async_hook(
        &ctx,
        type_,
        async_type.as_ref(),
        AsyncTarget::Native {
            id: async_id,
            trigger_id,
        },
    )?;
    let result = Object::new(ctx.clone())?;
    result.set("asyncId", BigInt::from_u64(ctx.clone(), async_id)?)?;
    result.set("triggerId", BigInt::from_u64(ctx.clone(), trigger_id)?)?;
    Ok(result)
}

pub fn promise_hook_tracker() -> PromiseHook {
    Box::new(
        |ctx: Ctx<'_>, type_: PromiseHookType, promise: Value<'_>, parent: Value<'_>| {
            if !is_hooking_enabled() {
                return;
            }

            let _ = invoke_async_hook(
                &ctx,
                type_,
                "PROMISE",
                AsyncTarget::Promise { promise, parent },
            );
        },
    )
}

fn invoke_async_hook<'js>(
    ctx: &Ctx<'js>,
    type_: PromiseHookType,
    async_type: &str,
    target: AsyncTarget<'js>,
) -> Result<(u64, u64)> {
    let tracking = tracking_mask(ctx)?;
    if !event_requires_tracking(tracking, type_) {
        return Ok((0, 0));
    }
    match type_ {
        PromiseHookType::Init => {
            let current_id = match &target {
                AsyncTarget::Native { .. } => next_native_id(ctx)?,
                AsyncTarget::Promise { promise, parent } => {
                    let current_id = insert_promise_id(ctx, promise, parent)?;
                    if has(tracking, FINALIZATION_MASK) {
                        let _ = register_finalization_registry(
                            ctx,
                            promise.clone(),
                            AsyncTokenKind::Promise,
                            current_id.0,
                            current_id.1,
                        );
                    }
                    current_id
                },
            };
            if has(tracking, TRACK_ALS) {
                propagate_async_local_storage(ctx, current_id.0, current_id.1)?;
            }
            trace!("Init(async_id, trigger_id): {:?}", current_id);

            if has(tracking, LEGACY_MASK) {
                dispatch_init(ctx, current_id.0, async_type, current_id.1);
            }
            Ok(current_id)
        },
        PromiseHookType::Before | PromiseHookType::After | PromiseHookType::Resolve => {
            let current_id = match &target {
                AsyncTarget::Native { id, trigger_id } => (*id, *trigger_id),
                AsyncTarget::Promise { promise, .. } => get_promise_id(ctx, promise)?,
            };
            if current_id.0 == 0 {
                return Ok((0, 0));
            }

            let _type = match type_ {
                PromiseHookType::Before => "Before",
                PromiseHookType::After => "After",
                PromiseHookType::Resolve => "Resolve",
                _ => unreachable!(),
            };
            trace!("{}(async_id, trigger_id): {:?}", _type, current_id);
            if type_ == PromiseHookType::Before {
                enter_async_scope(ctx, current_id)?;
            }

            dispatch_callback(ctx, type_, current_id.0);

            if type_ == PromiseHookType::After {
                exit_async_scope(ctx, current_id.0)?;
            }

            Ok(current_id)
        },
    }
}

pub(crate) fn event_requires_tracking(tracking: u8, type_: PromiseHookType) -> bool {
    match type_ {
        PromiseHookType::Init => tracking != 0,
        PromiseHookType::Before | PromiseHookType::After => has(tracking, BEFORE_OR_ALS_MASK),
        PromiseHookType::Resolve => has(tracking, TRACK_RESOLVE),
    }
}
