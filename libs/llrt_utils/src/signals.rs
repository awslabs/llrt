// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{prelude::Opt, Ctx, Exception, Result, Value};

use crate::result::ResultExt;
use std::io;

#[cfg(unix)]
macro_rules! generate_signal_from_str_fn {
    ($($signal:path),*) => {
        pub fn signal_from_str(signal: &str) -> Option<i32> {
            let signal = ["libc::", signal].concat();
            match signal.as_str() {
                $(stringify!($signal) => Some($signal),)*
                _ => None,
            }
        }

        pub fn signal_str_from_i32(signal: i32) -> Option<&'static str> {
            $(if signal == $signal {
                return Some(&stringify!($signal)[6..]);
            })*
            None
        }
    };
}

#[cfg(unix)]
generate_signal_from_str_fn!(
    libc::SIGHUP,
    libc::SIGINT,
    libc::SIGQUIT,
    libc::SIGILL,
    libc::SIGABRT,
    libc::SIGFPE,
    libc::SIGKILL,
    libc::SIGSEGV,
    libc::SIGPIPE,
    libc::SIGALRM,
    libc::SIGTERM
);

#[cfg(windows)]
static WINDOWS_SIGTERM: i32 = -1;

/// The `SIGTERM` value from wasi-libc's emulated signal ABI.
#[cfg(target_os = "wasi")]
const WASI_SIGTERM: i32 = 15;

pub fn parse_signal(signal: Option<Value<'_>>) -> Result<i32> {
    let Some(val) = signal else {
        #[cfg(unix)]
        return Ok(libc::SIGTERM);
        #[cfg(windows)]
        return Ok(WINDOWS_SIGTERM);
        #[cfg(target_os = "wasi")]
        return Ok(WASI_SIGTERM);
    };

    if let Some(num) = val.as_number() {
        let sig = num as i32;

        #[cfg(any(unix, target_os = "wasi"))]
        return Ok(sig);
        // On Windows: 0 checks existence, anything else kills
        #[cfg(windows)]
        return Ok(if sig == 0 { 0 } else { WINDOWS_SIGTERM });
    }

    if let Some(str_val) = val.as_string() {
        let s = str_val.to_string()?;

        #[cfg(unix)]
        let mapped_sig = signal_from_str(&s);

        #[cfg(windows)]
        let mapped_sig = match s.as_str() {
            "SIGINT" | "SIGTERM" | "SIGKILL" | "SIGQUIT" | "SIGHUP" | "SIGUSR1" => {
                Some(WINDOWS_SIGTERM)
            },
            _ => None,
        };

        // Match the Unix parser's names using wasi-libc's signal numbers, without emulation.
        #[cfg(target_os = "wasi")]
        let mapped_sig = match s.as_str() {
            "SIGHUP" => Some(1),
            "SIGINT" => Some(2),
            "SIGQUIT" => Some(3),
            "SIGILL" => Some(4),
            "SIGABRT" => Some(6),
            "SIGFPE" => Some(8),
            "SIGKILL" => Some(9),
            "SIGSEGV" => Some(11),
            "SIGPIPE" => Some(13),
            "SIGALRM" => Some(14),
            "SIGTERM" => Some(WASI_SIGTERM),
            _ => None,
        };

        return match mapped_sig {
            Some(sig) => Ok(sig),
            None => Err(Exception::throw_type(
                val.ctx(),
                &format!("Unknown signal: {}", s),
            )),
        };
    }

    Err(Exception::throw_type(val.ctx(), "Invalid signal type"))
}

#[cfg(unix)]
pub fn kill_process_raw(pid: u32, signal: i32) -> io::Result<()> {
    // libc::kill returns 0 on success, -1 on error
    // SAFETY: kill is a safe system call as long as the signal value is valid, which is ensured by parse_signal
    if unsafe { libc::kill(pid as i32, signal) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(windows)]
pub fn kill_process_raw(pid: u32, signal: i32) -> io::Result<()> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    // SAFETY: OpenProcess is safe to call with valid parameters, and PROCESS_TERMINATE is a valid access right
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle == std::ptr::null_mut() {
        return Err(io::Error::last_os_error());
    }

    let result = if signal == 0 {
        Ok(())
    } else {
        // SAFETY: TerminateProcess is safe to call with a valid process handle obtained from OpenProcess
        if unsafe { TerminateProcess(handle, 1) } != 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    };

    // SAFETY: CloseHandle is safe to call with a valid handle obtained from OpenProcess
    unsafe { CloseHandle(handle) };
    result
}

/// WASI provides neither process signaling nor signal-zero existence checks.
#[cfg(target_os = "wasi")]
pub fn kill_process_raw(_pid: u32, _signal: i32) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

pub fn kill(ctx: &Ctx<'_>, pid: u32, signal: Opt<Value<'_>>) -> Result<bool> {
    let signal = parse_signal(signal.0)?;

    match kill_process_raw(pid, signal) {
        Ok(()) => Ok(true),
        Err(error) => {
            #[cfg(unix)]
            if signal == 0 && error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(false);
            }

            #[cfg(windows)]
            if signal == 0 {
                return Ok(false);
            }

            #[cfg(target_os = "wasi")]
            if error.kind() == io::ErrorKind::Unsupported {
                let exception = Exception::from_message(ctx.clone(), "kill ENOSYS")?;
                exception.set("code", "ENOSYS")?;
                exception.set("syscall", "kill")?;
                return Err(exception.throw());
            }

            Err(error).or_throw(ctx)
        },
    }
}
