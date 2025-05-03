// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{object::ObjectExt, provider::ProviderType};
use rquickjs::{Ctx, Exception, Function, Result, Value};

#[derive(PartialEq)]
pub enum HookType {
    Init,
    Before,
    After,
}

pub fn invoke_async_hook(
    ctx: &Ctx<'_>,
    hook_type: HookType,
    provider_type: ProviderType,
    uid: usize,
) -> Result<()> {
    let hook_ = match hook_type {
        HookType::Init => "init",
        HookType::Before => "before",
        HookType::After => "after",
    };

    let provider_ = match provider_type {
        ProviderType::None if hook_type != HookType::Init => "",
        ProviderType::None => {
            return Err(Exception::throw_type(
                ctx,
                "Asynchronous types cannot be omitted in init hooks.",
            ))
        },
        ProviderType::Resource(s) => &["Resource(", &s, ")"].concat(),
        // Userland provider types
        ProviderType::Immediate => "Immediate",
        ProviderType::Interval => "Interval",
        ProviderType::MessagePort => "MessagePort",
        ProviderType::Microtask => "Microtask",
        ProviderType::TickObject => "TickObject",
        ProviderType::Timeout => "Timeout",
        // Internal provider types
        ProviderType::FsReqCallback => "FSREQCALLBACK",
        ProviderType::GetAddrInfoReqWrap => "GETADDRINFOREQWRAP",
        ProviderType::GetNameInfoReqWrap => "GETNAMEINFOREQWRAP",
        ProviderType::PipeWrap => "PIPEWRAP",
        ProviderType::StatWatcher => "STATWACHER",
        ProviderType::TcpWrap => "TCPWRAP",
        ProviderType::TimerWrap => "TIMERWRAP",
        ProviderType::TlsWrap => "TLSWRAP",
        ProviderType::UdpWrap => "UDPWRAP",
    };

    let invoke_async_hook = ctx
        .globals()
        .get_optional::<_, Function>("invokeAsyncHook")?;
    if let Some(func) = &invoke_async_hook {
        func.call::<_, ()>((hook_, provider_, uid))?;
    }
    Ok(())
}

pub fn register_finalization_registry<'js>(
    ctx: &Ctx<'js>,
    target: Value<'js>,
    uid: usize,
) -> Result<()> {
    if let Ok(register) =
        ctx.eval::<Function<'js>, &str>("globalThis.asyncFinalizationRegistry.register")
    {
        let _ = register.call::<_, ()>((target, uid));
    }
    Ok(())
}
