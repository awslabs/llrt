// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_context::CtxExtension;
use llrt_dns_cache::lookup_host;
use llrt_hooking::{invoke_async_hook, register_finalization_registry, HookType};
use llrt_utils::{
    module::{export_default, ModuleInfo},
    provider::ProviderType,
    result::ResultExt,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest},
    qjs, Ctx, Error, Exception, Function, IntoJs, Null, Result, Value,
};

fn lookup<'js>(ctx: Ctx<'js>, hostname: String, args: Rest<Value<'js>>) -> Result<()> {
    let mut args_iter = args.0.into_iter().rev();
    let cb: Function = args_iter
        .next()
        .and_then(|v| v.into_function())
        .or_throw_msg(&ctx, "Callback parameter is not a function")?;

    // SAFETY: Since it checks in advance whether it is an Function type, we can always get a pointer to the Function.
    let uid = unsafe { qjs::JS_VALUE_GET_PTR(cb.as_raw()) } as usize;
    register_finalization_registry(&ctx, cb.clone().into_value(), uid)?;
    invoke_async_hook(&ctx, HookType::Init, ProviderType::GetAddrInfoReqWrap, uid)?;

    ctx.clone().spawn_exit(async move {
        match lookup_host(&hostname, args_iter.next()).await {
            Ok((address, family)) => {
                invoke_async_hook(&ctx, HookType::Before, ProviderType::None, uid)?;
                () = cb.call((Null.into_js(&ctx), address, family))?;
                invoke_async_hook(&ctx, HookType::After, ProviderType::None, uid)?;
                Ok::<_, Error>(())
            },
            Err(err) => {
                invoke_async_hook(&ctx, HookType::Before, ProviderType::None, uid)?;
                () = cb.call((Exception::from_message(ctx.clone(), &err.to_string()),))?;
                invoke_async_hook(&ctx, HookType::After, ProviderType::None, uid)?;
                Ok(())
            },
        }
    })?;
    Ok(())
}

pub struct DnsModule;

impl ModuleDef for DnsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("lookup")?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("lookup", Func::from(lookup))?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<DnsModule> for ModuleInfo<DnsModule> {
    fn from(val: DnsModule) -> Self {
        ModuleInfo {
            name: "dns",
            module: val,
        }
    }
}
