// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use rquickjs::{Ctx, JsLifetime, Result, Value};

use crate::{AsyncResourceKind, PromiseHookController};

pub type AsyncResourceRegister =
    for<'js> fn(&Ctx<'js>, Value<'js>, AsyncResourceKind, u64, u64) -> Result<()>;

#[derive(Clone, Copy)]
pub enum AsyncContextEvent {
    Init {
        async_id: u64,
        trigger_id: u64,
    },
    Finalized {
        kind: AsyncResourceKind,
        token_id: u64,
        async_id: u64,
    },
}

pub type AsyncContextObserver = for<'js> fn(&Ctx<'js>, AsyncContextEvent) -> Result<()>;

pub struct AsyncContextBridge {
    pub promise_hook_controller: Rc<PromiseHookController>,
    pub async_resource_register: AsyncResourceRegister,
    observers: RefCell<Vec<AsyncContextObserver>>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncContextBridge {
    type Changed<'to> = AsyncContextBridge;
}

impl AsyncContextBridge {
    pub fn new(
        promise_hook_controller: Rc<PromiseHookController>,
        async_resource_register: AsyncResourceRegister,
    ) -> Self {
        Self {
            promise_hook_controller,
            async_resource_register,
            observers: RefCell::new(Vec::new()),
        }
    }

    pub fn add_observer(&self, observer: AsyncContextObserver) {
        self.observers.borrow_mut().push(observer);
    }

    pub fn notify_observers(&self, ctx: &Ctx<'_>, event: AsyncContextEvent) -> Result<()> {
        let observers = self.observers.borrow().clone();
        for observer in observers {
            observer(ctx, event)?;
        }
        Ok(())
    }
}

pub fn is_tracking_active(ctx: &Ctx<'_>) -> bool {
    ctx.userdata::<AsyncContextBridge>()
        .is_some_and(|bridge| bridge.promise_hook_controller.is_active())
}

pub fn promise_hook_controller(ctx: &Ctx<'_>) -> Option<Rc<PromiseHookController>> {
    ctx.userdata::<AsyncContextBridge>()
        .map(|bridge| bridge.promise_hook_controller.clone())
}

pub fn add_async_context_observer(ctx: &Ctx<'_>, observer: AsyncContextObserver) -> Result<()> {
    let bridge = ctx.userdata::<AsyncContextBridge>().ok_or_else(|| {
        rquickjs::Exception::throw_internal(ctx, "Async context is not initialized")
    })?;
    bridge.add_observer(observer);
    Ok(())
}

pub fn notify_async_context_observers(ctx: &Ctx<'_>, event: AsyncContextEvent) -> Result<()> {
    if let Some(bridge) = ctx.userdata::<AsyncContextBridge>() {
        bridge.notify_observers(ctx, event)?;
    }
    Ok(())
}
