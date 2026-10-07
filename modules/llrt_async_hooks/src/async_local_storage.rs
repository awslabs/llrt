// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use llrt_async_context::state::{
    enter_async_scope, exit_async_scope, get_current_id, get_promise_id, insert_promise_id,
    next_native_id,
};
use llrt_async_context::{add_async_context_observer, promise_hook_controller, AsyncContextEvent};
use rquickjs::{
    atom::PredefinedAtom,
    class::Trace,
    prelude::{Opt, Rest, This},
    Class, Ctx, Exception, Function, JsLifetime, Object, Persistent, Result, Type, Value,
};
use smallvec::SmallVec;

pub(crate) type AsyncLocalStorageHandle<'js> = Rc<RefCell<AsyncLocalStorageState<'js>>>;
pub(crate) type AsyncLocalStorageWeakHandle<'js> = Weak<RefCell<AsyncLocalStorageState<'js>>>;

pub(crate) fn register_async_context_observer(ctx: &Ctx<'_>) -> Result<()> {
    add_async_context_observer(ctx, observe_async_context)
}

fn observe_async_context(ctx: &Ctx<'_>, event: AsyncContextEvent) -> Result<()> {
    match event {
        AsyncContextEvent::Init {
            async_id,
            trigger_id,
        } => propagate_async_local_storage(ctx, async_id, trigger_id),
        AsyncContextEvent::Finalized { async_id, .. } => remove_async_local_storage(ctx, async_id),
    }
}

#[derive(Default)]
pub(crate) struct AsyncLocalStorageRegistry<'js> {
    pub(crate) storages: Vec<AsyncLocalStorageWeakHandle<'js>>,
    _marker: std::marker::PhantomData<&'js ()>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncLocalStorageRegistry<'js> {
    type Changed<'to> = AsyncLocalStorageRegistry<'to>;
}

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
    context: Option<Ctx<'js>>,
    pub(crate) stores: HashMap<u64, Persistent<Value<'static>>>,
    promise_fallback: PromiseStoreFallback,
    default_value: Option<Persistent<Value<'static>>>,
    name: Option<String>,
    pub(crate) enabled: bool,
}

impl<'js> AsyncLocalStorageState<'js> {
    fn new(ctx: Ctx<'js>) -> Self {
        Self {
            context: Some(ctx),
            stores: HashMap::new(),
            promise_fallback: PromiseStoreFallback::new(),
            default_value: None,
            name: None,
            enabled: true,
        }
    }

    fn enable(&mut self) {
        if !self.enabled {
            if let Some(context) = self.context.as_ref() {
                if let Some(controller) = promise_hook_controller(context) {
                    controller.acquire(context);
                }
            }
            self.enabled = true;
        }
    }

    fn clear_stores(&mut self) {
        self.stores.clear();
        self.promise_fallback.clear();
    }

    fn disable(&mut self) {
        if self.enabled {
            if let Some(context) = self.context.as_ref() {
                if let Some(controller) = promise_hook_controller(context) {
                    controller.release(context);
                }
            }
            self.enabled = false;
        }
        self.clear_stores();
    }

    fn cleanup(&mut self) {
        self.disable();
        self.context = None;
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

impl Drop for AsyncLocalStorageState<'_> {
    fn drop(&mut self) {
        if self.enabled {
            if let Some(context) = self.context.as_ref() {
                if let Some(controller) = promise_hook_controller(context) {
                    controller.release(context);
                }
            }
        }
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
        let mut state = AsyncLocalStorageState::new(ctx.clone());
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
        let controller = promise_hook_controller(&ctx).ok_or_else(|| {
            Exception::throw_internal(&ctx, "AsyncLocalStorage is not initialized")
        })?;
        controller.acquire(&ctx);
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
        let mut previous = None;
        let mut store_inserted = false;
        let mut result_async_id = (0, 0);
        let result = (|| {
            propagate_async_local_storage(&ctx, async_id, parent_id)?;
            previous = {
                let mut storage = self.storage.borrow_mut();
                storage.enable();
                storage
                    .stores
                    .insert(async_id, Persistent::save(&ctx, store.clone()))
            };
            store_inserted = true;

            let value = callback.call::<_, Value>((args,))?;
            if value.type_of() == Type::Promise {
                result_async_id = get_promise_id(&ctx, &value)?;
                if result_async_id.0 == 0 {
                    result_async_id =
                        insert_promise_id(&ctx, &value, &Value::new_undefined(ctx.clone()))?;
                }
            }
            Ok(value)
        })();
        let exit_result = exit_async_scope(&ctx, async_id);
        if store_inserted {
            let mut storage = self.storage.borrow_mut();
            storage.stores.remove(&async_id);
            if result_async_id.0 != 0 {
                storage.insert_promise_store(result_async_id.0, Persistent::save(&ctx, store));
            }
            if let Some(previous) = previous {
                storage.stores.insert(parent_id, previous);
            }
        }
        if let Err(error) = result {
            return Err(error);
        }
        exit_result?;
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
        this.0
    }
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

pub(crate) fn active_storages<'js>(
    ctx: &Ctx<'js>,
) -> Result<SmallVec<[AsyncLocalStorageHandle<'js>; 2]>> {
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
    drop(state);
    Ok(active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context, Runtime};

    #[test]
    fn dropping_state_without_bridge_is_safe() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            drop(AsyncLocalStorageState::new(ctx.clone()));
        });
    }
}
