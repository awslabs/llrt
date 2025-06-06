// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicU8, Ordering};

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
    Array, BigInt, Ctx, Error, Function, IntoJs, Object, Result, Value,
};

pub static EXIT_CODE: AtomicU8 = AtomicU8::new(0);

fn cwd(ctx: Ctx<'_>) -> Result<String> {
    env::current_dir()
        .or_throw(&ctx)
        .map(|path| path.to_string_lossy().to_string())
}

fn hr_time_big_int(ctx: Ctx<'_>) -> Result<BigInt<'_>> {
    let now = time::now_nanos();
    let started = time::origin_nanos();

    let elapsed = now.checked_sub(started).unwrap_or_default();

    BigInt::from_u64(ctx, elapsed)
}

fn hr_time(ctx: Ctx<'_>) -> Result<Array<'_>> {
    let now = time::now_nanos();
    let started = time::origin_nanos();
    let elapsed = now.checked_sub(started).unwrap_or_default();

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
    let code = match to_exit_code(&ctx, &code)? {
        Some(code) => code,
        None => EXIT_CODE.load(Ordering::Relaxed),
    };
    std::process::exit(code.into())
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
    let process = Object::new(ctx.clone())?;
    let process_versions = Object::new(ctx.clone())?;
    process_versions.set("llrt", VERSION)?;
    // Node.js version - Set for compatibility with some Node.js packages (e.g. cls-hooked).
    process_versions.set("node", "0.0.0")?;

    let hr_time = Function::new(ctx.clone(), hr_time)?;
    hr_time.set("bigint", Func::from(hr_time_big_int))?;

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
    process.set("hrtime", hr_time)?;
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

    #[tokio::test]
    async fn test_hr_time() {
        time::init();
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<ProcessModule>(ctx.clone(), "process")
                    .await
                    .unwrap();

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
        time::init();
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<ProcessModule>(ctx.clone(), "process")
                    .await
                    .unwrap();

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
