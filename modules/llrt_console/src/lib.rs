// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::io::{stderr, stdout, IsTerminal, Write};

use llrt_logging::{build_formatted_string, FormatOptions, NEWLINE};
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    object::Property,
    prelude::{Func, Rest},
    Class, Ctx, Object, Result, Value,
};

#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct Console {}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl Console {
    #[qjs(constructor)]
    pub fn new() -> Self {
        // We ignore the parameters for now since we don't support stream
        Self {}
    }

    pub fn log<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log(ctx, args)
    }
    pub fn clear(&self) {
        clear()
    }
    pub fn debug<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_debug(ctx, args)
    }
    pub fn info<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log(ctx, args)
    }
    pub fn trace<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_trace(ctx, args)
    }
    pub fn error<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_error(ctx, args)
    }
    pub fn warn<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_warn(ctx, args)
    }
    pub fn assert<'js>(
        &self,
        ctx: Ctx<'js>,
        expression: bool,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        log_assert(ctx, expression, args)
    }
}

pub fn log_fatal<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stderr(), &ctx, args)
}

pub fn log_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stderr(), &ctx, args)
}

fn log_warn<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stderr(), &ctx, args)
}

fn log_debug<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stdout(), &ctx, args)
}

fn log_trace<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stdout(), &ctx, args)
}

fn log_assert<'js>(ctx: Ctx<'js>, expression: bool, args: Rest<Value<'js>>) -> Result<()> {
    if !expression {
        write_log(stderr(), &ctx, args)?;
    }
    Ok(())
}

fn log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stdout(), &ctx, args)
}

fn clear() {
    let _ = stdout().write_all(b"\x1b[1;1H\x1b[0J");
}

fn write_log<'js, T>(mut output: T, ctx: &Ctx<'js>, args: Rest<Value<'js>>) -> Result<()>
where
    T: Write + IsTerminal,
{
    let is_tty = output.is_terminal();
    let mut result = String::new();

    let mut options = FormatOptions::new(ctx, is_tty, true)?;
    build_formatted_string(&mut result, ctx, args, &mut options)?;

    result.push(NEWLINE);

    //we don't care if output is interrupted
    let _ = output.write_all(result.as_bytes());

    Ok(())
}

pub struct ConsoleModule;

impl ModuleDef for ConsoleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(Console))?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<Console>::define(default)?;

            Ok(())
        })
    }
}

impl From<ConsoleModule> for ModuleInfo<ConsoleModule> {
    fn from(val: ConsoleModule) -> Self {
        ModuleInfo {
            name: "console",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    // NOTE: Console must be created from an empty object with no prototype.
    // https://console.spec.whatwg.org/#console-namespace
    let console = ctx.eval::<Object, &str>("Object.create({})")?;

    console.set("assert", Func::from(log_assert))?;
    console.set("clear", Func::from(clear))?;
    console.set("debug", Func::from(log_debug))?;
    console.set("error", Func::from(log_error))?;
    console.set("info", Func::from(log))?;
    console.set("log", Func::from(log))?;
    console.set("trace", Func::from(log_trace))?;
    console.set("warn", Func::from(log_warn))?;
    console.prop(
        PredefinedAtom::SymbolToStringTag,
        Property::from("console").configurable(),
    )?;

    globals.prop("console", Property::from(console).writable().configurable())?;

    Ok(())
}
