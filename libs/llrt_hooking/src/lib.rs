// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{borrow::Cow, env};

use llrt_utils::provider::ProviderType;
use once_cell::sync::Lazy;
use rquickjs::{
    function::This, BigInt, Ctx, Exception, Function, JsLifetime, Object, Persistent, Result, Value,
};

static HOOKING_MODE: Lazy<bool> = Lazy::new(|| env::var("LLRT_ASYNC_HOOKS").as_deref() == Ok("1"));

#[inline]
pub fn is_hooking_enabled() -> bool {
    *HOOKING_MODE
}

#[derive(PartialEq)]
pub enum HookType {
    Init,
    Before,
    After,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsyncTokenKind {
    Native = 0,
    Promise = 1,
}

pub struct AsyncHookBridge {
    pub registry: Persistent<Object<'static>>,
    pub register: Persistent<Function<'static>>,
    pub register_async_resource: Persistent<Function<'static>>,
    pub invoke_async_hook: Persistent<Function<'static>>,
}

unsafe impl<'js> JsLifetime<'js> for AsyncHookBridge {
    type Changed<'to> = AsyncHookBridge;
}

pub fn invoke_async_hook(
    ctx: &Ctx<'_>,
    hook_type: HookType,
    provider_type: ProviderType,
    async_id: u64,
    trigger_id: u64,
) -> Result<(u64, u64)> {
    if !is_hooking_enabled() {
        return Ok((0, 0));
    }

    let hook_ = match hook_type {
        HookType::Init => "init",
        HookType::Before => "before",
        HookType::After => "after",
    };

    let provider_: Cow<'_, str> = match provider_type {
        ProviderType::None if hook_type != HookType::Init => Cow::Borrowed(""),
        ProviderType::None => {
            return Err(Exception::throw_type(
                ctx,
                "Asynchronous types cannot be omitted in init hooks.",
            ))
        },
        ProviderType::Resource(s) => Cow::Owned(format!("Resource({s})")),
        // Userland provider types
        ProviderType::Immediate => Cow::Borrowed("Immediate"),
        ProviderType::Interval => Cow::Borrowed("Interval"),
        ProviderType::MessagePort => Cow::Borrowed("MessagePort"),
        ProviderType::Microtask => Cow::Borrowed("Microtask"),
        ProviderType::TickObject => Cow::Borrowed("TickObject"),
        ProviderType::Timeout => Cow::Borrowed("Timeout"),
        // Internal provider types
        ProviderType::FsReqCallback => Cow::Borrowed("FSREQCALLBACK"),
        ProviderType::GetAddrInfoReqWrap => Cow::Borrowed("GETADDRINFOREQWRAP"),
        ProviderType::GetNameInfoReqWrap => Cow::Borrowed("GETNAMEINFOREQWRAP"),
        ProviderType::PipeWrap => Cow::Borrowed("PIPEWRAP"),
        ProviderType::StatWatcher => Cow::Borrowed("STATWACHER"),
        ProviderType::TcpWrap => Cow::Borrowed("TCPWRAP"),
        ProviderType::TimerWrap => Cow::Borrowed("TIMERWRAP"),
        ProviderType::TlsWrap => Cow::Borrowed("TLSWRAP"),
        ProviderType::UdpWrap => Cow::Borrowed("UDPWRAP"),
    };

    let stored = ctx
        .userdata::<AsyncHookBridge>()
        .ok_or_else(|| Exception::throw_internal(ctx, "AsyncHookBridge is not initialized"))?;
    let invoke_async_hook = stored.invoke_async_hook.clone().restore(ctx)?;
    let result: Object =
        invoke_async_hook.call((hook_, provider_.as_ref(), async_id, trigger_id))?;
    let async_id = result.get::<_, BigInt>("asyncId")?.to_i64()? as u64;
    let trigger_id = result.get::<_, BigInt>("triggerId")?.to_i64()? as u64;
    Ok((async_id, trigger_id))
}

pub fn register_finalization_registry<'js>(
    ctx: &Ctx<'js>,
    target: Value<'js>,
    kind: AsyncTokenKind,
    async_id: u64,
    trigger_id: u64,
) -> Result<()> {
    if !is_hooking_enabled() || async_id == 0 {
        return Ok(());
    }

    let (registry, register, register_async_resource) = {
        let stored = ctx
            .userdata::<AsyncHookBridge>()
            .ok_or_else(|| Exception::throw_internal(ctx, "AsyncHookBridge is not initialized"))?;
        (
            stored.registry.clone().restore(ctx)?,
            stored.register.clone(),
            stored.register_async_resource.clone(),
        )
    };
    let register = register.restore(ctx)?;
    let token = Object::new(ctx.clone())?;
    token.set("kind", kind as u8)?;
    token.set("id", BigInt::from_u64(ctx.clone(), async_id)?)?;
    token.set("triggerId", BigInt::from_u64(ctx.clone(), trigger_id)?)?;
    register.call::<_, ()>((This(registry), target.clone(), token.clone()))?;
    if kind == AsyncTokenKind::Native {
        let register_resource = register_async_resource.restore(ctx)?;
        register_resource.call::<_, ()>((token, target))?;
    }
    Ok(())
}
