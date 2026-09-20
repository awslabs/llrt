// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::cell::RefCell;

use llrt_async_context::state::get_current_id;
use rquickjs::{
    prelude::{Rest, This},
    Ctx, Exception, Function, Persistent, Result, Value,
};

use crate::async_local_storage::{
    active_storages, AsyncLocalStorageHandle, AsyncLocalStorageRegistry,
};

type AsyncLocalStorageSnapshot<'js> = Vec<(
    AsyncLocalStorageHandle<'js>,
    Option<Persistent<Value<'static>>>,
)>;

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
