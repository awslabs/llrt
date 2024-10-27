// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use once_cell::sync::Lazy;
use rquickjs::{
    prelude::{Opt, Rest},
    Ctx, Exception, Result, Value,
};
use windows_registry::{Value, LOCAL_MACHINE};
use windows_result::Error;
use windows_version::OsVersion;

static OS_VERSION: Lazy<String> = Lazy::new(|| version().unwrap_or_default());
pub static EOL: &str = "\r\n";
pub static DEV_NULL: &str = "\\.\nul";

pub fn get_priority(who: Opt<u32>) -> i32 {
    0
}

pub fn set_priority(ctx: &Ctx<'_>, args: Rest<Value>) -> Result<()> {
    Err(Exception::throw_syntax(
        &ctx,
        "setPriority is not implemented.",
    ))
}

pub fn get_type() -> &'static str {
    // In theory there are more types linx MinGW but in practice this is good enough
    "Windows_NT"
}

pub fn get_version() -> &'static str {
    &OS_VERSION
}

fn version() -> Result<String> {
    let version = OsVersion::current();

    let registry_key = LOCAL_MACHINE.open("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")?;
    let value = registry_key.get_value("ProductName")?;
    let Value::String(mut value) = value else {
        return Err(Error::empty());
    };

    // Windows 11 shares dwMajorVersion with Windows 10
    // this workaround tries to disambiguate that by checking
    // if the dwBuildNumber is from Windows 11 releases (>= 22000).
    if version.major == 10 && version.build >= 22000 && value.starts_with("Windows 10") {
        value.replace_range(9..10, "1");
    }

    Ok(value)
}
