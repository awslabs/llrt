// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashMap, marker::PhantomData};

use rquickjs::{
    atom::PredefinedAtom, prelude::This, BigInt, Constructor, Ctx, Exception, Function, JsLifetime,
    Object, Persistent, Result, Value,
};

use crate::AsyncResourceKind;

pub fn init_async_context_state(ctx: &Ctx<'_>) -> Result<()> {
    let weak_map: Constructor = ctx.globals().get(PredefinedAtom::WeakMap)?;
    let promise_map: Object = weak_map.construct(())?;
    let state = RefCell::new(AsyncContextState::new(Persistent::save(ctx, promise_map)));
    ctx.store_userdata(state)
        .map_err(|error| Exception::throw_message(ctx, &error.to_string()))?;
    Ok(())
}

pub struct AsyncContextState<'js> {
    next_async_id: u64,
    async_resources: HashMap<u64, Persistent<Object<'static>>>,
    current_id: (u64, u64),
    context_stack: HashMap<u64, (u64, u64)>,
    promise_map: Persistent<Object<'static>>,
    _marker: PhantomData<&'js ()>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncContextState<'js> {
    type Changed<'to> = AsyncContextState<'to>;
}

impl AsyncContextState<'_> {
    pub fn new(promise_map: Persistent<Object<'static>>) -> Self {
        Self {
            next_async_id: 1,
            async_resources: HashMap::new(),
            current_id: (1, 1),
            context_stack: HashMap::new(),
            promise_map,
            _marker: PhantomData,
        }
    }

    fn clear_runtime_state(&mut self) {
        self.async_resources.clear();
        self.context_stack.clear();
    }

    fn next_id(&mut self, ctx: &Ctx<'_>) -> Result<u64> {
        self.next_async_id = self
            .next_async_id
            .checked_add(1)
            .ok_or_else(|| Exception::throw_internal(ctx, "Async resource ID overflow"))?;
        Ok(self.next_async_id)
    }

    fn next_native_id(&mut self, ctx: &Ctx<'_>) -> Result<(u64, u64)> {
        let async_id = self.next_id(ctx)?;
        Ok((async_id, self.current_id.0))
    }

    fn current_id(&self) -> (u64, u64) {
        self.current_id
    }

    fn enter_scope(&mut self, id: (u64, u64)) {
        self.context_stack.insert(id.0, self.current_id);
        self.current_id = id;
    }

    fn exit_scope(&mut self, async_id: u64) {
        if let Some(previous) = self.context_stack.remove(&async_id) {
            self.current_id = previous;
        }
    }

    fn insert_resource(&mut self, id: u64, resource: Persistent<Object<'static>>) {
        self.async_resources.insert(id, resource);
    }

    fn remove_resource(&mut self, id: u64) {
        self.async_resources.remove(&id);
    }

    fn resource(&self, id: u64) -> Option<Persistent<Object<'static>>> {
        self.async_resources.get(&id).cloned()
    }
}

fn with_state<T>(ctx: &Ctx<'_>, f: impl FnOnce(&AsyncContextState) -> T) -> Result<T> {
    let state = ctx
        .userdata::<RefCell<AsyncContextState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "Async context is not initialized"))?;
    let result = {
        let state = state.borrow();
        f(&state)
    };
    Ok(result)
}

fn with_state_mut<T>(ctx: &Ctx<'_>, f: impl FnOnce(&mut AsyncContextState) -> T) -> Result<T> {
    let state = ctx
        .userdata::<RefCell<AsyncContextState>>()
        .ok_or_else(|| Exception::throw_internal(ctx, "Async context is not initialized"))?;
    let result = {
        let mut state = state.borrow_mut();
        f(&mut state)
    };
    Ok(result)
}

pub fn get_promise_map<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    with_state(ctx, |state| state.promise_map.clone())?.restore(ctx)
}

pub fn get_promise_id<'js>(ctx: &Ctx<'js>, promise: &Value<'js>) -> Result<(u64, u64)> {
    let promise_map = get_promise_map(ctx)?;
    let get: Function = promise_map.get(PredefinedAtom::Getter)?;
    let token: Value = get.call((This(promise_map), promise.clone()))?;
    parse_promise_id(&token)
}

fn parse_promise_id(token: &Value<'_>) -> Result<(u64, u64)> {
    let Some(token) = token.as_object() else {
        return Ok((0, 0));
    };
    let async_id = parse_non_negative_id(token.get::<_, BigInt>("asyncId")?).unwrap_or(0);
    let trigger_id = parse_non_negative_id(token.get::<_, BigInt>("triggerId")?).unwrap_or(0);
    if async_id == 0 || trigger_id == 0 {
        return Ok((0, 0));
    }
    Ok((async_id, trigger_id))
}

fn parse_non_negative_id(value: BigInt) -> Option<u64> {
    value
        .to_i64()
        .ok()
        .filter(|id| *id >= 0)
        .map(|id| id as u64)
}

fn next_async_id(ctx: &Ctx<'_>) -> Result<u64> {
    with_state_mut(ctx, |state| state.next_id(ctx))?
}

pub fn insert_promise_id<'js>(
    ctx: &Ctx<'js>,
    promise: &Value<'js>,
    parent: &Value<'js>,
) -> Result<(u64, u64)> {
    let existing_id = get_promise_id(ctx, promise)?;
    if existing_id.0 != 0 {
        return Ok(existing_id);
    }
    let trigger_id = if parent.as_object().is_some() {
        get_promise_id(ctx, parent)?.0
    } else {
        get_current_id(ctx)?.0
    };
    let current_id = (
        next_async_id(ctx)?,
        if trigger_id == 0 { 1 } else { trigger_id },
    );
    let promise_map = get_promise_map(ctx)?;
    let set: Function = promise_map.get(PredefinedAtom::Setter)?;
    let token = Object::new(ctx.clone())?;
    token.set("asyncId", BigInt::from_u64(ctx.clone(), current_id.0)?)?;
    token.set("triggerId", BigInt::from_u64(ctx.clone(), current_id.1)?)?;
    set.call::<_, Value>((This(promise_map), promise.clone(), token))?;
    Ok(current_id)
}

pub fn get_trigger_id_from_token(token: &Value<'_>) -> Result<u64> {
    let Some(token) = token.as_object() else {
        return Ok(0);
    };
    let trigger_id = parse_non_negative_id(token.get::<_, BigInt>("triggerId")?).unwrap_or(0);
    Ok(trigger_id)
}

pub fn register_async_resource<'js>(
    ctx: Ctx<'js>,
    target: Value<'js>,
    resource: Value<'js>,
) -> Result<()> {
    let (kind, target) = parse_async_token(&ctx, &target)?;
    if kind != AsyncResourceKind::Native {
        return Err(Exception::throw_type(&ctx, "Invalid native resource token"));
    }
    let constructor: Constructor = ctx.globals().get("WeakRef")?;
    let weak_ref: Object = constructor.construct((resource,))?;
    with_state_mut(&ctx, |state| {
        state.insert_resource(target, Persistent::save(&ctx, weak_ref))
    })?;
    Ok(())
}

pub fn parse_async_token(ctx: &Ctx<'_>, token: &Value<'_>) -> Result<(AsyncResourceKind, u64)> {
    let token = token
        .as_object()
        .ok_or_else(|| Exception::throw_type(ctx, "Invalid async resource token"))?;
    let kind: u8 = token.get("kind")?;
    let kind = match kind {
        0 => AsyncResourceKind::Native,
        1 => AsyncResourceKind::Promise,
        _ => {
            return Err(Exception::throw_type(
                ctx,
                "Invalid async resource token kind",
            ))
        },
    };
    let id: BigInt = token.get("asyncId")?;
    let id = parse_non_negative_id(id)
        .ok_or_else(|| Exception::throw_type(ctx, "Invalid async resource token"))?;
    Ok((kind, id))
}

pub fn next_native_id(ctx: &Ctx<'_>) -> Result<(u64, u64)> {
    with_state_mut(ctx, |state| state.next_native_id(ctx))?
}

pub fn remove_native_resource(ctx: &Ctx<'_>, async_id: u64) -> Result<()> {
    with_state_mut(ctx, |state| state.remove_resource(async_id))?;
    Ok(())
}

pub fn get_current_id(ctx: &Ctx<'_>) -> Result<(u64, u64)> {
    with_state(ctx, |state| state.current_id())
}

pub fn get_current_resource(ctx: Ctx<'_>) -> Result<Object<'_>> {
    let weak_ref = with_state(&ctx, |state| state.resource(state.current_id().0))?;
    let Some(weak_ref) = weak_ref else {
        return Object::new(ctx.clone());
    };
    let weak_ref = weak_ref.restore(&ctx)?;
    let deref: Function = weak_ref.get("deref")?;
    let resource: Value = deref.call((This(weak_ref),))?;
    match resource.as_object().cloned() {
        Some(resource) => Ok(resource),
        None => Object::new(ctx.clone()),
    }
}

pub fn enter_async_scope(ctx: &Ctx<'_>, id: (u64, u64)) -> Result<()> {
    with_state_mut(ctx, |state| state.enter_scope(id))?;
    Ok(())
}

pub fn exit_async_scope(ctx: &Ctx<'_>, async_id: u64) -> Result<()> {
    with_state_mut(ctx, |state| state.exit_scope(async_id))?;
    Ok(())
}

pub fn cleanup_async_context(ctx: &Ctx<'_>) {
    if let Some(state) = ctx.userdata::<RefCell<AsyncContextState>>() {
        state.borrow_mut().clear_runtime_state();
    }
}

#[cfg(test)]
mod tests {
    use super::{get_current_id, init_async_context_state, AsyncContextState};
    use rquickjs::{Context, Ctx, Object, Persistent, Runtime};

    fn new_state<'js>(ctx: &Ctx<'js>) -> AsyncContextState<'js> {
        AsyncContextState::new(Persistent::save(ctx, Object::new(ctx.clone()).unwrap()))
    }

    #[test]
    fn nested_scopes_restore_previous_ids() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            let mut state = new_state(&ctx);
            assert_eq!(state.current_id(), (1, 1));
            state.enter_scope((2, 1));
            state.enter_scope((3, 2));
            assert_eq!(state.current_id(), (3, 2));
            state.exit_scope(3);
            assert_eq!(state.current_id(), (2, 1));
            state.exit_scope(2);
            assert_eq!(state.current_id(), (1, 1));
            state.exit_scope(0);
            assert_eq!(state.current_id(), (1, 1));
        });
    }

    #[test]
    fn clear_runtime_state_removes_scope_stack_without_resetting_current_id() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            let mut state = new_state(&ctx);
            state.enter_scope((9, 8));
            state.clear_runtime_state();
            state.exit_scope(9);
            assert_eq!(state.current_id(), (9, 8));
        });
    }

    #[test]
    fn next_id_starts_after_reserved_root_id() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            let mut state = new_state(&ctx);
            assert_eq!(state.next_id(&ctx).unwrap(), 2);
            assert_eq!(state.next_id(&ctx).unwrap(), 3);
        });
    }

    #[test]
    fn initializes_context_tracking_state() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        context.with(|ctx| {
            init_async_context_state(&ctx).unwrap();
            assert_eq!(get_current_id(&ctx).unwrap(), (1, 1));
        });
    }
}
