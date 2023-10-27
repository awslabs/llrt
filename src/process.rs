use std::{
    collections::HashMap,
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use rquickjs::{prelude::Func, Ctx, Object, Result};

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

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let process = Object::new(ctx.clone())?;

    let release = Object::new(ctx.clone())?;
    release.prop("name", "llrt")?;

    let hr_time = Object::new(ctx.clone())?;
    hr_time.set("bigint", Func::from(current_time_micros))?;

    let env_map: HashMap<String, String> = env::vars().collect();
    let args: Vec<String> = env::args().collect();

    process.set("env", env_map)?;
    process.set("cwd", Func::from(cwd))?;
    process.set("argv", args)?;
    process.set("platform", get_platform())?;
    process.set("arch", get_arch())?;
    process.set("hrtime", hr_time)?;
    process.set("release", release)?;
    process.set("exit", Func::from(exit))?;

    globals.set("process", process)?;

    Ok(())
}
