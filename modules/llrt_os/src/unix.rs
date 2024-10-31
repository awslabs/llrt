// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    ffi::CStr,
    sync::{Arc, Mutex},
};

use libc::{getpriority, setpriority, PRIO_PROCESS};
use once_cell::sync::Lazy;
use rquickjs::{
    prelude::{Opt, Rest},
    Ctx, Exception, IntoJs, Null, Object, Result, Value,
};
use users::{os::unix::UserExt, Groups, Users, UsersCache};

use crate::get_home_dir;

static USER_CACHE: Lazy<Arc<Mutex<UsersCache>>> =
    Lazy::new(|| Arc::new(Mutex::new(UsersCache::new())));

static OS_INFO: Lazy<(String, String, String)> = Lazy::new(uname);
pub static EOL: &str = "\n";
pub static DEV_NULL: &str = "/dev/null";

pub fn get_priority(who: Opt<u32>) -> i32 {
    let who = who.0.unwrap_or(0);

    unsafe { getpriority(PRIO_PROCESS, who) }
}

pub fn set_priority(ctx: Ctx<'_>, args: Rest<Value>) -> Result<()> {
    let mut args_iter = args.0.into_iter().rev();
    let prio: i32 = args_iter
        .next()
        .and_then(|v| v.as_number())
        .ok_or_else(|| {
            Exception::throw_type(&ctx, "The `priority` argument must be of type number.")
        })? as i32;
    let who: u32 = args_iter.next().and_then(|v| v.as_number()).unwrap_or(0f64) as u32;

    if !(-20..=19).contains(&prio) {
        return Err(Exception::throw_range(
            &ctx,
            "The value of `priority` is out of range. It must be >= -20 && <= 19.",
        ));
    }

    unsafe {
        setpriority(PRIO_PROCESS, who, prio);
    }
    Ok(())
}

pub fn get_type() -> &'static str {
    &OS_INFO.0
}

pub fn get_user_info<'js>(ctx: Ctx<'js>, _options: Opt<Value>) -> Result<Object<'js>> {
    let cache = USER_CACHE.lock().unwrap();

    let obj = Object::new(ctx.clone())?;

    let uid = cache.get_current_uid();
    obj.set("uid", uid)?;
    obj.set("gid", cache.get_current_gid())?;

    let (username, shell) = if let Some(user) = cache.get_user_by_uid(uid) {
        (
            user.name().to_str().into_js(&ctx)?,
            user.shell().to_str().into_js(&ctx)?,
        )
    } else {
        (Null.into_js(&ctx)?, Null.into_js(&ctx)?)
    };
    obj.set("username", username)?;
    obj.set("homedir", get_home_dir(ctx.clone()))?;
    obj.set("shell", shell)?;
    Ok(obj)
}

pub fn get_release() -> &'static str {
    &OS_INFO.1
}

pub fn get_version() -> &'static str {
    &OS_INFO.2
}

fn uname() -> (String, String, String) {
    let mut info = std::mem::MaybeUninit::uninit();
    // SAFETY: `info` is a valid pointer to a `libc::utsname` struct.
    let res = unsafe { libc::uname(info.as_mut_ptr()) };
    if res != 0 {
        return (String::new(), String::new(), String::new());
    }
    // SAFETY: `uname` returns 0 on success and info is initialized.
    let info = unsafe { info.assume_init() };
    (
        // SAFETY: `info.sysname` is a valid NUL-terminated pointer.
        unsafe {
            CStr::from_ptr(info.sysname.as_ptr())
                .to_string_lossy()
                .into_owned()
        },
        // SAFETY: `info.release` is a valid NUL-terminated pointer.
        unsafe {
            CStr::from_ptr(info.release.as_ptr())
                .to_string_lossy()
                .into_owned()
        },
        // SAFETY: `info.version` is a valid NUL-terminated pointer.
        unsafe {
            CStr::from_ptr(info.version.as_ptr())
                .to_string_lossy()
                .into_owned()
        },
    )
}
