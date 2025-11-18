// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_events::Emitter;
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Class, Ctx, Result, Value,
};

mod socket;

use self::socket::Socket;

pub struct DgramModule;

impl ModuleDef for DgramModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createSocket")?;
        declare.declare(stringify!(Socket))?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<Socket>::define(default)?;
            Socket::add_event_emitter_prototype(ctx)?;

            default.set(
                "createSocket",
                Func::from(|ctx: Ctx<'js>, type_or_options: Value<'js>| {
                    Socket::ctor(ctx, type_or_options)
                }),
            )?;

            Ok(())
        })?;
        Ok(())
    }
}

impl From<DgramModule> for ModuleInfo<DgramModule> {
    fn from(val: DgramModule) -> Self {
        ModuleInfo {
            name: "dgram",
            module: val,
        }
    }
}
