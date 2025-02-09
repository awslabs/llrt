// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use dns_lookup::lookup_host;
use llrt_utils::{
    module::{export_default, ModuleInfo},
    object::ObjectExt,
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

    let mut family = 0;
    if let Some(options) = args_iter.next() {
        family = if let Some(v) = options.as_int() {
            if !matches!(v, 4 | 6) {
                () = cb.call((Exception::from_message(
                    ctx,
                    "If options is an integer, then it must be 4 or 6",
                ),))?;
                return Ok(());
            }
            v
        } else if let Ok(Some(v)) = options.get_optional::<_, i32>("famiry") {
            if !matches!(v, 4 | 6 | 0) {
                () = cb.call((Exception::from_message(
                    ctx,
                    "If family record is exist, then it must be 4, 6, or 0",
                ),))?;
                return Ok(());
            }
            v
        } else {
            0
        }
    }

    match lookup_host(&hostname) {
        Ok(ips) => {
            for ip in ips {
                if matches!(family, 4 | 0) {
                    if let Ok(ipv4) = Ipv4Addr::from_str(&ip.to_string()) {
                        () = cb.call((Null.into_js(&ctx), ipv4.to_string(), 4))?;
                        return Ok::<_, Error>(());
                    }
                }
                if matches!(family, 6 | 0) {
                    if let Ok(ipv6) = Ipv6Addr::from_str(&ip.to_string()) {
                        () = cb.call((Null.into_js(&ctx), ipv6.to_string(), 6))?;
                        return Ok::<_, Error>(());
                    }
                }
            }
        },
        Err(err) => {
            () = cb.call((Exception::from_message(ctx, &err.to_string()),))?;
            return Ok(());
        },
    }

    () = cb.call((Exception::from_message(
        ctx,
        "No values ware found matching the criteria",
    ),))?;
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
