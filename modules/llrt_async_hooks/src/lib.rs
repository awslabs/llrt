// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    promise::PromiseHookType,
    runtime::PromiseHook,
    Ctx, Function, JsLifetime, Object, Result, Value,
};
use tracing::trace;

struct Hook<'js> {
    enabled: Rc<RefCell<bool>>,
    init: Option<Function<'js>>,
    before: Option<Function<'js>>,
    after: Option<Function<'js>>,
    promise_resolve: Option<Function<'js>>,
}

struct AsyncHookState<'js> {
    hooks: Vec<Hook<'js>>,
}

impl Default for AsyncHookState<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHookState<'_> {
    fn new() -> Self {
        Self { hooks: Vec::new() }
    }
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookState<'js> {
    type Changed<'to> = AsyncHookState<'to>;
}

struct AsyncHookIds<'js> {
    next_async_id: u64,
    id_map: HashMap<usize, (u64, u64)>, // (execution_async_id, trigger_async_id)
    current_id: (u64, u64),             // (execution_async_id, trigger_async_id)
    _marker: PhantomData<&'js ()>,
}

impl Default for AsyncHookIds<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHookIds<'_> {
    fn new() -> Self {
        Self {
            next_async_id: 0,
            id_map: HashMap::new(),
            current_id: (0, 0),
            _marker: PhantomData,
        }
    }
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookIds<'js> {
    type Changed<'to> = AsyncHookIds<'to>;
}

fn create_hook<'js>(ctx: Ctx<'js>, hooks_obj: Object<'js>) -> Result<Value<'js>> {
    let init = hooks_obj.get::<_, Function>("init").ok();
    let before = hooks_obj.get::<_, Function>("before").ok();
    let after = hooks_obj.get::<_, Function>("after").ok();
    let promise_resolve = hooks_obj.get::<_, Function>("promiseResolve").ok();
    let enabled = Rc::new(RefCell::new(false));

    let hook = Hook {
        enabled: enabled.clone(),
        init,
        before,
        after,
        promise_resolve,
    };

    let binding = ctx.userdata::<RefCell<AsyncHookState>>().unwrap();
    let mut state = binding.borrow_mut();
    state.hooks.push(hook);

    let obj = Object::new(ctx.clone())?;
    {
        let enabled_clone = enabled.clone();
        obj.set(
            "enable",
            Function::new(ctx.clone(), move || -> Result<()> {
                *enabled_clone.borrow_mut() = true;
                Ok(())
            }),
        )?;
    }
    {
        let enabled_clone = enabled.clone();
        obj.set(
            "disable",
            Function::new(ctx.clone(), move || -> Result<()> {
                *enabled_clone.borrow_mut() = false;
                Ok(())
            }),
        )?;
    }

    Ok(obj.into())
}

fn current_id() -> u64 {
    // NOTE: This method is now obsolete. Therefore, it does not return a valid value.
    // But we will define it because it is used by cls-hooked.
    0
}

fn execution_async_id(ctx: Ctx<'_>) -> u64 {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let ids = bind_ids.borrow();
    ids.current_id.0
}

fn trigger_async_id(ctx: Ctx<'_>) -> u64 {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let ids = bind_ids.borrow();
    ids.current_id.1
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

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let global = ctx.globals();

    let _ = ctx.store_userdata(RefCell::new(AsyncHookState::default()));
    let _ = ctx.store_userdata(RefCell::new(AsyncHookIds::default()));

    global.set(
        "invokeAsyncHook",
        Func::from(
            move |ctx: Ctx<'_>, type_: String, async_type: String, uid: usize| {
                let type_ = match type_.as_ref() {
                    "init" => PromiseHookType::Init,
                    "before" => PromiseHookType::Before,
                    "after" => PromiseHookType::After,
                    "resolve" => PromiseHookType::Resolve,
                    _ => return,
                };

                let _ = invoke_async_hook(&ctx, type_, async_type.as_ref(), uid, None);
            },
        ),
    )?;

    Ok(())
}

pub fn promise_hook_tracker() -> PromiseHook {
    Box::new(
        |ctx: Ctx<'_>, type_: PromiseHookType, promise: Value<'_>, parent: Value<'_>| {
            // SAFETY: Since it checks in advance whether it is an Object type, we can always get a pointer to the object.
            let uid1 = promise
                .as_object()
                .map(|v| unsafe { v.as_raw().u.ptr } as usize)
                .unwrap();
            let uid2 = parent
                .as_object()
                .map(|v| unsafe { v.as_raw().u.ptr } as usize);

            let _ = invoke_async_hook(&ctx, type_, "PROMISE", uid1, uid2);
        },
    )
}

fn invoke_async_hook(
    ctx: &Ctx<'_>,
    type_: PromiseHookType,
    async_type: &str,
    object: usize,
    parent: Option<usize>,
) -> Result<()> {
    let bind_state = ctx.userdata::<RefCell<AsyncHookState>>().unwrap();
    let state = bind_state.borrow();

    for hook in &state.hooks {
        if *hook.enabled.as_ref().borrow() {
            match type_ {
                PromiseHookType::Init => {
                    let current_id = insert_id_map(ctx, object, parent);
                    update_current_id(ctx, current_id);
                    trace!("Init(async_id, trigger_id): {:?}", current_id);

                    if let Some(func) = &hook.init {
                        let _ = func
                            .call::<_, ()>((current_id.0, async_type, current_id.1))
                            .or_else(|_| func.call::<_, ()>((current_id.0, async_type)))
                            .or_else(|_| func.call::<_, ()>((current_id.0,)))
                            .or_else(|_| func.call::<_, ()>(()));
                    }
                },
                PromiseHookType::Before => {
                    let current_id = get_id_map(ctx, object);
                    update_current_id(ctx, current_id);
                    trace!("Before(async_id, trigger_id): {:?}", current_id);

                    if let Some(func) = &hook.before {
                        let _ = func
                            .call::<_, ()>((current_id.0,))
                            .or_else(|_| func.call::<_, ()>(()));
                    }
                },
                PromiseHookType::After => {
                    let current_id = get_id_map(ctx, object);
                    update_current_id(ctx, current_id);
                    trace!("After(async_id, trigger_id): {:?}", current_id);

                    if let Some(func) = &hook.after {
                        let _ = func
                            .call::<_, ()>((current_id.0,))
                            .or_else(|_| func.call::<_, ()>(()));
                    }
                },
                PromiseHookType::Resolve => {
                    let current_id = remove_id_map(ctx, object);
                    update_current_id(ctx, current_id);
                    trace!("Resolve(async_id, trigger_id): {:?}", current_id);

                    if let Some(func) = &hook.promise_resolve {
                        let _ = func
                            .call::<_, ()>((current_id.0,))
                            .or_else(|_| func.call::<_, ()>(()));
                    }
                },
            }
        }
    }
    Ok(())
}

fn insert_id_map(ctx: &Ctx<'_>, uid: usize, parent: Option<usize>) -> (u64, u64) {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let mut ids = bind_ids.borrow_mut();
    ids.next_async_id += 1;
    let async_id = ids.next_async_id;
    let trigger_id = parent
        .and_then(|tid| ids.id_map.get(&tid))
        .map(|id| id.0)
        .unwrap_or(0);
    ids.id_map.insert(uid, (async_id, trigger_id));
    (async_id, trigger_id)
}

fn get_id_map(ctx: &Ctx<'_>, uid: usize) -> (u64, u64) {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let ids = bind_ids.borrow();
    *ids.id_map.get(&uid).unwrap_or(&(0, 0))
}

fn remove_id_map(ctx: &Ctx<'_>, uid: usize) -> (u64, u64) {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    let mut ids = bind_ids.borrow_mut();
    ids.id_map
        .remove_entry(&uid)
        .map(|(_, (async_id, trigger_id))| (async_id, trigger_id))
        .unwrap_or((0, 0))
}

fn update_current_id(ctx: &Ctx<'_>, id: (u64, u64)) {
    let bind_ids = ctx.userdata::<RefCell<AsyncHookIds>>().unwrap();
    bind_ids.borrow_mut().current_id = id;
}
