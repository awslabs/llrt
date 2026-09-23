// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use llrt_hooking::{acquire_hooking, release_hooking, AsyncTokenKind};
use llrt_utils::result::ResultExt;
use rquickjs::{
    prelude::This, promise::PromiseHookType, Ctx, Exception, Function, JsLifetime, Object, Result,
    Value,
};
use smallvec::SmallVec;
use tracing::trace;

use crate::async_context::{get_current_id, get_current_resource, update_current_id};
use crate::tracking::{add, callback_mask, set_legacy_mask};

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

#[derive(Default)]
pub(crate) struct AsyncHookRegistry<'js> {
    pub(crate) hooks: Vec<Hook<'js>>,
    callbacks: Vec<Rc<HookCallbacks<'js>>>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookRegistry<'js> {
    type Changed<'to> = AsyncHookRegistry<'to>;
}

impl<'js> AsyncHookRegistry<'js> {
    fn register_callbacks(&mut self, callbacks: Rc<HookCallbacks<'js>>) -> usize {
        let id = self.callbacks.len();
        self.callbacks.push(callbacks);
        id
    }

    fn add_hook(&mut self, id: usize) {
        let callbacks = self
            .callbacks
            .get(id)
            .cloned()
            .expect("async hook callbacks must be registered");
        self.hooks.push(Hook { id, callbacks });
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
        self.hooks.len() != hook_count
    }

    fn legacy_mask(&self) -> u8 {
        self.hooks.iter().fold(0, |mut mask, hook| {
            add(&mut mask, callback_mask_for(&hook.callbacks));
            mask
        })
    }

    pub(crate) fn cleanup(&mut self) {
        for _ in &self.hooks {
            release_hooking();
        }
        self.hooks.clear();
        self.callbacks.clear();
    }
}

fn enable_hook<'js>(ctx: &Ctx<'js>, id: usize) -> Result<()> {
    let registry = ctx.userdata::<RefCell<AsyncHookRegistry>>().or_throw(ctx)?;
    let mut registry = registry.borrow_mut();
    registry.add_hook(id);
    set_legacy_mask(ctx, registry.legacy_mask())?;
    acquire_hooking();
    Ok(())
}

fn disable_hook<'js>(ctx: &Ctx<'js>, id: usize) -> Result<()> {
    let registry = ctx.userdata::<RefCell<AsyncHookRegistry>>().or_throw(ctx)?;
    let mut registry = registry.borrow_mut();
    if registry.remove_hook(id) {
        set_legacy_mask(ctx, registry.legacy_mask())?;
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
    let hook_id = {
        let registry = ctx
            .userdata::<RefCell<AsyncHookRegistry>>()
            .or_throw(&ctx)?;
        let hook_id = registry.borrow_mut().register_callbacks(callbacks);
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
                    enable_hook(state_ctx, hook_id)?;
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

pub(crate) fn dispatch_init(ctx: &Ctx<'_>, async_id: u64, async_type: &str, trigger_id: u64) {
    let Some(registry) = ctx.userdata::<RefCell<AsyncHookRegistry>>() else {
        return;
    };
    let callbacks = registry.borrow().callbacks_for(PromiseHookType::Init);
    for callback in callbacks {
        if let Err(error) = callback.call::<_, ()>((async_id, async_type, trigger_id)) {
            trace!("async_hooks init callback failed: {:?}", error);
        }
    }
}

pub(crate) fn dispatch_callback(ctx: &Ctx<'_>, type_: PromiseHookType, async_id: u64) {
    let Some(registry) = ctx.userdata::<RefCell<AsyncHookRegistry>>() else {
        return;
    };
    let callbacks = registry.borrow().callbacks_for(type_);
    for callback in callbacks {
        if let Err(error) = callback.call::<_, ()>((async_id,)) {
            trace!("async_hooks callback failed: {:?}", error);
        }
    }
}

pub(crate) fn dispatch_destroy(
    ctx: &Ctx<'_>,
    kind: AsyncTokenKind,
    token_id: u64,
    current_id: (u64, u64),
) -> Result<()> {
    let registry = ctx
        .userdata::<RefCell<AsyncHookRegistry>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncHookRegistry is not initialized"))?;
    let previous_id = get_current_id(ctx)?;
    update_current_id(ctx, current_id)?;
    trace!(
        "Destroy[{:?}:{}](async_id, trigger_id): {:?}",
        kind,
        token_id,
        current_id
    );

    let callbacks = registry
        .borrow()
        .hooks
        .iter()
        .filter_map(|hook| hook.callbacks.destroy.clone())
        .collect::<SmallVec<[_; 2]>>();
    for callback in callbacks {
        if let Err(error) = callback.call::<_, ()>((current_id.0,)) {
            trace!("async_hooks destroy callback failed: {:?}", error);
        }
    }
    update_current_id(ctx, previous_id)
}

fn callback_mask_for(callbacks: &HookCallbacks<'_>) -> u8 {
    callback_mask(
        callbacks.init.is_some(),
        callbacks.before.is_some(),
        callbacks.after.is_some(),
        callbacks.promise_resolve.is_some(),
        callbacks.destroy.is_some(),
    )
}
