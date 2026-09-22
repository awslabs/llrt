// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    borrow::Cow,
    sync::atomic::{AtomicUsize, Ordering},
};

use rquickjs::{
    function::This, BigInt, Ctx, Exception, Function, JsLifetime, Object, Persistent, Result, Value,
};

static HOOKING_USERS: AtomicUsize = AtomicUsize::new(0);

#[derive(PartialEq)]
pub enum ProviderType {
    None,
    Resource(String),
    Immediate,
    Interval,
    MessagePort,
    Microtask,
    TickObject,
    Timeout,
    FsReqCallback,
    GetAddrInfoReqWrap,
    GetNameInfoReqWrap,
    PipeWrap,
    StatWatcher,
    TcpWrap,
    TimerWrap,
    TlsWrap,
    UdpWrap,
}

#[inline]
pub fn is_hooking_enabled() -> bool {
    HOOKING_USERS.load(Ordering::Relaxed) != 0
}

pub fn acquire_hooking() {
    HOOKING_USERS.fetch_add(1, Ordering::Relaxed);
}

pub fn release_hooking() {
    let _ = HOOKING_USERS.try_update(Ordering::Relaxed, Ordering::Relaxed, |users| {
        users.checked_sub(1)
    });
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
    pub finalization_registry: Persistent<Object<'static>>,
    pub finalization_register: Persistent<Function<'static>>,
    pub async_resource_register: Persistent<Function<'static>>,
    pub async_hook_invoker: Persistent<Function<'static>>,
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

    let Some(stored) = ctx.userdata::<AsyncHookBridge>() else {
        return Ok((0, 0));
    };
    let async_hook_invoker = stored.async_hook_invoker.clone().restore(ctx)?;
    let result: Object =
        async_hook_invoker.call((hook_, provider_.as_ref(), async_id, trigger_id))?;
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

    let Some(stored) = ctx.userdata::<AsyncHookBridge>() else {
        return Ok(());
    };

    let token = Object::new(ctx.clone())?;
    token.set("kind", kind as u8)?;
    token.set("id", BigInt::from_u64(ctx.clone(), async_id)?)?;
    token.set("triggerId", BigInt::from_u64(ctx.clone(), trigger_id)?)?;

    let registry = stored.finalization_registry.clone().restore(ctx)?;
    let register = stored.finalization_register.clone().restore(ctx)?;
    register.call::<_, ()>((This(registry), target.clone(), token.clone()))?;

    if kind == AsyncTokenKind::Native {
        let async_resource_register = stored.async_resource_register.clone().restore(ctx)?;
        async_resource_register.call::<_, ()>((token, target))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{acquire_hooking, is_hooking_enabled, release_hooking, HOOKING_USERS};
    use std::sync::atomic::Ordering;

    #[test]
    fn tracks_users_without_underflowing() {
        HOOKING_USERS.store(0, Ordering::Relaxed);
        assert!(!is_hooking_enabled());

        acquire_hooking();
        acquire_hooking();
        assert!(is_hooking_enabled());

        release_hooking();
        assert!(is_hooking_enabled());
        release_hooking();
        assert!(!is_hooking_enabled());

        release_hooking();
        assert!(!is_hooking_enabled());
    }
}
