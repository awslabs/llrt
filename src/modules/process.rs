// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::HashMap, env, sync::atomic::Ordering};

use rquickjs::{
    atom::PredefinedAtom,
    convert::Coerced,
    function::Constructor,
    module::{Declarations, Exports, ModuleDef},
    object::Property,
    prelude::Func,
    Array, BigInt, Ctx, Function, IntoJs, Object, Result, Value,
};

use chrono::Utc;

use crate::modules::module::export_default;

use crate::VERSION;

use crate::vm::TIME_ORIGIN;

fn cwd() -> String {
    env::current_dir().unwrap().to_string_lossy().to_string()
}

pub fn get_arch() -> &'static str {
    let arch = env::consts::ARCH;

    match arch {
        "x86_64" | "x86" => return "x64",
        "aarch64" => return "arm64",
        _ => (),
    }

    arch
}

pub fn get_platform() -> &'static str {
    let platform = env::consts::OS;
    match platform {
        "macos" => "darwin",
        "windows" => "win32",
        _ => platform,
    }
}

fn hr_time_big_int(ctx: Ctx<'_>) -> Result<BigInt> {
    let now = Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64;
    let started = TIME_ORIGIN.load(Ordering::Relaxed) as u64;

    let elapsed = now - started;

    BigInt::from_u64(ctx, elapsed)
}

fn hr_time(ctx: Ctx<'_>) -> Result<Array<'_>> {
    let now = Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64;
    let started = TIME_ORIGIN.load(Ordering::Relaxed) as u64;
    let elapsed = now - started;

    let seconds = elapsed / 1_000_000_000;
    let remaining_nanos = elapsed % 1_000_000_000;

    let array = Array::new(ctx)?;

    array.set(0, seconds)?;
    array.set(1, remaining_nanos)?;

    Ok(array)
}

fn exit(code: i32) {
    std::process::exit(code)
}

fn env_proxy_setter<'js>(
    target: Object<'js>,
    prop: Value<'js>,
    value: Coerced<String>,
) -> Result<bool> {
    target.set(prop, value.to_string())?;
    Ok(true)
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let process = Object::new(ctx.clone())?;
    let process_versions = Object::new(ctx.clone())?;
    process_versions.set("llrt", VERSION)?;

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

    let proxy_ctor = globals.get::<_, Constructor>(PredefinedAtom::Proxy)?;

    let env_obj = env_map.into_js(ctx)?;
    let env_proxy_cfg = Object::new(ctx.clone())?;
    env_proxy_cfg.set(PredefinedAtom::Setter, Func::from(env_proxy_setter))?;
    let env_proxy = proxy_ctor.construct::<_, Value>((env_obj, env_proxy_cfg))?;

    process.set("env", env_proxy)?;
    process.set("cwd", Func::from(cwd))?;
    process.set("argv0", args.clone().first().cloned().unwrap_or_default())?;
    process.set("id", std::process::id())?;
    process.set("argv", args)?;
    process.set("platform", get_platform())?;
    process.set("arch", get_arch())?;
    process.set("hrtime", hr_time)?;
    process.set("release", release)?;
    process.set("version", VERSION)?;
    process.set("versions", process_versions)?;
    process.set("exit", Func::from(exit))?;

    globals.set("process", process)?;

    Ok(())
}

pub struct ProcessModule;

impl ModuleDef for ProcessModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
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
        declare.declare("exit")?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
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
