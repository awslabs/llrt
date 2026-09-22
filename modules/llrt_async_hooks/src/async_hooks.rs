// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use llrt_hooking::{acquire_hooking, release_hooking};
use llrt_utils::result::ResultExt;
use rquickjs::{
    prelude::This, promise::PromiseHookType, Ctx, Function, JsLifetime, Object, Result, Value,
};
use smallvec::SmallVec;

use crate::async_context::{get_current_id, get_current_resource};
use crate::async_local_storage::{cleanup_async_local_storage, AsyncLocalStorageWeakHandle};

const TRACK_INIT: u8 = 1 << 0;
const TRACK_BEFORE: u8 = 1 << 1;
const TRACK_AFTER: u8 = 1 << 2;
const TRACK_RESOLVE: u8 = 1 << 3;
pub(crate) const TRACK_DESTROY: u8 = 1 << 4;
pub(crate) const TRACK_ALS: u8 = 1 << 5;

pub(crate) struct Hook<'js> {
    pub(crate) id: usize,
    pub(crate) callbacks: Rc<HookCallbacks<'js>>,
}

pub(crate) struct HookCallbacks<'js> {
    pub(crate) init: Option<Function<'js>>,
    pub(crate) before: Option<Function<'js>>,
    pub(crate) after: Option<Function<'js>>,
    pub(crate) promise_resolve: Option<Function<'js>>,
    pub(crate) destroy: Option<Function<'js>>,
}

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
pub(crate) struct AsyncHookState<'js> {
    pub(crate) hooks: Vec<Hook<'js>>,
    registered_callbacks: Vec<Rc<HookCallbacks<'js>>>,
    pub(crate) async_local_storages: Vec<AsyncLocalStorageWeakHandle<'js>>,
    pub(crate) tracking: u8,
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookState<'js> {
    type Changed<'to> = AsyncHookState<'to>;
}

impl<'js> AsyncHookState<'js> {
    fn recompute_tracking(&mut self) {
        self.tracking = self.hooks.iter().fold(
            if crate::async_local_storage::has_active_async_local_storage(
                &self.async_local_storages,
            ) {
                TRACK_ALS
            } else {
                0
            },
            |mask, hook| mask | callback_mask(&hook.callbacks),
        );
    }

    fn register_callbacks(&mut self, callbacks: Rc<HookCallbacks<'js>>) -> usize {
        let id = self.registered_callbacks.len();
        self.registered_callbacks.push(callbacks);
        id
    }

    fn add_hook(&mut self, id: usize, hook_mask: u8) {
        let callbacks = self
            .registered_callbacks
            .get(id)
            .cloned()
            .expect("async hook callbacks must be registered");
        self.hooks.push(Hook { id, callbacks });
        self.tracking |= hook_mask;
    }

    pub(crate) fn callbacks_for(&self, type_: PromiseHookType) -> SmallVec<[Function<'js>; 2]> {
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

    fn remove_hook(&mut self, id: usize) -> bool {
        let hook_count = self.hooks.len();
        self.hooks.retain(|hook| hook.id != id);
        self.recompute_tracking();
        self.hooks.len() != hook_count
    }

    pub(crate) fn cleanup(&mut self) {
        for _ in &self.hooks {
            release_hooking();
        }
        cleanup_async_local_storage(&self.async_local_storages);
        self.hooks.clear();
        self.registered_callbacks.clear();
        self.async_local_storages.clear();
        self.tracking = 0;
    }
}

fn enable_hook<'js>(ctx: &Ctx<'js>, id: usize, hook_mask: u8) -> Result<()> {
    let state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
    let mut state = state.borrow_mut();
    state.add_hook(id, hook_mask);
    acquire_hooking();
    Ok(())
}

fn disable_hook<'js>(ctx: &Ctx<'js>, id: usize) -> Result<()> {
    let state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(ctx)?;
    let mut state = state.borrow_mut();
    if state.remove_hook(id) {
        release_hooking();
    }
    Ok(())
}

pub(crate) fn create_hook<'js>(ctx: Ctx<'js>, hooks_obj: Object<'js>) -> Result<Value<'js>> {
    let callbacks = Rc::new(HookCallbacks {
        init: hooks_obj.get("init").ok(),
        before: hooks_obj.get("before").ok(),
        after: hooks_obj.get("after").ok(),
        promise_resolve: hooks_obj.get("promiseResolve").ok(),
        destroy: hooks_obj.get("destroy").ok(),
    });
    let hook_mask = callback_mask(&callbacks);
    let hook_id = {
        let state = ctx.userdata::<RefCell<AsyncHookState>>().or_throw(&ctx)?;
        let hook_id = state.borrow_mut().register_callbacks(callbacks);
        hook_id
    };
    let enabled = Rc::new(RefCell::new(false));
    let obj = Object::new(ctx.clone())?;

    let enabled_clone = enabled.clone();
    obj.set(
        "enable",
        Function::new(
            ctx.clone(),
            move |this: This<Object<'js>>| -> Result<Object<'js>> {
                if !*enabled_clone.borrow() {
                    let state_ctx = this.0.ctx();
                    enable_hook(state_ctx, hook_id, hook_mask)?;
                    *enabled_clone.borrow_mut() = true;
                }
                Ok(this.0)
            },
        ),
    )?;

    let enabled_clone = enabled.clone();
    obj.set(
        "disable",
        Function::new(
            ctx.clone(),
            move |this: This<Object<'js>>| -> Result<Object<'js>> {
                if *enabled_clone.borrow() {
                    let state_ctx = this.0.ctx();
                    disable_hook(state_ctx, hook_id)?;
                    *enabled_clone.borrow_mut() = false;
                }
                Ok(this.0)
            },
        ),
    )?;

    Ok(obj.into())
}

pub(crate) fn current_id() -> u64 {
    0
}

pub(crate) fn execution_async_id(ctx: Ctx<'_>) -> Result<u64> {
    Ok(get_current_id(&ctx)?.0)
}

pub(crate) fn execution_async_resource(ctx: Ctx<'_>) -> Result<Object<'_>> {
    get_current_resource(ctx)
}

pub(crate) fn trigger_async_id(ctx: Ctx<'_>) -> Result<u64> {
    Ok(get_current_id(&ctx)?.1)
}

pub(crate) fn tracking_mask(ctx: &Ctx<'_>) -> u8 {
    let Some(state) = ctx.userdata::<RefCell<AsyncHookState>>() else {
        return 0;
    };
    let tracking = state.borrow().tracking;
    tracking
}

pub(crate) fn event_requires_tracking(tracking: u8, type_: PromiseHookType) -> bool {
    match type_ {
        PromiseHookType::Init => tracking != 0,
        PromiseHookType::Before | PromiseHookType::After => {
            tracking & (TRACK_BEFORE | TRACK_AFTER | TRACK_ALS) != 0
        },
        PromiseHookType::Resolve => tracking & TRACK_RESOLVE != 0,
    }
}
