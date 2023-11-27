use std::{
    collections::HashMap,
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use rquickjs::{
    atom::PredefinedAtom, convert::Coerced, function::Constructor, object::Property, prelude::Func,
    Ctx, IntoJs, Object, Result, Value,
};

use crate::VERSION;

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
    if platform == "macos" {
        return "darwin";
    }
    platform
}

fn current_time_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
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

    let release = Object::new(ctx.clone())?;
    release.prop("name", Property::from("llrt").enumerable())?;

    let hr_time = Object::new(ctx.clone())?;
    hr_time.set("bigint", Func::from(current_time_micros))?;

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
    process.set("exit", Func::from(exit))?;

    globals.set("process", process)?;

    Ok(())
}
