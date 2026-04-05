// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
};
use rquickjs::{
    class::Trace,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Array, Class, Ctx, JsLifetime, Result, Value,
};

fn isatty(fd: i32) -> bool {
    unsafe { libc::isatty(fd) != 0 }
}

// Returns (columns, rows) for the given fd via TIOCGWINSZ ioctl.
// Returns (0, 0) when the fd is not a TTY or the ioctl fails.
#[cfg(unix)]
fn get_window_size(fd: i32) -> (u16, u16) {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
    if ret == 0 {
        (ws.ws_col, ws.ws_row)
    } else {
        (0, 0)
    }
}

#[cfg(not(unix))]
fn get_window_size(_fd: i32) -> (u16, u16) {
    (0, 0)
}

// Saved original termios per-fd so setRawMode(false) can restore exactly what
// was there before setRawMode(true), rather than applying generic sane defaults.
// Only the stdin fd (0) is expected in practice; a fixed-size array avoids heap.
#[cfg(unix)]
static SAVED_TERMIOS: std::sync::Mutex<[Option<libc::termios>; 3]> =
    std::sync::Mutex::new([None; 3]);

// Enable or disable raw mode on the given fd via tcsetattr.
// In raw mode: no echo, no line buffering, no signal chars, single-byte reads.
// On disable: restores the exact termios captured when raw mode was enabled,
// rather than applying generic sane defaults that may differ from the original.
#[cfg(unix)]
fn set_raw_mode(fd: i32, enable: bool) -> bool {
    // Only stdin/stdout/stderr are tracked in SAVED_TERMIOS. Reject any other
    // fd up front so we never succeed at enabling raw mode without a slot to
    // save the original termios into — which would make disable impossible.
    if fd < 0 || fd > 2 {
        return false;
    }
    let mut termios: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut termios) } != 0 {
        return false;
    }
    if enable {
        // Capture the original termios before mutating it, so disable can
        // restore it exactly. Only save when the slot is empty; a second
        // setRawMode(true) call must not overwrite the saved original with
        // an already-raw termios, which would make setRawMode(false) restore
        // raw mode instead of the true original.
        // Only track fds 0-2 (stdin/stdout/stderr).
        // Recover a poisoned mutex guard rather than silently skipping the save,
        // which would leave us unable to restore the original termios on disable.
        let mut saved = SAVED_TERMIOS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(slot) = saved.get_mut(fd as usize) {
            if slot.is_none() {
                *slot = Some(termios);
            }
        }
        unsafe { libc::cfmakeraw(&mut termios) };
    } else {
        // Restore the exact pre-raw termios if we have one; otherwise fall
        // back to the current termios unchanged (a no-op tcsetattr).
        // Recover a poisoned guard here too — leaving the terminal in raw mode
        // would be a worse outcome than proceeding with whatever state we have.
        let mut saved = SAVED_TERMIOS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(slot) = saved.get_mut(fd as usize) {
            if let Some(original) = *slot {
                // Only clear the snapshot after a successful restore; if
                // tcsetattr fails the original is still in the slot so a
                // retry remains possible.
                if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) } == 0 {
                    *slot = None;
                    return true;
                }
                return false;
            }
        }
    }
    let ret = unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) };
    ret == 0
}

#[cfg(not(unix))]
fn set_raw_mode(_fd: i32, _enable: bool) -> bool {
    false
}

// Synchronous write via libc::write — avoids needing an async context.
// Returns Err on broken pipe, disk full, or other write failures.
#[cfg(unix)]
fn write_fd(fd: i32, bytes: &[u8]) -> std::io::Result<()> {
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let ret = unsafe {
            libc::write(
                fd,
                remaining.as_ptr() as *const libc::c_void,
                remaining.len(),
            )
        };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            // EINTR means the write was interrupted by a signal; retry.
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }
        remaining = &remaining[ret as usize..];
    }
    Ok(())
}

#[cfg(not(unix))]
fn write_fd(_fd: i32, _bytes: &[u8]) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "tty.WriteStream.write is not supported on this platform",
    ))
}

// ── WriteStream ───────────────────────────────────────────────────────────────

#[rquickjs::class(rename = "WriteStream")]
#[derive(Trace)]
pub struct WriteStream {
    #[qjs(skip_trace)]
    fd: i32,
}

unsafe impl<'js> JsLifetime<'js> for WriteStream {
    type Changed<'to> = WriteStream;
}

#[rquickjs::methods]
impl WriteStream {
    #[qjs(constructor)]
    pub fn new(fd: i32) -> Self {
        Self { fd }
    }

    #[qjs(get)]
    pub fn fd(&self) -> i32 {
        self.fd
    }

    #[qjs(get)]
    pub fn columns(&self) -> u32 {
        // Each getter makes its own ioctl call. Use getWindowSize() when you
        // need both dimensions to avoid issuing two ioctls.
        get_window_size(self.fd).0 as u32
    }

    #[qjs(get)]
    pub fn rows(&self) -> u32 {
        // Each getter makes its own ioctl call. Use getWindowSize() when you
        // need both dimensions to avoid issuing two ioctls.
        get_window_size(self.fd).1 as u32
    }

    #[qjs(get, rename = "isTTY")]
    pub fn is_tty(&self) -> bool {
        isatty(self.fd)
    }

    pub fn write(&self, ctx: Ctx<'_>, data: Value<'_>) -> Result<bool> {
        let bytes: Vec<u8> = if let Some(s) = data.as_string() {
            s.to_string()?.into_bytes()
        } else {
            return Ok(false);
        };
        write_fd(self.fd, &bytes).or_throw(&ctx)?;
        Ok(true)
    }

    #[qjs(rename = "setRawMode")]
    pub fn set_raw_mode(&self, enable: bool) -> bool {
        set_raw_mode(self.fd, enable)
    }

    /// Returns [columns, rows] from a single ioctl call.
    /// Prefer this over accessing .columns and .rows separately to avoid two ioctls.
    #[qjs(rename = "getWindowSize")]
    pub fn get_window_size_js<'js>(&self, ctx: Ctx<'js>) -> Result<Array<'js>> {
        let (cols, rows) = get_window_size(self.fd);
        let arr = Array::new(ctx)?;
        arr.set(0, cols as u32)?;
        arr.set(1, rows as u32)?;
        Ok(arr)
    }
}

// ── ReadStream ────────────────────────────────────────────────────────────────

#[rquickjs::class(rename = "ReadStream")]
#[derive(Trace)]
pub struct ReadStream {
    #[qjs(skip_trace)]
    fd: i32,
}

unsafe impl<'js> JsLifetime<'js> for ReadStream {
    type Changed<'to> = ReadStream;
}

#[rquickjs::methods]
impl ReadStream {
    #[qjs(constructor)]
    pub fn new(fd: i32) -> Self {
        Self { fd }
    }

    #[qjs(get)]
    pub fn fd(&self) -> i32 {
        self.fd
    }

    #[qjs(get, rename = "isTTY")]
    pub fn is_tty(&self) -> bool {
        isatty(self.fd)
    }

    #[qjs(rename = "setRawMode")]
    pub fn set_raw_mode(&self, enable: bool) -> bool {
        set_raw_mode(self.fd, enable)
    }
}

// ── Module ────────────────────────────────────────────────────────────────────

pub struct TtyModule;

impl ModuleDef for TtyModule {
    fn declare(declare: &Declarations<'_>) -> Result<()> {
        declare.declare("isatty")?;
        declare.declare("ReadStream")?;
        declare.declare("WriteStream")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("isatty", Func::from(isatty))?;
            Class::<WriteStream>::define(default)?;
            Class::<ReadStream>::define(default)?;
            Ok(())
        })
    }
}

impl From<TtyModule> for ModuleInfo<TtyModule> {
    fn from(val: TtyModule) -> Self {
        ModuleInfo {
            name: "tty",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::TtyModule;
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};
    use std::io::{stderr, stdin, stdout, IsTerminal};

    #[tokio::test]
    async fn test_isatty() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<TtyModule>(ctx.clone(), "tty")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { isatty } from 'tty';

                        export async function test() {
                            return new Array(3).fill(0).map((_, i) => +isatty(i)).join('')
                        }
                    "#,
                )
                .await
                .unwrap();
                let expect = [
                    stdin().is_terminal(),
                    stdout().is_terminal(),
                    stderr().is_terminal(),
                ]
                .map(|i| (i as u8).to_string())
                .join("");
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, expect);
            })
        })
        .await;
    }
}
