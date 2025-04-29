// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::object::ObjectExt;
use rquickjs::{Ctx, Exception, Function, Result, Value};

#[derive(PartialEq)]
pub enum HookType {
    Init,
    Before,
    After,
}

pub enum ProviderType {
    None,
    Resource(String),   // Custom asynchronous resource
    Timeout,            // [Timeout] Timer by setTimeout()
    Immediate,          // [Immediate] Processing by setImmediate()
    Interval,           // [Interval] Timer by setInterval()
    TickObject,         // [TickObject] Processing by process.nextTick()
    TimerWrap,          // [TIMERWRAP] Internal timer wrap (low level)
    TcpWrap,            // [TCPWRAP] TCP socket wrap (net.Socket, etc.)
    UdpWrap,            // [UDPWRAP] UDP socket wrap (dgram module)
    PipeWrap,           // [PIPEWRAP] Pipe connection
    StatWatcher,        // [STATWACHER] File monitoring such as fs.watch()
    FsReqCallback,      // [FSREQCALLBACK] Callback for file system operations
    GetAddrInfoReqWrap, // [GETADDRINFOREQWRAP] When resolving DNS (dns.lookup(), etc.)
    GetNameInfoReqWrap, // [GETNAMEINFOREQWRAP] DNS reverse lookup
    TlsWrap,            // [TLSWRAP] TLS socket (HTTPS, etc.)
    MessagePort,        // [MessagePort] Port for worker_threads
}

#[allow(dependency_on_unit_never_type_fallback)]
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

    let async_ = match provider_type {
        ProviderType::None if hook_type != HookType::Init => "",
        ProviderType::None => {
            return Err(Exception::throw_type(
                ctx,
                "Asynchronous types cannot be omitted in init hooks.",
            ))
        },
        ProviderType::Resource(s) => &["Resource(", &s, ")"].concat(),
        ProviderType::Immediate => "Immediate",
        ProviderType::Interval => "Interval",
        ProviderType::Timeout => "Timeout",
        ProviderType::TimerWrap => "TIMERWRAP",
        _ => {
            return Err(Exception::throw_type(
                ctx,
                "This asynchronous types is not yet supported.",
            ))
        },
    };

    let invoke_async_hook = ctx
        .globals()
        .get_optional::<_, Function>("invokeAsyncHook")?;
    if let Some(func) = &invoke_async_hook {
        func.call((hook_, async_, uid))?;
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
