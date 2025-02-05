// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use once_cell::sync::Lazy;
use rquickjs::{
    prelude::{Opt, Rest},
    Ctx, Exception, IntoJs, Null, Object,
};

use windows_registry::LOCAL_MACHINE;
use windows_result::Result;
use windows_version::OsVersion;

use crate::get_home_dir;

static OS_RELEASE: Lazy<String> = Lazy::new(release);
static OS_VERSION: Lazy<String> = Lazy::new(|| version().unwrap_or_default());
pub static EOL: &str = "\r\n";
pub static DEV_NULL: &str = "\\.\nul";

pub fn get_priority(_who: Opt<u32>) -> i32 {
    0
}

pub fn set_priority(ctx: Ctx<'_>, _args: Rest<rquickjs::Value>) -> rquickjs::Result<()> {
    Err(Exception::throw_syntax(
        &ctx,
        "setPriority is not implemented.",
    ))
}

pub fn get_type() -> &'static str {
    // In theory there are more types linx MinGW but in practice this is good enough
    "Windows_NT"
}

pub fn get_release() -> &'static str {
    &OS_RELEASE
}

pub fn get_version() -> &'static str {
    &OS_VERSION
}

fn release() -> String {
    let version = OsVersion::current();
    format!("{}.{}.{}", version.major, version.minor, version.build)
}

pub fn get_user_info<'js>(
    ctx: Ctx<'js>,
    _options: Opt<rquickjs::Value>,
) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx.clone())?;

    obj.set("uid", -1)?;
    obj.set("gid", -1)?;
    obj.set("username", whoami::username())?;
    obj.set("homedir", get_home_dir(ctx.clone()))?;
    obj.set("shell", Null.into_js(&ctx)?)?;
    Ok(obj)
}

fn version() -> Result<String> {
    let version = OsVersion::current();

    let registry_key = LOCAL_MACHINE
        .open("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .map_err(std::io::Error::from)?;
    let mut value = registry_key
        .get_string("ProductName")
        .map_err(std::io::Error::from)?;
    // Windows 11 shares dwMajorVersion with Windows 10
    // this workaround tries to disambiguate that by checking
    // if the dwBuildNumber is from Windows 11 releases (>= 22000).
    if version.major == 10 && version.build >= 22000 && value.starts_with("Windows 10") {
        value.replace_range(9..10, "1");
    }

    Ok(value)
}
