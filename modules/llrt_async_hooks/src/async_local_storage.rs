// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use rquickjs::{
    atom::PredefinedAtom,
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, Function, JsLifetime, Object, Persistent, Result, Type, Value,
};
use smallvec::SmallVec;

use super::{get_current_id, next_native_id, AsyncHookState, TRACK_ALS};
use crate::async_context::{enter_async_scope, get_promise_id, insert_promise_id};

pub(crate) type AsyncLocalStorageHandle<'js> = Rc<RefCell<AsyncLocalStorageState<'js>>>;
pub(crate) type AsyncLocalStorageWeakHandle<'js> = Weak<RefCell<AsyncLocalStorageState<'js>>>;
type AsyncLocalStorageSnapshot<'js> = Vec<(
    AsyncLocalStorageHandle<'js>,
    Option<Persistent<Value<'static>>>,
)>;

pub(crate) struct AsyncLocalStorageState<'js> {
    stores: HashMap<u64, Persistent<Value<'static>>>,
    last_store: Option<Persistent<Value<'static>>>,
    default_value: Option<Persistent<Value<'static>>>,
    name: Option<String>,
    enabled: bool,
    _marker: std::marker::PhantomData<&'js ()>,
}

impl<'js> AsyncLocalStorageState<'js> {
    fn new() -> Self {
        Self {
            stores: HashMap::new(),
            last_store: None,
            default_value: None,
            name: None,
            enabled: true,
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(rquickjs::class::Trace)]
#[rquickjs::class]
pub(crate) struct AsyncLocalStorage<'js> {
    #[qjs(skip_trace)]
    storage: AsyncLocalStorageHandle<'js>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncLocalStorage<'js> {
    type Changed<'to> = AsyncLocalStorage<'to>;
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> AsyncLocalStorage<'js> {
    #[qjs(constructor)]
    pub(crate) fn new(ctx: Ctx<'js>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut state = AsyncLocalStorageState::new();
        if let Some(options) = options.0 {
            let default_value = options.get::<_, Option<Value>>("defaultValue")?;
            state.default_value = default_value.map(|value| Persistent::save(&ctx, value));
            state.name = options.get::<_, Option<String>>(PredefinedAtom::Name)?;
        }
        let storage = Rc::new(RefCell::new(state));
        let state = ctx
            .userdata::<RefCell<AsyncHookState>>()
            .ok_or_else(|| Exception::throw_internal(&ctx, "AsyncHookState is not initialized"))?;
        let mut state = state.borrow_mut();
        state.async_local_storages.push(Rc::downgrade(&storage));
        state.tracking |= TRACK_ALS;
        Ok(Self { storage })
    }

    pub(crate) fn run(
        &self,
        ctx: Ctx<'js>,
        store: Value<'js>,
        callback: Function<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<Value<'js>> {
        let parent_id = get_current_id(&ctx)?.0;
        let async_id = next_native_id(&ctx)?.0;
        enter_async_scope(&ctx, (async_id, parent_id))?;
        propagate_async_local_storage(&ctx, async_id, parent_id)?;
        let previous = {
            let mut storage = self.storage.borrow_mut();
            storage.enabled = true;
            storage
                .stores
                .insert(async_id, Persistent::save(&ctx, store.clone()))
        };
        enable_async_local_storage_tracking(&ctx)?;
        let result = callback.call::<_, Value>((args,));
        let result_async_id = match &result {
            Ok(value) if value.type_of() == Type::Promise => {
                let result_async_id = get_promise_id(&ctx, value)?;
                if result_async_id.0 == 0 {
                    insert_promise_id(&ctx, value, &Value::new_undefined(ctx.clone()))?
                } else {
                    result_async_id
                }
            },
            _ => (0, 0),
        };
        super::async_context::exit_async_scope(&ctx, async_id)?;
        let mut storage = self.storage.borrow_mut();
        storage.stores.remove(&async_id);
        if result_async_id.0 != 0 {
            storage
                .stores
                .insert(result_async_id.0, Persistent::save(&ctx, store));
            storage.last_store = storage.stores.get(&result_async_id.0).cloned();
        }
        if let Some(previous) = previous {
            storage.stores.insert(parent_id, previous);
        }
        result
    }

    pub(crate) fn enter_with(&self, ctx: Ctx<'js>, store: Value<'js>) -> Result<()> {
        let (async_id, trigger_id) = get_current_id(&ctx)?;
        let mut storage = self.storage.borrow_mut();
        storage.enabled = true;
        storage
            .stores
            .insert(async_id, Persistent::save(&ctx, store.clone()));
        if trigger_id != async_id {
            storage
                .stores
                .insert(trigger_id, Persistent::save(&ctx, store));
        }
        drop(storage);
        enable_async_local_storage_tracking(&ctx)?;
        Ok(())
    }

    pub(crate) fn exit(
        &self,
        ctx: Ctx<'js>,
        callback: Function<'js>,
        args: Rest<Value<'js>>,
    ) -> Result<Value<'js>> {
        let async_id = get_current_id(&ctx)?.0;
        let previous = self.storage.borrow_mut().stores.remove(&async_id);
        let result = callback.call::<_, Value>((args,));
        let mut storage = self.storage.borrow_mut();
        storage.stores.remove(&async_id);
        if let Some(previous) = previous {
            storage.stores.insert(async_id, previous);
        }
        storage.last_store = None;
        result
    }

    pub(crate) fn get_store(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let async_id = get_current_id(&ctx)?.0;
        let storage = self.storage.borrow();
        if storage.enabled {
            if let Some(store) = storage.stores.get(&async_id) {
                return store.clone().restore(&ctx);
            }
            if storage.last_store.is_some() {
                if let Some(store) = &storage.last_store {
                    return store.clone().restore(&ctx);
                }
            }
            if let Some(default_value) = &storage.default_value {
                return default_value.clone().restore(&ctx);
            }
        }
        Ok(Value::new_undefined(ctx))
    }

    #[qjs(get)]
    pub(crate) fn name(&self) -> Option<String> {
        self.storage.borrow().name.clone()
    }

    pub(crate) fn disable(this: This<Class<'js, Self>>) -> Class<'js, Self> {
        let storage = this.borrow().storage.clone();
        let mut storage = storage.borrow_mut();
        storage.enabled = false;
        storage.stores.clear();
        storage.last_store = None;
        this.0
    }
}

fn enable_async_local_storage_tracking(ctx: &Ctx<'_>) -> Result<()> {
    let state = ctx
        .userdata::<RefCell<AsyncHookState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncHookState is not initialized"))?;
    state.borrow_mut().tracking |= TRACK_ALS;
    Ok(())
}

pub(crate) fn propagate_async_local_storage<'js>(
    ctx: &Ctx<'js>,
    child_async_id: u64,
    trigger_async_id: u64,
) -> Result<()> {
    let state = ctx
        .userdata::<RefCell<AsyncHookState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncHookState is not initialized"))?;
    if state.borrow().async_local_storages.is_empty() {
        return Ok(());
    }
    let storages = active_storages(&state);
    for storage in storages {
        let mut storage = storage.borrow_mut();
        if storage.enabled {
            if let Some(store) = storage.stores.get(&trigger_async_id).cloned() {
                storage.stores.insert(child_async_id, store);
            }
        }
    }
    Ok(())
}

pub(crate) fn remove_async_local_storage<'js>(ctx: &Ctx<'js>, async_id: u64) -> Result<()> {
    let state = ctx
        .userdata::<RefCell<AsyncHookState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncHookState is not initialized"))?;
    if state.borrow().async_local_storages.is_empty() {
        return Ok(());
    }
    let storages = active_storages(&state);
    for storage in storages {
        storage.borrow_mut().stores.remove(&async_id);
    }
    Ok(())
}

pub(crate) fn cleanup_async_local_storage<'js>(storages: &[AsyncLocalStorageWeakHandle<'js>]) {
    for storage in storages {
        if let Some(storage) = storage.upgrade() {
            let mut storage = storage.borrow_mut();
            storage.stores.clear();
            storage.last_store = None;
            storage.default_value = None;
        }
    }
}

pub(crate) fn has_active_async_local_storage<'js>(
    storages: &[AsyncLocalStorageWeakHandle<'js>],
) -> bool {
    storages.iter().any(|storage| {
        let Some(storage) = storage.upgrade() else {
            return false;
        };
        let enabled = storage.borrow().enabled;
        enabled
    })
}

fn active_storages<'js>(
    state: &RefCell<AsyncHookState<'js>>,
) -> SmallVec<[AsyncLocalStorageHandle<'js>; 2]> {
    let mut state = state.borrow_mut();
    state
        .async_local_storages
        .retain(|storage| storage.strong_count() > 0);
    let active: SmallVec<[AsyncLocalStorageHandle<'js>; 2]> = state
        .async_local_storages
        .iter()
        .filter_map(|storage| {
            let storage = Weak::upgrade(storage)?;
            let enabled = storage.borrow().enabled;
            enabled.then_some(storage)
        })
        .collect();
    if active.is_empty() {
        state.tracking &= !TRACK_ALS;
    }
    active
}

fn capture_snapshot<'js>(ctx: &Ctx<'js>) -> Result<AsyncLocalStorageSnapshot<'js>> {
    let async_id = get_current_id(ctx)?.0;
    let state = ctx
        .userdata::<RefCell<AsyncHookState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncHookState is not initialized"))?;
    if state.borrow().async_local_storages.is_empty() {
        return Ok(Vec::new());
    }
    let storages = active_storages(&state);
    Ok(storages
        .into_iter()
        .map(|storage| {
            let store = storage.borrow().stores.get(&async_id).cloned();
            (storage, store)
        })
        .collect())
}

fn call_with_snapshot<'js>(
    ctx: &Ctx<'js>,
    snapshot: &AsyncLocalStorageSnapshot<'js>,
    callback: &Function<'js>,
    this: Value<'js>,
    args: Rest<Value<'js>>,
) -> Result<Value<'js>> {
    let async_id = get_current_id(ctx)?.0;
    let previous = snapshot
        .iter()
        .map(|(storage, store)| {
            let mut storage = storage.borrow_mut();
            let previous = storage.stores.remove(&async_id);
            if storage.enabled {
                if let Some(store) = store {
                    storage.stores.insert(async_id, store.clone());
                }
            }
            previous
        })
        .collect::<Vec<_>>();
    let result = callback.call::<_, Value>((This(this), args));
    for ((storage, _), previous) in snapshot.iter().zip(previous) {
        let mut storage = storage.borrow_mut();
        storage.stores.remove(&async_id);
        if let Some(previous) = previous {
            storage.stores.insert(async_id, previous);
        }
    }
    result
}

pub(crate) fn bind<'js>(ctx: Ctx<'js>, callback: Function<'js>) -> Result<Function<'js>> {
    let snapshot = capture_snapshot(&ctx)?;
    let call_ctx = ctx.clone();
    Function::new(
        ctx,
        move |this: This<Value<'js>>, args: Rest<Value<'js>>| {
            call_with_snapshot(&call_ctx, &snapshot, &callback, this.0, args)
        },
    )
}

pub(crate) fn snapshot<'js>(ctx: Ctx<'js>) -> Result<Function<'js>> {
    let snapshot = capture_snapshot(&ctx)?;
    let call_ctx = ctx.clone();
    Function::new(
        ctx,
        move |this: This<Value<'js>>, callback: Function<'js>, args: Rest<Value<'js>>| {
            call_with_snapshot(&call_ctx, &snapshot, &callback, this.0, args)
        },
    )
}
