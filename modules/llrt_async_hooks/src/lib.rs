// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Function, JsLifetime, Object, Result, Value,
};

mod hooking;

pub(crate) struct Hook<'js> {
    pub(crate) enabled: Rc<RefCell<bool>>,
    pub(crate) init: Option<Function<'js>>,
    pub(crate) before: Option<Function<'js>>,
    pub(crate) after: Option<Function<'js>>,
    pub(crate) destroy: Option<Function<'js>>,
    pub(crate) promise_resolve: Option<Function<'js>>,
}

pub(crate) struct AsyncHookState<'js> {
    pub(crate) hooks: Vec<Hook<'js>>,
}

impl Default for AsyncHookState<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHookState<'_> {
    pub(crate) fn new() -> Self {
        Self { hooks: Vec::new() }
    }
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookState<'js> {
    type Changed<'to> = AsyncHookState<'to>;
}

pub(crate) struct AsyncHookIds<'js> {
    pub(crate) next_async_id: u64,
    pub(crate) execution_async_id: u64,
    pub(crate) trigger_async_id: u64,
    _marker: PhantomData<&'js ()>,
}

impl Default for AsyncHookIds<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHookIds<'_> {
    pub(crate) fn new() -> Self {
        Self {
            next_async_id: 0,
            execution_async_id: 0,
            trigger_async_id: 0,
            _marker: PhantomData,
        }
    }
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookIds<'js> {
    type Changed<'to> = AsyncHookIds<'to>;
}

pub(crate) fn create_hook<'js>(ctx: Ctx<'js>, hooks_obj: Object<'js>) -> Result<Value<'js>> {
    hooking::promise(&ctx)?;
    hooking::timeout(&ctx)?;
    hooking::immediate(&ctx)?;

    let init = hooks_obj.get::<_, Function>("init").ok();
    let before = hooks_obj.get::<_, Function>("before").ok();
    let after = hooks_obj.get::<_, Function>("after").ok();
    let destroy = hooks_obj.get::<_, Function>("destroy").ok();
    let promise_resolve = hooks_obj.get::<_, Function>("promiseResolve").ok();
    let enabled = Rc::new(RefCell::new(false));

    let hook = Hook {
        enabled: enabled.clone(),
        init,
        before,
        after,
        destroy,
        promise_resolve,
    };

    let binding = ctx.userdata::<RefCell<AsyncHookState>>().unwrap();
    let mut state = binding.borrow_mut();
    state.hooks.push(hook);

    let obj = Object::new(ctx.clone())?;
    let enabled_clone = enabled.clone();
    obj.set(
        "enable",
        Function::new(ctx.clone(), move || -> Result<()> {
            *enabled_clone.borrow_mut() = true;
            Ok(())
        }),
    )?;
    let enabled_clone = enabled.clone();
    obj.set(
        "disable",
        Function::new(ctx.clone(), move || -> Result<()> {
            *enabled_clone.borrow_mut() = false;
            Ok(())
        }),
    )?;

    Ok(obj.into())
}

fn current_id() -> u64 {
    // NOTE: This method is now obsolete. Therefore, it does not return a valid value.
    // But we will define it because it is used by cls-hooked.
    0
}

fn next_async_id(ctx: Ctx<'_>) -> u64 {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let mut ids = bind_ids.borrow_mut();
    ids.next_async_id += 1;
    ids.next_async_id
}

fn execution_async_id(ctx: Ctx<'_>) -> u64 {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let ids = bind_ids.borrow();
    ids.execution_async_id
}

fn trigger_async_id(ctx: Ctx<'_>) -> u64 {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let ids = bind_ids.borrow();
    ids.trigger_async_id
}

pub struct AsyncHooksModule;

impl ModuleDef for AsyncHooksModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createHook")?;
        declare.declare("currentId")?;
        declare.declare("executionAsyncId")?;
        declare.declare("triggerAsyncId")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("createHook", Func::from(create_hook))?;
            default.set("currentId", Func::from(current_id))?;
            default.set("executionAsyncId", Func::from(execution_async_id))?;
            default.set("triggerAsyncId", Func::from(trigger_async_id))?;

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

pub fn init<'js>(ctx: &Ctx<'js>) -> Result<()> {
    let global = ctx.globals();

    let _ = ctx.store_userdata(RefCell::new(AsyncHookState::default()));
    let _ = ctx.store_userdata(RefCell::new(AsyncHookIds::default()));

    global.set("__async_hook_next_id", Func::from(next_async_id))?;
    global.set("__async_hook_exec_id", Func::from(execution_async_id))?;

    global.set(
        "__async_hook_init",
        Func::from(
            move |ctx: Ctx<'js>, id: u64, name: String, trigger_id: Option<u64>| {
                let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
                let mut ids = bind_ids.borrow_mut();
                ids.execution_async_id = id;
                ids.trigger_async_id = trigger_id.unwrap_or_default();
                drop(ids);

                let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().unwrap();
                let state = bind_state.borrow();

                for hook in &state.hooks {
                    if *hook.enabled.as_ref().borrow() {
                        if let Some(func) = &hook.init {
                            let _: Result<()> = func.call((id, &name, trigger_id));
                        }
                    }
                }
                drop(state);
            },
        ),
    )?;

    global.set(
        "__async_hook_func",
        Func::from(move |ctx: Ctx<'js>, event: String, id: u64| {
            if event.as_str() == "before" {
                let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
                let mut ids = bind_ids.borrow_mut();
                ids.execution_async_id = id;
                drop(ids);
            }

            let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().unwrap();
            let state = bind_state.borrow();

            match event.as_str() {
                "before" => {
                    for hook in &state.hooks {
                        if *hook.enabled.as_ref().borrow() {
                            if let Some(func) = &hook.before {
                                let _: Result<()> = func.call((id,));
                            }
                        }
                    }
                },
                "after" => {
                    for hook in &state.hooks {
                        if *hook.enabled.as_ref().borrow() {
                            if let Some(func) = &hook.after {
                                let _: Result<()> = func.call((id,));
                            }
                        }
                    }
                },
                "destroy" => {
                    for hook in &state.hooks {
                        if *hook.enabled.as_ref().borrow() {
                            if let Some(func) = &hook.destroy {
                                let _: Result<()> = func.call((id,));
                            }
                        }
                    }
                },
                "promiseResolve" => {
                    for hook in &state.hooks {
                        if *hook.enabled.as_ref().borrow() {
                            if let Some(func) = &hook.promise_resolve {
                                let _: Result<()> = func.call((id,));
                            }
                        }
                    }
                },
                _ => {},
            }
            drop(state);
        }),
    )?;

    Ok(())
}
