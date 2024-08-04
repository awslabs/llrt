use std::env;

pub fn get_platform() -> &'static str {
    let platform = env::consts::OS;
    match platform {
        "macos" => "darwin",
        "windows" => "win32",
        _ => platform,
    }
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
