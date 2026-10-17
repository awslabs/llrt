// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_async_context::{lifecycle::AsyncResourceHandle, AsyncResourceKind};
use rquickjs::{
    class::Trace,
    prelude::{Opt, Rest, This},
    Class, Ctx, Function, JsLifetime, Object, Persistent, Result, Value,
};

use crate::{
    async_hooks_bridge::register_async_resource_target,
    binding::{capture_snapshot, run_with_snapshot, AsyncLocalStorageSnapshot},
};

#[derive(Trace)]
#[rquickjs::class]
pub(crate) struct AsyncResource<'js> {
    #[qjs(skip_trace)]
    handle: AsyncResourceHandle,
    initialized: bool,
    #[qjs(skip_trace)]
    snapshot: AsyncLocalStorageSnapshot<'js>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncResource<'js> {
    type Changed<'to> = AsyncResource<'to>;
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> AsyncResource<'js> {
    #[qjs(constructor)]
    pub(crate) fn new(
        ctx: Ctx<'js>,
        _resource_type: String,
        options: Opt<Object<'js>>,
    ) -> Result<Self> {
        let trigger_id = options
            .0
            .map(|options| options.get::<_, Option<u64>>("triggerAsyncId"))
            .transpose()?
            .flatten();
        Ok(Self {
            handle: AsyncResourceHandle::new(&ctx, trigger_id)?,
            initialized: false,
            snapshot: capture_snapshot(&ctx)?,
        })
    }

    pub(crate) fn async_id(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<u64> {
        register_resource(&ctx, &this.0)?;
        Ok(this.borrow().handle.async_id())
    }

    pub(crate) fn trigger_async_id(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<u64> {
        register_resource(&ctx, &this.0)?;
        Ok(this.borrow().handle.trigger_id())
    }

    pub(crate) fn run_in_async_scope(
        this: This<Class<'js, AsyncResource<'js>>>,
        ctx: Ctx<'js>,
        callback: Function<'js>,
        this_arg: Opt<Value<'js>>,
        args: Rest<Value<'js>>,
    ) -> Result<Value<'js>> {
        register_resource(&ctx, &this.0)?;
        let (handle, snapshot) = {
            let resource = this.borrow();
            (resource.handle.clone(), resource.snapshot.clone())
        };
        let this_arg = this_arg
            .0
            .unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        handle.run(&ctx, || {
            run_with_snapshot(&snapshot, handle.async_id(), || {
                callback.call::<_, Value>((This(this_arg), args))
            })
        })
    }

    pub(crate) fn bind(
        this: This<Class<'js, Self>>,
        ctx: Ctx<'js>,
        callback: Function<'js>,
        this_arg: Opt<Value<'js>>,
    ) -> Result<Function<'js>> {
        register_resource(&ctx, &this)?;
        let (handle, snapshot) = {
            let resource = this.borrow();
            (resource.handle.clone(), resource.snapshot.clone())
        };
        let root = Persistent::save(&ctx, this.0.clone().into_value());
        let this_arg = this_arg
            .0
            .unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        create_bound_function(ctx, handle, snapshot, callback, this_arg, root)
    }

    #[qjs(static, rename = "bind")]
    pub(crate) fn bind_static(
        ctx: Ctx<'js>,
        callback: Function<'js>,
        _resource_type: Opt<String>,
        this_arg: Opt<Value<'js>>,
    ) -> Result<Function<'js>> {
        let resource = Class::instance(
            ctx.clone(),
            Self {
                handle: AsyncResourceHandle::new(&ctx, None)?,
                initialized: false,
                snapshot: capture_snapshot(&ctx)?,
            },
        )?;
        register_resource(&ctx, &resource)?;
        let (handle, snapshot) = {
            let resource = resource.borrow();
            (resource.handle.clone(), resource.snapshot.clone())
        };
        let root = Persistent::save(&ctx, resource.into_value());
        let this_arg = this_arg
            .0
            .unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        create_bound_function(ctx, handle, snapshot, callback, this_arg, root)
    }
}

fn create_bound_function<'js>(
    ctx: Ctx<'js>,
    handle: AsyncResourceHandle,
    snapshot: AsyncLocalStorageSnapshot<'js>,
    callback: Function<'js>,
    this_arg: Value<'js>,
    resource_root: Persistent<Value<'static>>,
) -> Result<Function<'js>> {
    let call_ctx = ctx.clone();
    Function::new(ctx, move |args: Rest<Value<'js>>| {
        let _resource = resource_root.clone().restore(&call_ctx)?;
        handle.run(&call_ctx, || {
            run_with_snapshot(&snapshot, handle.async_id(), || {
                callback.call::<_, Value>((This(this_arg.clone()), args))
            })
        })
    })
}

fn register_resource<'js>(ctx: &Ctx<'js>, this: &Class<'js, AsyncResource<'js>>) -> Result<()> {
    let (handle, should_register) = {
        let mut resource = this.borrow_mut();
        let should_register = !resource.initialized;
        resource.initialized = true;
        (resource.handle.clone(), should_register)
    };
    if should_register {
        let target: Value = this.clone().into_value();
        if let Err(error) = register_async_resource_target(
            ctx,
            target,
            AsyncResourceKind::Native,
            handle.async_id(),
            handle.trigger_id(),
        ) {
            this.borrow_mut().initialized = false;
            return Err(error);
        }
    }
    Ok(())
}
