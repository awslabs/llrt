// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::Cell,
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::NonNull,
};

use rquickjs::{promise::PromiseHookType, qjs, runtime::PromiseHook, Ctx, Value};

pub struct PromiseHookController {
    active_consumers: Cell<usize>,
    tracker: PromiseHook,
}

impl PromiseHookController {
    pub fn new(tracker: PromiseHook) -> Self {
        Self {
            active_consumers: Cell::new(0),
            tracker,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active_consumers.get() != 0
    }

    pub fn acquire(&self, ctx: &Ctx<'_>) {
        let active = self.active_consumers.get();
        self.active_consumers.set(active.saturating_add(1));
        if active == 0 {
            self.set_hook(ctx, true);
        }
    }

    pub fn release(&self, ctx: &Ctx<'_>) {
        let active = self.active_consumers.get();
        if active == 0 {
            return;
        }
        self.active_consumers.set(active - 1);
        if active == 1 {
            self.set_hook(ctx, false);
        }
    }

    pub fn shutdown(&self, ctx: &Ctx<'_>) {
        self.set_hook(ctx, false);
        self.active_consumers.set(0);
    }

    fn set_hook(&self, ctx: &Ctx<'_>, enabled: bool) {
        // SAFETY: `ctx` is used only while the caller holds the QuickJS runtime lock.
        let runtime = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
        let opaque = if enabled {
            self as *const Self as *mut c_void
        } else {
            std::ptr::null_mut()
        };
        // SAFETY: QuickJS invokes this synchronously while the runtime and controller are valid.
        unsafe {
            qjs::JS_SetPromiseHook(runtime, enabled.then_some(promise_hook_trampoline), opaque);
        }
    }
}

// SAFETY: QuickJS supplies a live context and valid values; the opaque controller remains alive while the hook is registered.
unsafe extern "C" fn promise_hook_trampoline(
    raw_ctx: *mut qjs::JSContext,
    type_: qjs::JSPromiseHookType,
    promise: qjs::JSValue,
    parent: qjs::JSValue,
    opaque: *mut c_void,
) {
    let (Some(raw_ctx), Some(controller)) = (
        NonNull::new(raw_ctx),
        NonNull::new(opaque.cast::<PromiseHookController>()),
    ) else {
        return;
    };

    let _ = catch_unwind(AssertUnwindSafe(|| {
        let ctx = unsafe { Ctx::from_raw(raw_ctx) };
        let type_ = match type_ {
            qjs::JSPromiseHookType_JS_PROMISE_HOOK_INIT => PromiseHookType::Init,
            qjs::JSPromiseHookType_JS_PROMISE_HOOK_BEFORE => PromiseHookType::Before,
            qjs::JSPromiseHookType_JS_PROMISE_HOOK_AFTER => PromiseHookType::After,
            qjs::JSPromiseHookType_JS_PROMISE_HOOK_RESOLVE => PromiseHookType::Resolve,
            _ => return,
        };
        let promise = unsafe {
            Value::from_raw(
                ctx.clone(),
                qjs::JS_DupValue(ctx.as_raw().as_ptr(), promise),
            )
        };
        let parent = unsafe {
            Value::from_raw(ctx.clone(), qjs::JS_DupValue(ctx.as_raw().as_ptr(), parent))
        };
        let controller = unsafe { controller.as_ref() };
        (controller.tracker)(ctx, type_, promise, parent);
    }));
}

#[cfg(test)]
mod tests {
    use super::PromiseHookController;
    use rquickjs::{promise::PromiseHookType, runtime::PromiseHook, Context, Ctx, Runtime, Value};
    use std::sync::{atomic::AtomicUsize, Arc};

    #[test]
    fn tracks_users_without_underflowing() {
        let tracker: PromiseHook =
            Box::new(|_: Ctx<'_>, _: PromiseHookType, _: Value<'_>, _: Value<'_>| {});
        let controller = PromiseHookController::new(tracker);
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        assert!(!controller.is_active());
        context.with(|ctx| controller.acquire(&ctx));
        context.with(|ctx| controller.acquire(&ctx));
        assert!(controller.is_active());
        context.with(|ctx| controller.release(&ctx));
        assert!(controller.is_active());
        context.with(|ctx| controller.release(&ctx));
        assert!(!controller.is_active());
        context.with(|ctx| controller.release(&ctx));
        assert!(!controller.is_active());
    }

    #[test]
    fn toggles_promise_hook_with_active_als() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let hook_calls = Arc::new(AtomicUsize::new(0));
        let hook_calls_for_tracker = hook_calls.clone();
        let tracker: PromiseHook = Box::new(
            move |_: Ctx<'_>, _: PromiseHookType, _: Value<'_>, _: Value<'_>| {
                hook_calls_for_tracker.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            },
        );
        let controller = PromiseHookController::new(tracker);
        context.with(|ctx| {
            ctx.eval::<Value, _>("Promise.resolve(1)").unwrap();
        });
        assert_eq!(hook_calls.load(std::sync::atomic::Ordering::Relaxed), 0);
        context.with(|ctx| {
            controller.acquire(&ctx);
            ctx.eval::<Value, _>("Promise.resolve(1)").unwrap();
        });
        assert!(hook_calls.load(std::sync::atomic::Ordering::Relaxed) > 0);
        context.with(|ctx| controller.release(&ctx));
        let calls_after_release = hook_calls.load(std::sync::atomic::Ordering::Relaxed);
        context.with(|ctx| {
            ctx.eval::<Value, _>("Promise.resolve(1)").unwrap();
        });
        assert_eq!(
            hook_calls.load(std::sync::atomic::Ordering::Relaxed),
            calls_after_release
        );
    }
}
