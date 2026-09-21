// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use llrt_async_runtime::shutdown_state;
use llrt_hooking::{
    acquire_hooking, is_hooking_enabled, register_finalization_registry, release_hooking,
    AsyncHookBridge, AsyncTokenKind,
};
use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, This},
    promise::PromiseHookType,
    runtime::PromiseHook,
    BigInt, Class, Constructor, Ctx, Function, JsLifetime, Object, Persistent, Result, Value,
};
use smallvec::SmallVec;
use tracing::trace;

mod async_context;
mod async_local_storage;
mod finalization_registry;

use crate::async_context::{
    cleanup as cleanup_async_resource, enter_async_scope, exit_async_scope, get_current_id,
    get_current_resource, get_promise_id, insert_promise_id, next_native_id,
    register_async_resource, AsyncResourceState, AsyncTarget,
};
use crate::async_local_storage::{
    bind, cleanup_async_local_storage, propagate_async_local_storage, snapshot,
    AsyncLocalStorageWeakHandle,
};
use crate::finalization_registry::create_finalization_registry;

struct Hook<'js> {
    callbacks: Rc<HookCallbacks<'js>>,
}

struct HookCallbacks<'js> {
    init: Option<Function<'js>>,
    before: Option<Function<'js>>,
    after: Option<Function<'js>>,
    promise_resolve: Option<Function<'js>>,
    destroy: Option<Function<'js>>,
}

const TRACK_INIT: u8 = 1 << 0;
const TRACK_BEFORE: u8 = 1 << 1;
const TRACK_AFTER: u8 = 1 << 2;
const TRACK_RESOLVE: u8 = 1 << 3;
const TRACK_DESTROY: u8 = 1 << 4;
const TRACK_ALS: u8 = 1 << 5;

fn callback_mask(callbacks: &HookCallbacks<'_>) -> u8 {
    (if callbacks.init.is_some() {
        TRACK_INIT
    } else {
        0
    }) | (if callbacks.before.is_some() {
        TRACK_BEFORE
    } else {
        0
    }) | (if callbacks.after.is_some() {
        TRACK_AFTER
    } else {
        0
    }) | (if callbacks.promise_resolve.is_some() {
        TRACK_RESOLVE
    } else {
        0
    }) | (if callbacks.destroy.is_some() {
        TRACK_DESTROY
    } else {
        0
    })
}

#[derive(Default)]
struct AsyncHookState<'js> {
    hooks: Vec<Hook<'js>>,
    async_local_storages: Vec<AsyncLocalStorageWeakHandle<'js>>,
    tracking: u8,
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookState<'js> {
    type Changed<'to> = AsyncHookState<'to>;
}

impl<'js> AsyncHookState<'js> {
    fn recompute_tracking(&mut self) {
        self.tracking = self.hooks.iter().fold(
            if async_local_storage::has_active_async_local_storage(&self.async_local_storages) {
                TRACK_ALS
            } else {
                0
            },
            |mask, hook| mask | callback_mask(&hook.callbacks),
        );
    }
    fn add_hook(&mut self, callbacks: Rc<HookCallbacks<'js>>, hook_mask: u8) {
        self.hooks.push(Hook { callbacks });
        self.tracking |= hook_mask;
    }

    fn callbacks_for(&self, type_: PromiseHookType) -> SmallVec<[Function<'js>; 2]> {
        self.hooks
            .iter()
            .filter_map(|hook| match type_ {
                PromiseHookType::Init => hook.callbacks.init.clone(),
                PromiseHookType::Before => hook.callbacks.before.clone(),
                PromiseHookType::After => hook.callbacks.after.clone(),
                PromiseHookType::Resolve => hook.callbacks.promise_resolve.clone(),
            })
            .collect()
    }

    fn remove_hook(&mut self, callbacks: &Rc<HookCallbacks<'js>>) -> bool {
        let hook_count = self.hooks.len();
        self.hooks
            .retain(|hook| !Rc::ptr_eq(&hook.callbacks, callbacks));
        self.recompute_tracking();
        self.hooks.len() != hook_count
    }

    fn cleanup(&mut self) {
        for _ in &self.hooks {
            release_hooking();
        }
        cleanup_async_local_storage(&self.async_local_storages);
        self.hooks.clear();
        self.async_local_storages.clear();
        self.tracking = 0;
    }
}

fn enable_hook<'js>(
    ctx: &Ctx<'js>,
    callbacks: Rc<HookCallbacks<'js>>,
    hook_mask: u8,
) -> Result<()> {
    let state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
    let mut state = state.borrow_mut();
    state.add_hook(callbacks, hook_mask);
    acquire_hooking();
    Ok(())
}

fn disable_hook<'js>(ctx: &Ctx<'js>, callbacks: &Rc<HookCallbacks<'js>>) -> Result<()> {
    let state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
    let mut state = state.borrow_mut();
    if state.remove_hook(callbacks) {
        release_hooking();
    }
    Ok(())
}

fn create_hook<'js>(ctx: Ctx<'js>, hooks_obj: Object<'js>) -> Result<Value<'js>> {
    let callbacks = Rc::new(HookCallbacks {
        init: hooks_obj.get("init").ok(),
        before: hooks_obj.get("before").ok(),
        after: hooks_obj.get("after").ok(),
        promise_resolve: hooks_obj.get("promiseResolve").ok(),
        destroy: hooks_obj.get("destroy").ok(),
    });
    let hook_mask = callback_mask(&callbacks);
    let enabled = Rc::new(RefCell::new(false));
    let obj = Object::new(ctx.clone())?;

    let state_ctx = ctx.clone();
    let enabled_clone = enabled.clone();
    let callbacks_clone = callbacks.clone();
    obj.set(
        "enable",
        Function::new(
            ctx.clone(),
            move |this: This<Object<'js>>| -> Result<Object<'js>> {
                if !*enabled_clone.borrow() {
                    enable_hook(&state_ctx, callbacks_clone.clone(), hook_mask)?;
                    *enabled_clone.borrow_mut() = true;
                }
                Ok(this.0)
            },
        ),
    )?;

    let state_ctx = ctx.clone();
    let enabled_clone = enabled.clone();
    obj.set(
        "disable",
        Function::new(
            ctx.clone(),
            move |this: This<Object<'js>>| -> Result<Object<'js>> {
                if *enabled_clone.borrow() {
                    disable_hook(&state_ctx, &callbacks)?;
                    *enabled_clone.borrow_mut() = false;
                }
                Ok(this.0)
            },
        ),
    )?;

    Ok(obj.into())
}

fn current_id() -> u64 {
    0
}

fn execution_async_id(ctx: Ctx<'_>) -> Result<u64> {
    Ok(get_current_id(&ctx)?.0)
}

fn execution_async_resource(ctx: Ctx<'_>) -> Result<Object<'_>> {
    get_current_resource(ctx)
}

fn trigger_async_id(ctx: Ctx<'_>) -> Result<u64> {
    Ok(get_current_id(&ctx)?.1)
}

pub struct AsyncHooksModule;

impl ModuleDef for AsyncHooksModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createHook")?;
        declare.declare("AsyncLocalStorage")?;
        declare.declare("currentId")?;
        declare.declare("executionAsyncId")?;
        declare.declare("executionAsyncResource")?;
        declare.declare("triggerAsyncId")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("createHook", Func::from(create_hook))?;
            Class::<async_local_storage::AsyncLocalStorage>::define(default)?;
            let constructor: Function = default.get("AsyncLocalStorage")?;
            constructor.set("bind", Func::from(bind))?;
            constructor.set("snapshot", Func::from(snapshot))?;
            default.set("currentId", Func::from(current_id))?;
            default.set("executionAsyncId", Func::from(execution_async_id))?;
            default.set(
                "executionAsyncResource",
                Func::from(execution_async_resource),
            )?;
            default.set("triggerAsyncId", Func::from(trigger_async_id))?;
            Ok(())
        })?;
        Ok(())
    }
}

impl From<AsyncHooksModule> for ModuleInfo<AsyncHooksModule> {
    fn from(module: AsyncHooksModule) -> Self {
        ModuleInfo {
            name: "async_hooks",
            module,
        }
    }
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

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    let _ = ctx.store_userdata(RefCell::new(AsyncHookState::default()));
    let weak_map: Constructor = global.get(PredefinedAtom::WeakMap)?;
    let promise_map: Object = weak_map.construct(())?;
    let promise_map = Persistent::save(ctx, promise_map);
    let _ = ctx.store_userdata(RefCell::new(AsyncResourceState::new(promise_map)));

    let (registry, register) = create_finalization_registry(ctx)?;
    let invoke_async_hook = Function::new(ctx.clone(), invoke_native_async_hook)?;
    let register_async_resource = Function::new(ctx.clone(), register_async_resource)?;
    let _ = ctx.store_userdata(AsyncHookBridge {
        registry,
        register,
        register_async_resource: Persistent::save(ctx, register_async_resource),
        invoke_async_hook: Persistent::save(ctx, invoke_async_hook),
    });

    Ok(())
}

pub fn promise_hook_tracker() -> PromiseHook {
    Box::new(
        |ctx: Ctx<'_>, type_: PromiseHookType, promise: Value<'_>, parent: Value<'_>| {
            if !is_hooking_enabled() {
                return;
            }
            let tracking = tracking_mask(&ctx);
            if !event_requires_tracking(tracking, type_) {
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

fn tracking_mask(ctx: &Ctx<'_>) -> u8 {
    let Some(state) = ctx.userdata::<RefCell<AsyncHookState>>() else {
        return 0;
    };
    let tracking = state.borrow().tracking;
    tracking
}

fn event_requires_tracking(tracking: u8, type_: PromiseHookType) -> bool {
    match type_ {
        PromiseHookType::Init => tracking != 0,
        PromiseHookType::Before | PromiseHookType::After => {
            tracking & (TRACK_BEFORE | TRACK_AFTER | TRACK_ALS) != 0
        },
        PromiseHookType::Resolve => tracking & TRACK_RESOLVE != 0,
    }
}

pub fn cleanup(ctx: &Ctx<'_>) -> Result<()> {
    shutdown_state(ctx)?;
    cleanup_async_resource(ctx);
    if let Some(state) = ctx.userdata::<RefCell<AsyncHookState>>() {
        state.borrow_mut().cleanup();
    }
    let _ = ctx.remove_userdata::<RefCell<AsyncResourceState>>();
    let _ = ctx.remove_userdata::<RefCell<AsyncHookState>>();
    let _ = ctx.remove_userdata::<AsyncHookBridge>();
    Ok(())
}

fn invoke_async_hook<'js>(
    ctx: &Ctx<'js>,
    type_: PromiseHookType,
    async_type: &str,
    target: AsyncTarget<'js>,
) -> Result<(u64, u64)> {
    let tracking = tracking_mask(ctx);
    if !event_requires_tracking(tracking, type_) {
        return Ok((0, 0));
    }
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;

    match type_ {
        PromiseHookType::Init => {
            let current_id = match &target {
                AsyncTarget::Native { .. } => next_native_id(ctx)?,
                AsyncTarget::Promise { promise, parent } => {
                    let current_id = insert_promise_id(ctx, promise, parent)?;
                    if tracking & (TRACK_DESTROY | TRACK_ALS) != 0 {
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
            propagate_async_local_storage(ctx, current_id.0, current_id.1)?;
            trace!("Init(async_id, trigger_id): {:?}", current_id);

            let callbacks = bind_state.borrow().callbacks_for(PromiseHookType::Init);
            for callback in callbacks {
                if let Err(error) = callback.call::<_, ()>((current_id.0, async_type, current_id.1))
                {
                    trace!("async_hooks init callback failed: {:?}", error);
                }
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

            let callbacks = bind_state.borrow().callbacks_for(type_);
            for callback in callbacks {
                if let Err(error) = callback.call::<_, ()>((current_id.0,)) {
                    trace!("async_hooks callback failed: {:?}", error);
                }
            }

            if type_ == PromiseHookType::After {
                exit_async_scope(ctx, current_id.0)?;
            }

            Ok(current_id)
        },
    }
}
