// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use llrt_hooking::{acquire_hooking, release_hooking};
use rquickjs::{
    atom::PredefinedAtom,
    class::Trace,
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, Function, JsLifetime, Object, Persistent, Result, Type, Value,
};
use smallvec::SmallVec;

use crate::async_context::{
    enter_async_scope, exit_async_scope, get_current_id, get_promise_id, insert_promise_id,
    next_native_id,
};
use crate::tracking::{set_bit, TRACK_ALS};

pub(crate) type AsyncLocalStorageHandle<'js> = Rc<RefCell<AsyncLocalStorageState<'js>>>;
pub(crate) type AsyncLocalStorageWeakHandle<'js> = Weak<RefCell<AsyncLocalStorageState<'js>>>;

#[derive(Default)]
pub(crate) struct AsyncLocalStorageRegistry<'js> {
    pub(crate) storages: Vec<AsyncLocalStorageWeakHandle<'js>>,
    _marker: std::marker::PhantomData<&'js ()>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncLocalStorageRegistry<'js> {
    type Changed<'to> = AsyncLocalStorageRegistry<'to>;
}

type AsyncLocalStorageSnapshot<'js> = Vec<(
    AsyncLocalStorageHandle<'js>,
    Option<Persistent<Value<'static>>>,
)>;

// QuickJS does not expose the promise executing a normal reaction job while
// JS_ExecutePendingJob drains the microtask queue. As a result, the async ID
// is not available when an await continuation calls getStore(), even though
// the Promise Hook has already assigned an ID to that promise. Keep the latest
// promise store as a compatibility fallback for that gap. This is only a
// runtime workaround and should be removed when QuickJS can report the active
// job context. Because it stores only the latest value, it cannot completely
// isolate concurrent Promise continuations; correct concurrent propagation
// requires the runtime to identify the executing job. See:
// https://github.com/quickjs-ng/quickjs/issues/1047
struct PromiseStoreFallback {
    last_store: Option<Persistent<Value<'static>>>,
}

impl PromiseStoreFallback {
    fn new() -> Self {
        Self { last_store: None }
    }

    fn fallback(&self) -> Option<Persistent<Value<'static>>> {
        self.last_store.clone()
    }

    fn remember(&mut self, stores: &HashMap<u64, Persistent<Value<'static>>>, async_id: u64) {
        self.last_store = stores.get(&async_id).cloned();
    }

    fn clear(&mut self) {
        self.last_store = None;
    }
}

pub(crate) struct AsyncLocalStorageState<'js> {
    stores: HashMap<u64, Persistent<Value<'static>>>,
    promise_fallback: PromiseStoreFallback,
    default_value: Option<Persistent<Value<'static>>>,
    name: Option<String>,
    enabled: bool,
    _marker: std::marker::PhantomData<&'js ()>,
}

impl<'js> AsyncLocalStorageState<'js> {
    fn new() -> Self {
        Self {
            stores: HashMap::new(),
            promise_fallback: PromiseStoreFallback::new(),
            default_value: None,
            name: None,
            enabled: true,
            _marker: std::marker::PhantomData,
        }
    }

    fn enable(&mut self) {
        if !self.enabled {
            acquire_hooking();
            self.enabled = true;
        }
    }

    fn clear_stores(&mut self) {
        self.stores.clear();
        self.promise_fallback.clear();
    }

    fn disable(&mut self) {
        if self.enabled {
            release_hooking();
            self.enabled = false;
        }
        self.clear_stores();
    }

    fn cleanup(&mut self) {
        self.disable();
        self.default_value = None;
    }

    fn store_for_async_id(&self, async_id: u64) -> Option<Persistent<Value<'static>>> {
        if let Some(store) = self.stores.get(&async_id) {
            return Some(store.clone());
        }
        self.promise_fallback.fallback()
    }

    fn insert_promise_store(&mut self, async_id: u64, store: Persistent<Value<'static>>) {
        self.stores.insert(async_id, store);
        self.promise_fallback.remember(&self.stores, async_id);
    }

    fn clear_promise_fallback(&mut self) {
        self.promise_fallback.clear();
    }
}

#[derive(Trace)]
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
            .userdata::<RefCell<AsyncLocalStorageRegistry>>()
            .ok_or_else(|| {
                Exception::throw_internal(&ctx, "AsyncLocalStorage is not initialized")
            })?;
        state.borrow_mut().storages.push(Rc::downgrade(&storage));
        set_bit(&ctx, TRACK_ALS, true)?;
        acquire_hooking();
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
            storage.enable();
            storage
                .stores
                .insert(async_id, Persistent::save(&ctx, store.clone()))
        };
        set_bit(&ctx, TRACK_ALS, true)?;
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
        exit_async_scope(&ctx, async_id)?;
        let mut storage = self.storage.borrow_mut();
        storage.stores.remove(&async_id);
        if result_async_id.0 != 0 {
            storage.insert_promise_store(result_async_id.0, Persistent::save(&ctx, store));
        }
        if let Some(previous) = previous {
            storage.stores.insert(parent_id, previous);
        }
        result
    }

    pub(crate) fn enter_with(&self, ctx: Ctx<'js>, store: Value<'js>) -> Result<()> {
        let (async_id, trigger_id) = get_current_id(&ctx)?;
        let mut storage = self.storage.borrow_mut();
        storage.enable();
        storage
            .stores
            .insert(async_id, Persistent::save(&ctx, store.clone()));
        if trigger_id != async_id {
            storage
                .stores
                .insert(trigger_id, Persistent::save(&ctx, store));
        }
        drop(storage);
        set_bit(&ctx, TRACK_ALS, true)?;
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
        storage.clear_promise_fallback();
        result
    }

    pub(crate) fn get_store(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let async_id = get_current_id(&ctx)?.0;
        let storage = self.storage.borrow();
        if storage.enabled {
            if let Some(store) = storage.store_for_async_id(async_id) {
                return store.clone().restore(&ctx);
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
        storage.borrow_mut().disable();
        refresh_tracking(this.0.ctx());
        this.0
    }
}

fn refresh_tracking(ctx: &Ctx<'_>) {
    let Some(registry) = ctx.userdata::<RefCell<AsyncLocalStorageRegistry>>() else {
        return;
    };
    let enabled = registry.borrow().storages.iter().any(|storage| {
        storage
            .upgrade()
            .is_some_and(|storage| storage.borrow().enabled)
    });
    drop(registry);
    let _ = set_bit(ctx, TRACK_ALS, enabled);
}

pub(crate) fn propagate_async_local_storage<'js>(
    ctx: &Ctx<'js>,
    child_async_id: u64,
    trigger_async_id: u64,
) -> Result<()> {
    let storages = active_storages(ctx)?;
    for storage in storages {
        let mut storage = storage.borrow_mut();
        if let Some(store) = storage.stores.get(&trigger_async_id).cloned() {
            storage.stores.insert(child_async_id, store);
        }
    }
    Ok(())
}

pub(crate) fn remove_async_local_storage<'js>(ctx: &Ctx<'js>, async_id: u64) -> Result<()> {
    let storages = active_storages(ctx)?;
    for storage in storages {
        storage.borrow_mut().stores.remove(&async_id);
    }
    Ok(())
}

pub(crate) fn cleanup<'js>(storages: &[AsyncLocalStorageWeakHandle<'js>]) {
    for storage in storages {
        if let Some(storage) = storage.upgrade() {
            storage.borrow_mut().cleanup();
        }
    }
}

fn active_storages<'js>(ctx: &Ctx<'js>) -> Result<SmallVec<[AsyncLocalStorageHandle<'js>; 2]>> {
    let state = ctx
        .userdata::<RefCell<AsyncLocalStorageRegistry>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncLocalStorage is not initialized"))?;
    let mut state = state.borrow_mut();
    state.storages.retain(|storage| storage.strong_count() > 0);
    let active: SmallVec<[AsyncLocalStorageHandle<'js>; 2]> = state
        .storages
        .iter()
        .filter_map(|storage| {
            let storage = Weak::upgrade(storage)?;
            let enabled = storage.borrow().enabled;
            enabled.then_some(storage)
        })
        .collect();
    let enabled = !active.is_empty();
    drop(state);
    set_bit(ctx, TRACK_ALS, enabled)?;
    Ok(active)
}

fn capture_snapshot<'js>(ctx: &Ctx<'js>) -> Result<AsyncLocalStorageSnapshot<'js>> {
    let async_id = get_current_id(ctx)?.0;
    let state = ctx
        .userdata::<RefCell<AsyncLocalStorageRegistry>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncLocalStorage is not initialized"))?;
    if state.borrow().storages.is_empty() {
        return Ok(Vec::new());
    }
    let storages = active_storages(ctx)?;
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
