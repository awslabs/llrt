// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashMap, marker::PhantomData};

use llrt_hooking::AsyncTokenKind;
use llrt_utils::result::ResultExt;
use rquickjs::{
    atom::PredefinedAtom, prelude::This, BigInt, Constructor, Ctx, Exception, Function, JsLifetime,
    Object, Persistent, Result, Value,
};

pub(crate) struct AsyncResourceState<'js> {
    next_async_id: u64,
    async_resources: HashMap<u64, Persistent<Object<'static>>>,
    current_id: (u64, u64),
    context_stack: HashMap<u64, (u64, u64)>,
    promise_map: Persistent<Object<'static>>,
    _marker: PhantomData<&'js ()>,
}

impl AsyncResourceState<'_> {
    pub(crate) fn new(promise_map: Persistent<Object<'static>>) -> Self {
        Self {
            next_async_id: 1,
            async_resources: HashMap::new(),
            current_id: (1, 1),
            context_stack: HashMap::new(),
            promise_map,
            _marker: PhantomData,
        }
    }

    fn clear(&mut self) {
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

    fn current_id(&self) -> (u64, u64) {
        self.current_id
    }

    fn update_current_id(&mut self, id: (u64, u64)) {
        self.current_id = id;
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

pub(crate) enum AsyncTarget<'js> {
    Native {
        id: u64,
        trigger_id: u64,
    },
    Promise {
        promise: Value<'js>,
        parent: Value<'js>,
    },
}

unsafe impl<'js> JsLifetime<'js> for AsyncResourceState<'js> {
    type Changed<'to> = AsyncResourceState<'to>;
}

fn with_state<T>(ctx: &Ctx<'_>, f: impl FnOnce(&AsyncResourceState) -> T) -> Result<T> {
    let state = ctx
        .userdata::<RefCell<AsyncResourceState>>()
        .or_throw(ctx)?;
    let result = f(&state.borrow());
    Ok(result)
}

fn with_state_mut<T>(ctx: &Ctx<'_>, f: impl FnOnce(&mut AsyncResourceState) -> T) -> Result<T> {
    let state = ctx
        .userdata::<RefCell<AsyncResourceState>>()
        .or_throw(ctx)?;
    let result = f(&mut state.borrow_mut());
    Ok(result)
}

pub(crate) fn get_promise_map<'js>(ctx: &Ctx<'js>) -> Result<Object<'js>> {
    let promise_map = with_state(ctx, |state| state.promise_map.clone())?;
    promise_map.restore(ctx)
}

pub(crate) fn get_promise_id<'js>(ctx: &Ctx<'js>, promise: &Value<'js>) -> Result<(u64, u64)> {
    let promise_map = get_promise_map(ctx)?;
    let get: Function = promise_map.get(PredefinedAtom::Getter)?;
    let token: Value = get.call((This(promise_map), promise.clone()))?;
    parse_promise_id(&token)
}

fn parse_promise_id(token: &Value<'_>) -> Result<(u64, u64)> {
    let Some(token) = token.as_object() else {
        return Ok((0, 0));
    };
    let async_id = token
        .get::<_, BigInt>("id")?
        .to_i64()
        .ok()
        .filter(|id| *id >= 0)
        .unwrap_or(0) as u64;
    let trigger_id = token
        .get::<_, BigInt>("triggerId")?
        .to_i64()
        .ok()
        .filter(|id| *id >= 0)
        .unwrap_or(0) as u64;
    if async_id == 0 || trigger_id == 0 {
        return Ok((0, 0));
    }
    Ok((async_id, trigger_id))
}

fn next_async_id(ctx: &Ctx<'_>) -> Result<u64> {
    with_state_mut(ctx, |state| state.next_id(ctx))?
}

pub(crate) fn insert_promise_id<'js>(
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
    token.set("id", BigInt::from_u64(ctx.clone(), current_id.0)?)?;
    token.set("triggerId", BigInt::from_u64(ctx.clone(), current_id.1)?)?;
    set.call::<_, Value>((This(promise_map), promise.clone(), token))?;
    Ok(current_id)
}

pub(crate) fn get_id_from_token(token: &Value<'_>) -> Result<(u64, u64)> {
    parse_promise_id(token)
}

pub(crate) fn register_async_resource<'js>(
    ctx: Ctx<'js>,
    target: Value<'js>,
    resource: Value<'js>,
) -> Result<()> {
    let (kind, target) = parse_async_token(&ctx, &target)?;
    if kind != AsyncTokenKind::Native {
        return Err(Exception::throw_type(&ctx, "Invalid native resource token"));
    }
    let constructor: Constructor = ctx.globals().get("WeakRef")?;
    let weak_ref: Object = constructor.construct((resource,))?;
    with_state_mut(&ctx, |state| {
        state.insert_resource(target, Persistent::save(&ctx, weak_ref));
    })?;
    Ok(())
}

pub(crate) fn parse_async_token(ctx: &Ctx<'_>, token: &Value<'_>) -> Result<(AsyncTokenKind, u64)> {
    let token = token
        .as_object()
        .ok_or_else(|| Exception::throw_type(ctx, "Invalid async resource token"))?;
    let kind: u8 = token.get("kind")?;
    let kind = match kind {
        0 => AsyncTokenKind::Native,
        1 => AsyncTokenKind::Promise,
        _ => {
            return Err(Exception::throw_type(
                ctx,
                "Invalid async resource token kind",
            ))
        },
    };
    let id: BigInt = token.get("id")?;
    let id = id
        .to_i64()
        .ok()
        .filter(|id| *id >= 0)
        .map(|id| id as u64)
        .ok_or_else(|| Exception::throw_type(ctx, "Invalid async resource token"))?;
    Ok((kind, id))
}

pub(crate) fn next_native_id(ctx: &Ctx<'_>) -> Result<(u64, u64)> {
    let async_id = next_async_id(ctx)?;
    let trigger_id = get_current_id(ctx)?.0;
    Ok((async_id, trigger_id))
}

pub(crate) fn remove_native_resource(ctx: &Ctx<'_>, async_id: u64) -> Result<()> {
    with_state_mut(ctx, |state| {
        state.remove_resource(async_id);
    })?;
    Ok(())
}

pub(crate) fn update_current_id(ctx: &Ctx<'_>, id: (u64, u64)) -> Result<()> {
    with_state_mut(ctx, |state| state.update_current_id(id))?;
    Ok(())
}

pub(crate) fn get_current_id(ctx: &Ctx<'_>) -> Result<(u64, u64)> {
    with_state(ctx, |state| state.current_id())
}

pub(crate) fn get_current_resource(ctx: Ctx<'_>) -> Result<Object<'_>> {
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

pub(crate) fn enter_async_scope(ctx: &Ctx<'_>, id: (u64, u64)) -> Result<()> {
    with_state_mut(ctx, |state| state.enter_scope(id))?;
    Ok(())
}

pub(crate) fn exit_async_scope(ctx: &Ctx<'_>, async_id: u64) -> Result<()> {
    with_state_mut(ctx, |state| state.exit_scope(async_id))?;
    Ok(())
}

pub(crate) fn cleanup(ctx: &Ctx<'_>) {
    if let Some(state) = ctx.userdata::<RefCell<AsyncResourceState>>() {
        state.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::AsyncResourceState;
    use rquickjs::{Context, Object, Persistent, Runtime};

    fn new_state<'js>(ctx: &rquickjs::Ctx<'js>) -> AsyncResourceState<'js> {
        let promise_map = Persistent::save(ctx, Object::new(ctx.clone()).unwrap());
        AsyncResourceState::new(promise_map)
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
    fn clear_removes_scope_stack_without_resetting_current_id() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();

        context.with(|ctx| {
            let mut state = new_state(&ctx);
            state.update_current_id((8, 4));
            state.enter_scope((9, 8));

            state.clear();
            state.exit_scope(9);

            assert_eq!(state.current_id(), (9, 8));
        });
    }

    #[test]
    fn update_current_id_changes_the_active_context() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();

        context.with(|ctx| {
            let mut state = new_state(&ctx);
            state.update_current_id((12, 7));

            assert_eq!(state.current_id(), (12, 7));
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
}
