// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;

use llrt_tty::{ReadStream, WriteStream};
use llrt_utils::signals;

use llrt_utils::primordials::{BasePrimordials, Primordial};
pub use llrt_utils::sysinfo;
use llrt_utils::{
    module::{export_default, ModuleInfo},
    object::Proxy,
    result::ResultExt,
    sysinfo::{ARCH, PLATFORM},
    time, VERSION,
};
use rquickjs::Exception;
use rquickjs::{
    convert::Coerced,
    module::{Declarations, Exports, ModuleDef},
    object::{Accessor, Property},
    prelude::Func,
    Array, BigInt, Class, Ctx, Error, Function, IntoJs, Object, Persistent, Result, Value,
};

pub static EXIT_CODE: AtomicU8 = AtomicU8::new(0);

/// Drain EXIT_LISTENERS and invoke each callback with the current exit code.
/// Must be called while a live `Ctx` is available (before the runtime is freed).
/// This is called from `main` for natural (non-`process.exit()`) termination.
pub fn run_exit_listeners(ctx: &Ctx<'_>) {
    let code = EXIT_CODE.load(Ordering::Relaxed);
    let listeners: Vec<ExitListener> = {
        let mut guard = EXIT_LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut guard.0)
    };
    for listener in listeners {
        if let Ok(f) = listener.func.restore(ctx) {
            if let Err(err) = f.call::<(u32,), ()>((code as u32,)) {
                eprintln!("process exit listener error: {err}");
            }
        }
    }
}

// Exit listeners are stored on the Rust side so they are not reachable from
// arbitrary JS code via `globalThis`. `Persistent<Function>` keeps the JS
// function alive across GC cycles.
//
// SAFETY: rquickjs runs a single JS context per process and never moves
// JSRuntime/JSContext pointers across threads. The Mutex ensures exclusive
// access, so the raw pointer fields inside Persistent<Function> are only
// ever touched from the thread that holds the lock — the same thread that
// owns the JS context.
struct ExitListener {
    func: Persistent<Function<'static>>,
    /// If true, the listener should fire at most once across multiple drain calls.
    /// Currently, exit drains the entire Vec so this is always honoured naturally,
    /// but the flag is kept for future signal-handler support.
    _once: bool,
}

struct ExitListeners(Vec<ExitListener>);
unsafe impl Send for ExitListeners {}
unsafe impl Sync for ExitListeners {}

static EXIT_LISTENERS: Mutex<ExitListeners> = Mutex::new(ExitListeners(Vec::new()));

fn cwd(ctx: Ctx<'_>) -> Result<String> {
    env::current_dir()
        .or_throw(&ctx)
        .map(|path| path.to_string_lossy().to_string())
}

fn hr_time_big_int(ctx: Ctx<'_>) -> Result<BigInt<'_>> {
    let now = time::now_nanos();
    let started = time::origin_nanos();

    let elapsed = now.saturating_sub(started);

    BigInt::from_u64(ctx, elapsed)
}

fn hr_time(ctx: Ctx<'_>) -> Result<Array<'_>> {
    let now = time::now_nanos();
    let started = time::origin_nanos();
    let elapsed = now.saturating_sub(started);

    let seconds = elapsed / 1_000_000_000;
    let remaining_nanos = elapsed % 1_000_000_000;

    let array = Array::new(ctx)?;

    array.set(0, seconds)?;
    array.set(1, remaining_nanos)?;

    Ok(array)
}

fn to_exit_code(ctx: &Ctx<'_>, code: &Value<'_>) -> Result<Option<u8>> {
    if let Ok(code) = code.get::<Coerced<f64>>() {
        let code = code.0;
        let code: u8 = if code.fract() != 0.0 {
            return Err(Exception::throw_range(
                ctx,
                "The value of 'code' must be an integer",
            ));
        } else {
            (code as i32).rem_euclid(256) as u8
        };
        return Ok(Some(code));
    }
    Ok(None)
}

fn exit(ctx: Ctx<'_>, code: Value<'_>) -> Result<()> {
    let exit_code = match to_exit_code(&ctx, &code)? {
        Some(code) => code,
        None => EXIT_CODE.load(Ordering::Relaxed),
    };

    // Store the exit code before running listeners so that process.exitCode
    // returns the correct value if a listener reads it during execution.
    EXIT_CODE.store(exit_code, Ordering::Relaxed);

    // Drain and call all registered exit listeners. We drain rather than iterate
    // so that each listener is dropped (and GC-released) after it runs. Errors
    // from individual listeners are printed to stderr so all listeners run
    // regardless, matching Node.js behaviour.
    let listeners: Vec<ExitListener> = {
        let mut guard = EXIT_LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut guard.0)
    };
    for listener in listeners {
        if let Ok(f) = listener.func.restore(&ctx) {
            if let Err(err) = f.call::<(u32,), ()>((exit_code as u32,)) {
                eprintln!("process exit listener error: {err}");
            }
        }
    }

    std::process::exit(exit_code.into())
}

fn env_proxy_setter<'js>(
    target: Object<'js>,
    prop: Value<'js>,
    value: Coerced<String>,
) -> Result<bool> {
    target.set(prop, value.to_string())?;
    Ok(true)
}

#[cfg(unix)]
fn getuid() -> u32 {
    unsafe { libc::getuid() }
}

#[cfg(unix)]
fn getgid() -> u32 {
    unsafe { libc::getgid() }
}

#[cfg(unix)]
fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}

#[cfg(unix)]
fn getegid() -> u32 {
    unsafe { libc::getegid() }
}

#[cfg(unix)]
fn setuid(id: u32) -> i32 {
    unsafe { libc::setuid(id) }
}

#[cfg(unix)]
fn setgid(id: u32) -> i32 {
    unsafe { libc::setgid(id) }
}

#[cfg(unix)]
fn seteuid(id: u32) -> i32 {
    unsafe { libc::seteuid(id) }
}

#[cfg(unix)]
fn setegid(id: u32) -> i32 {
    unsafe { libc::setegid(id) }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    BasePrimordials::init(ctx)?;
    let process = Object::new(ctx.clone())?;
    let process_versions = Object::new(ctx.clone())?;
    process_versions.set("llrt", VERSION)?;
    // Node.js version - Set for compatibility with some Node.js packages (e.g. cls-hooked).
    process_versions.set("node", "0.0.0")?;

    let hr_time_fn = Function::new(ctx.clone(), hr_time)?;
    hr_time_fn.set("bigint", Func::from(hr_time_big_int))?;

    let release = Object::new(ctx.clone())?;
    release.prop("name", Property::from("llrt").enumerable())?;

    let env_map: HashMap<String, String> = env::vars().collect();
    let mut args: Vec<String> = env::args().collect();

    if let Some(arg) = args.get(1) {
        if arg == "-e" || arg == "--eval" {
            args.remove(1);
            args.remove(1);
        }
    }

    let env_obj = env_map.into_js(ctx)?;

    let env_proxy = Proxy::with_target(ctx.clone(), env_obj)?;
    env_proxy.setter(Func::from(env_proxy_setter))?;

    process.set("env", env_proxy)?;
    process.set("cwd", Func::from(cwd))?;
    process.set("argv0", args.clone().first().cloned().unwrap_or_default())?;
    process.set("id", std::process::id())?;
    process.set("argv", args)?;
    process.set("platform", PLATFORM)?;
    process.set("arch", ARCH)?;
    process.set("hrtime", hr_time_fn)?;
    process.set("release", release)?;
    process.set("version", VERSION)?;
    process.set("versions", process_versions)?;

    process.prop(
        "exitCode",
        Accessor::new(
            |ctx| {
                struct Args<'js>(Ctx<'js>);
                let Args(ctx) = Args(ctx);
                ctx.globals().get::<_, Value>("__exitCode")
            },
            |ctx, code| {
                struct Args<'js>(Ctx<'js>, Value<'js>);
                let Args(ctx, code) = Args(ctx, code);
                if let Some(code) = to_exit_code(&ctx, &code)? {
                    EXIT_CODE.store(code, Ordering::Relaxed);
                }
                ctx.globals().set("__exitCode", code)?;
                Ok::<_, Error>(())
            },
        )
        .configurable()
        .enumerable(),
    )?;
    process.set("exit", Func::from(exit))?;
    process.set(
        "kill",
        Func::from(|ctx, pid, signal| signals::kill(&ctx, pid, signal)),
    )?;

    #[cfg(unix)]
    {
        process.set("getuid", Func::from(getuid))?;
        process.set("getgid", Func::from(getgid))?;
        process.set("geteuid", Func::from(geteuid))?;
        process.set("getegid", Func::from(getegid))?;
        process.set("setuid", Func::from(setuid))?;
        process.set("setgid", Func::from(setgid))?;
        process.set("seteuid", Func::from(seteuid))?;
        process.set("setegid", Func::from(setegid))?;
    }

    // ── stdio streams ──────────────────────────────────────────────────────
    // WriteStream and ReadStream use synchronous libc writes so output is
    // immediately visible even when process.exit() is called right after.
    // WriteStream also exposes .columns / .rows / .isTTY / .setRawMode().
    // The constructors are intentionally NOT registered on globalThis —
    // they are internal implementation details exposed only via process.stdin/
    // stdout/stderr.

    let stdout_stream = Class::instance(ctx.clone(), WriteStream::new(1))?;
    process.set("stdout", stdout_stream)?;

    let stderr_stream = Class::instance(ctx.clone(), WriteStream::new(2))?;
    process.set("stderr", stderr_stream)?;

    let stdin_stream = Class::instance(ctx.clone(), ReadStream::new(0))?;
    process.set("stdin", stdin_stream)?;

    // ── process.on / process.off ───────────────────────────────────────────
    // Exit listeners are stored in a Rust-side static Vec (EXIT_LISTENERS) so
    // they are invisible to JS code and cannot be tampered with via globalThis.
    // All event-registration methods return the process object for chaining,
    // matching the Node.js EventEmitter contract.

    // process.on / process.addListener — persists the listener for every exit.
    fn on_fn<'js>(ctx: Ctx<'js>, event: String, cb: Function<'js>) -> Result<Value<'js>> {
        if event == "exit" {
            let mut guard = EXIT_LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
            // Warn at Node.js default max-listener threshold.
            if guard.0.len() >= 128 {
                eprintln!(
                    "process: MaxListenersExceededWarning: Possible memory leak detected. \
                     {} exit listeners added. Use process.off() to remove listeners.",
                    guard.0.len() + 1
                );
            }
            guard.0.push(ExitListener {
                func: Persistent::save(&ctx, cb),
                _once: false,
            });
        }
        // Other events (SIGINT, uncaughtException, etc.) are silently accepted.
        // Return the process object so callers can chain: process.on(...).on(...)
        ctx.globals().get("process")
    }
    let process_on = Function::new(ctx.clone(), on_fn)?;
    process.set("on", process_on.clone())?;
    process.set("addListener", process_on)?;

    // process.once — registers a listener that fires at most once.
    // The `once` flag is stored alongside the Persistent<Function> in the
    // ExitListener struct so no wrapper closure (and no nested Persistent) is
    // needed — avoiding the JS GC leak that a nested Persistent would cause.
    fn once_fn<'js>(ctx: Ctx<'js>, event: String, cb: Function<'js>) -> Result<Value<'js>> {
        if event == "exit" {
            let mut guard = EXIT_LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
            if guard.0.len() >= 128 {
                eprintln!(
                    "process: MaxListenersExceededWarning: Possible memory leak detected. \
                     {} exit listeners added. Use process.off() to remove listeners.",
                    guard.0.len() + 1
                );
            }
            guard.0.push(ExitListener {
                func: Persistent::save(&ctx, cb),
                _once: true,
            });
        }
        ctx.globals().get("process")
    }
    process.set("once", Function::new(ctx.clone(), once_fn)?)?;

    // process.off / process.removeListener — removes the first listener whose
    // JS identity (raw JSValue pointer) matches `cb`.
    fn off_fn<'js>(ctx: Ctx<'js>, event: String, cb: Function<'js>) -> Result<Value<'js>> {
        if event == "exit" {
            let mut guard = EXIT_LISTENERS.lock().unwrap_or_else(|e| e.into_inner());

            let cb_val: Value<'js> = cb.into_value();
            if let Some(idx) = guard.0.iter().position(|listener: &ExitListener| {
                listener
                    .func
                    .clone()
                    .restore(&ctx)
                    .map(|f| f.into_value() == cb_val)
                    .unwrap_or(false)
            }) {
                guard.0.remove(idx);
            }
        }
        ctx.globals().get("process")
    }
    let process_off = Function::new(ctx.clone(), off_fn)?;
    process.set("removeListener", process_off.clone())?;
    process.set("off", process_off)?;

    globals.set("process", process)?;

    Ok(())
}

pub struct ProcessModule;

impl ModuleDef for ProcessModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("env")?;
        declare.declare("cwd")?;
        declare.declare("argv0")?;
        declare.declare("id")?;
        declare.declare("argv")?;
        declare.declare("platform")?;
        declare.declare("arch")?;
        declare.declare("hrtime")?;
        declare.declare("release")?;
        declare.declare("version")?;
        declare.declare("versions")?;
        declare.declare("exitCode")?;
        declare.declare("exit")?;
        declare.declare("kill")?;
        declare.declare("stdout")?;
        declare.declare("stderr")?;
        declare.declare("stdin")?;
        declare.declare("on")?;
        declare.declare("once")?;
        declare.declare("addListener")?;
        declare.declare("removeListener")?;
        declare.declare("off")?;

        #[cfg(unix)]
        {
            declare.declare("getuid")?;
            declare.declare("getgid")?;
            declare.declare("geteuid")?;
            declare.declare("getegid")?;
            declare.declare("setuid")?;
            declare.declare("setgid")?;
            declare.declare("seteuid")?;
            declare.declare("setegid")?;
        }

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();
        let process: Object = globals.get("process")?;

        export_default(ctx, exports, |default| {
            for name in process.keys::<String>() {
                let name = name?;
                let value: Value = process.get(&name)?;
                default.set(name, value)?;
            }

            Ok(())
        })?;

        Ok(())
    }
}

impl From<ProcessModule> for ModuleInfo<ProcessModule> {
    fn from(val: ProcessModule) -> Self {
        ModuleInfo {
            name: "process",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use super::*;

    async fn setup(ctx: &Ctx<'_>) {
        time::init();
        init(ctx).unwrap();
        ModuleEvaluator::eval_rust::<ProcessModule>(ctx.clone(), "process")
            .await
            .unwrap();
    }

    // ── hrtime (pre-existing) ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_hr_time() {
        test_async_with(|ctx| {
            Box::pin(async move {
                setup(&ctx).await;
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { hrtime } from 'process';

                        export async function test() {
                            // TODO: Delaying with setTimeout
                            for(let i=0; i < (1<<20); i++){}
                            return hrtime()
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<Vec<u32>, _>(&ctx, &module, ()).await;
                assert_eq!(result.len(), 2);
                assert_eq!(result[0], 0);
                assert!(result[1] > 0);
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_hr_time_bigint() {
        test_async_with(|ctx| {
            Box::pin(async move {
                setup(&ctx).await;
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { hrtime } from 'process';

                        export async function test() {
                            // TODO: Delaying with setTimeout
                            for(let i=0; i < (1<<20); i++){}
                            return hrtime.bigint()
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<Coerced<i64>, _>(&ctx, &module, ()).await;
                assert!(result.0 > 0);
            })
        })
        .await;
    }
}
