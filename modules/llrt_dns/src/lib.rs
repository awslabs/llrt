// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_dns_cache::lookup_host;
use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest},
    Ctx, Error, Exception, Function, IntoJs, Null, Result, Value,
};

fn lookup<'js>(ctx: Ctx<'js>, hostname: String, args: Rest<Value<'js>>) -> Result<()> {
    let mut args_iter = args.0.into_iter().rev();
    let cb: Function = args_iter
        .next()
        .and_then(|v| v.into_function())
        .or_throw_msg(&ctx, "Callback parameter is not a function")?;

    match lookup_host(&ctx, &hostname, args_iter.next()) {
        Ok((address, family)) => {
            () = cb.call((Null.into_js(&ctx), address, family))?;
            Ok::<_, Error>(())
        },
        Err(err) => {
            () = cb.call((Exception::from_message(ctx, &err.to_string()),))?;
            Ok(())
        },
    }
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
