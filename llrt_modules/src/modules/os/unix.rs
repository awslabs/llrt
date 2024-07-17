use std::ffi::CStr;

use once_cell::sync::Lazy;

static OS_INFO: Lazy<(String, String, String)> = Lazy::new(uname);

pub fn get_type() -> &'static str {
    &OS_INFO.0
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
