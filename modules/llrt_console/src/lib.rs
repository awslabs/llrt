// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{stderr, stdout, IsTerminal, Write},
};

use llrt_logging::{build_formatted_string, FormatOptions, NEWLINE};
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    object::Property,
    prelude::{Func, Opt, Rest},
    Class, Coerced, Ctx, Object, Result, Value,
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

pub fn log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
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

/// Sets up the global `console` object with the correct property descriptors,
pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    // Per-context console state (counters/timers). Silently ignore the
    // `Err` variant from `store_userdata` — it only returns `Err` when the
    // userdata is currently being accessed, which is impossible here since
    // `init` runs before any console method is registered.
    let _ = ctx.store_userdata(ConsoleState::default());

    let globals = ctx.globals();

    let proto = Object::new(ctx.clone())?;
    let console = Object::new_proto(ctx.clone(), Some(&proto))?;

    console.set("assert", Func::from(log_assert))?;
    console.set("clear", Func::from(clear))?;
    console.set("debug", Func::from(log_debug))?;
    console.set("error", Func::from(log_error))?;
    console.set("info", Func::from(log))?;
    console.set("log", Func::from(log))?;
    console.set("trace", Func::from(log_trace))?;
    console.set("warn", Func::from(log_warn))?;
    console.set("count", Func::from(console_count))?;
    console.set("countReset", Func::from(console_count_reset))?;
    console.set("time", Func::from(console_time))?;
    console.set("timeLog", Func::from(console_time_log))?;
    console.set("timeEnd", Func::from(console_time_end))?;
    console.prop(
        PredefinedAtom::SymbolToStringTag,
        Property::from("console").configurable(),
    )?;
    globals.prop("console", Property::from(console).writable().configurable())?;

    Ok(())
}

/// Per-context state for the `console.count()` / `console.time()` families.
/// Stored in `Ctx::userdata` so two contexts in the same runtime don't share
/// timers and counters (a `thread_local!` would conflate them on worker
/// runtimes that run multiple contexts on the same OS thread).
#[derive(Default)]
struct ConsoleState {
    counters: RefCell<HashMap<String, u32>>,
    timers: RefCell<HashMap<String, std::time::Instant>>,
}

unsafe impl<'js> rquickjs::JsLifetime<'js> for ConsoleState {
    type Changed<'to> = ConsoleState;
}

fn with_state<R>(ctx: &Ctx<'_>, f: impl FnOnce(&ConsoleState) -> R) -> R {
    // `init_console` stores a `ConsoleState` before any console.count/time
    // method can be called, so the guard is always present. If a caller
    // somehow runs these functions before init, fall back to a transient
    // state (matches previous `thread_local!` behaviour of starting fresh).
    match ctx.userdata::<ConsoleState>() {
        Some(guard) => f(&guard),
        None => f(&ConsoleState::default()),
    }
}

fn get_label(label: Opt<Coerced<String>>) -> String {
    label
        .into_inner()
        .map(|c| c.0)
        .unwrap_or_else(|| "default".to_string())
}

fn console_count(ctx: Ctx<'_>, label: Opt<Coerced<String>>) {
    let label = get_label(label);
    with_state(&ctx, |state| {
        let mut map = state.counters.borrow_mut();
        let count = map.entry(label.clone()).or_insert(0);
        *count += 1;
        let _ = writeln!(stdout(), "{}: {}", label, count);
    });
}

fn console_count_reset(ctx: Ctx<'_>, label: Opt<Coerced<String>>) {
    let label = get_label(label);
    with_state(&ctx, |state| {
        state.counters.borrow_mut().remove(&label);
    });
}

fn console_time(ctx: Ctx<'_>, label: Opt<Coerced<String>>) {
    let label = get_label(label);
    with_state(&ctx, |state| {
        state
            .timers
            .borrow_mut()
            .entry(label)
            .or_insert_with(std::time::Instant::now);
    });
}

fn console_time_log<'js>(
    ctx: Ctx<'js>,
    label: Opt<Coerced<String>>,
    args: Rest<Value<'js>>,
) -> Result<()> {
    let label = get_label(label);
    let elapsed = with_state(&ctx, |state| {
        state
            .timers
            .borrow()
            .get(&label)
            .map(|start| start.elapsed().as_millis())
    });
    let Some(elapsed) = elapsed else {
        return Ok(());
    };
    let _ = write!(stdout(), "{}: {}ms", label, elapsed);
    if !args.0.is_empty() {
        let _ = write!(stdout(), " ");
        let mut options = FormatOptions::new(&ctx, stdout().is_terminal(), true)?;
        let mut result = String::new();
        build_formatted_string(&mut result, &ctx, args, &mut options)?;
        let _ = write!(stdout(), "{}", result);
    }
    let _ = writeln!(stdout());
    Ok(())
}

fn console_time_end(ctx: Ctx<'_>, label: Opt<Coerced<String>>) {
    let label = get_label(label);
    let elapsed = with_state(&ctx, |state| {
        state
            .timers
            .borrow_mut()
            .remove(&label)
            .map(|start| start.elapsed().as_millis())
    });
    if let Some(elapsed) = elapsed {
        let _ = writeln!(stdout(), "{}: {}ms", label, elapsed);
    }
}
