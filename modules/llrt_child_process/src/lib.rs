// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

#[cfg(windows)]
use std::os::windows::{
    io::{FromRawHandle, RawHandle},
    process::CommandExt,
};
#[cfg(unix)]
use std::os::{
    fd::FromRawFd,
    unix::process::{CommandExt, ExitStatusExt},
};
use std::{
    collections::HashMap,
    io::Result as IoResult,
    process::{Command as StdCommand, Stdio},
    ptr::NonNull,
    sync::{Arc, RwLock},
    time::Duration,
};

use llrt_context::CtxExtension;
use llrt_events::{EmitError, Emitter, EventEmitter, EventList};
use llrt_stream::{
    readable::{DefaultReadableStream, ReadableStream},
    writable::{DefaultWritableStream, WritableStream},
};
use llrt_utils::{
    module::{export_default, ModuleInfo},
    object::ObjectExt,
    result::ResultExt,
};
use rquickjs::{
    class::{Trace, Tracer},
    convert::Coerced,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt, Rest, This},
    qjs, Array, Class, Ctx, Error, Exception, Function, IntoJs, Object, Persistent, Result, Value,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, ChildStderr, ChildStdout, Command},
    sync::{
        broadcast::{channel as broadcast_channel, Receiver, Sender},
        oneshot::Receiver as OneshotReceiver,
    },
};

#[cfg(unix)]
use llrt_utils::signals::{kill, signal_str_from_i32};

#[cfg(not(unix))]
use llrt_utils::signals::{kill_process_raw, parse_signal};

#[allow(unused_variables)]
fn prepare_shell_args(
    shell: &str,
    windows_verbatim_arguments: &mut bool,
    cmd: String,
    command_args: Option<Vec<String>>,
) -> Vec<String> {
    let mut string_args = cmd;

    #[cfg(windows)]
    let shell_is_cmd = shell.ends_with("cmd") || shell.ends_with("cmd.exe");

    #[cfg(windows)]
    {
        if shell_is_cmd {
            *windows_verbatim_arguments = true;
            string_args.insert(0, '"');
        }
    }

    if let Some(command_args) = command_args {
        //reserve at least arg length +1
        let total_length = command_args.iter().map(|s| s.len() + 1).sum();
        string_args.reserve(total_length);
        string_args.push(' ');

        for arg in command_args.iter() {
            string_args.push_str(arg);
            string_args.push(' ');
        }
    } else {
        string_args.push(' ');
    }

    #[cfg(windows)]
    {
        if shell_is_cmd {
            string_args.push('"');
            return vec![
                String::from("/d"),
                String::from("/s"),
                String::from("/c"),
                string_args,
            ];
        }
    }

    vec!["-c".into(), string_args]
}

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct ChildProcess<'js> {
    emitter: EventEmitter<'js>,
    /// Channel to signal process termination. On Windows, used in kill().
    /// The receiver is used by run_with_accumulation on all platforms.
    #[cfg_attr(unix, allow(dead_code))]
    kill_tx: Option<Sender<()>>,
    pid: Option<u32>,
}

impl<'js> Trace<'js> for ChildProcess<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.emitter.trace(tracer);
    }
}

#[derive(Clone)]
enum StdioEnum {
    Piped,
    Ignore,
    Inherit,
    Fd(i32),
}
impl StdioEnum {
    fn to_stdio(&self) -> Stdio {
        match self {
            StdioEnum::Piped => Stdio::piped(),
            StdioEnum::Ignore => Stdio::null(),
            StdioEnum::Inherit => Stdio::inherit(),
            StdioEnum::Fd(id) => {
                #[cfg(unix)]
                unsafe {
                    Stdio::from_raw_fd(*id)
                }
                #[cfg(windows)]
                unsafe {
                    Stdio::from_raw_handle(*id as RawHandle)
                }
            },
        }
    }
}

/// Configuration for exec/execFile accumulation mode.
/// Uses Persistent<Function> to safely hold the callback across async boundaries.
/// This is the same pattern used by llrt_timers for setTimeout/setInterval.
struct ExecConfig {
    /// The callback saved as Persistent for async safety
    callback: Persistent<Function<'static>>,
    /// Raw context pointer for restoring the callback
    raw_ctx: NonNull<qjs::JSContext>,
    /// Maximum buffer size for stdout + stderr combined (default: 1MB)
    max_buffer: usize,
    /// Timeout in milliseconds (None = no timeout)
    timeout_ms: Option<u64>,
    /// Signal to send on timeout (default: "SIGTERM")
    kill_signal: String,
}

// SAFETY: The callback is only restored on the same JS runtime.
// LLRT is single-threaded, so this is safe.
unsafe impl Send for ExecConfig {}

impl ExecConfig {
    fn new<'js>(
        ctx: &Ctx<'js>,
        callback: Function<'js>,
        max_buffer: usize,
        timeout_ms: Option<u64>,
        kill_signal: String,
    ) -> Self {
        Self {
            callback: Persistent::<Function>::save(ctx, callback),
            raw_ctx: ctx.as_raw(),
            max_buffer,
            timeout_ms,
            kill_signal,
        }
    }
}

/// Result of accumulated execution
struct AccumulatedResult {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: Option<i32>,
    signal: Option<String>,
    timed_out: bool,
    max_buffer_exceeded: bool,
}

#[rquickjs::methods]
impl<'js> ChildProcess<'js> {
    #[qjs(get)]
    fn pid(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.pid.into_js(&ctx)
    }

    #[allow(unused_variables)]
    fn kill(&mut self, ctx: Ctx<'js>, signal: Opt<Value<'js>>) -> Result<bool> {
        if let Some(pid) = self.pid {
            #[cfg(unix)]
            {
                return kill(&ctx, pid, signal);
            }

            #[cfg(windows)]
            {
                let signal = parse_signal(signal.0)?;
                if signal == 0 {
                    return kill_process_raw(pid, 0)
                        .map(|_| true)
                        .or_else(|_| Ok(false));
                }

                if let Some(tx) = self.kill_tx.take() {
                    return Ok(tx.send(()).is_ok());
                }
            }
        }

        Ok(false)
    }
}

impl<'js> ChildProcess<'js> {
    fn new(ctx: Ctx<'js>, command: String, child: IoResult<Child>) -> Result<Class<'js, Self>> {
        let (kill_tx, kill_rx) = broadcast_channel::<()>(1);

        let instance = Self {
            emitter: EventEmitter::new(),
            pid: None,
            kill_tx: Some(kill_tx),
        };

        let stdout_instance = DefaultReadableStream::new(ctx.clone())?;
        let stderr_instance = DefaultReadableStream::new(ctx.clone())?;
        let stdin_instance = DefaultWritableStream::new(ctx.clone())?;

        let instance = Class::instance(ctx.clone(), instance)?;
        let instance2 = instance.clone();
        let instance3 = instance.clone();
        let instance4 = instance.clone();

        instance.set("stderr", stderr_instance.clone())?;
        instance.set("stdout", stdout_instance.clone())?;
        instance.set("stdin", stdin_instance.clone())?;

        match child {
            Ok(mut child) => {
                instance2.borrow_mut().pid = child.id();

                if let Some(child_stdin) = child.stdin.take() {
                    DefaultWritableStream::process(stdin_instance.clone(), &ctx, child_stdin)?;
                };

                let stdout_join_receiver =
                    create_output(&ctx, child.stdout.take(), stdout_instance.clone())?;

                let stderr_join_receiver =
                    create_output(&ctx, child.stderr.take(), stderr_instance.clone())?;

                let ctx2 = ctx.clone();
                let ctx3 = ctx.clone();

                ctx.spawn_exit(async move {
                    let spawn_proc = async move {
                        let mut exit_code = None;
                        let mut exit_signal = None;

                        wait_for_process(child, &ctx3, kill_rx, &mut exit_code, &mut exit_signal)
                            .await?;

                        let code = match exit_code {
                            Some(c) => c.into_js(&ctx3)?,
                            None => rquickjs::Null.into_value(ctx3.clone()),
                        };
                        let signal;
                        #[cfg(unix)]
                        {
                            if let Some(s) = exit_signal {
                                signal = signal_str_from_i32(s).into_js(&ctx3)?;
                            } else {
                                signal = rquickjs::Null.into_value(ctx3.clone());
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            signal = "SIGKILL".into_js(&ctx3)?;
                        }

                        ChildProcess::emit_str(
                            This(instance2.clone()),
                            &ctx3,
                            "exit",
                            vec![code.clone(), signal.clone()],
                            false,
                        )?;

                        if let Some(stderr_join_receiver) = stderr_join_receiver {
                            //ok if sender drops
                            let _ = stderr_join_receiver.await;
                        }
                        if let Some(stdout_join_receiver) = stdout_join_receiver {
                            //ok if sender drops
                            let _ = stdout_join_receiver.await;
                        }

                        WritableStream::end(This(stdin_instance));

                        ReadableStream::drain(stdout_instance, &ctx3)?;
                        ReadableStream::drain(stderr_instance, &ctx3)?;

                        ChildProcess::emit_str(
                            This(instance2.clone()),
                            &ctx3,
                            "close",
                            vec![code, signal],
                            false,
                        )?;

                        Ok::<_, Error>(())
                    };

                    spawn_proc
                        .await
                        .emit_error("child_process", &ctx2, instance4)?;

                    Ok(())
                })?;
            },
            Err(err) => {
                let ctx3 = ctx.clone();

                let err_message = format!("Child process failed to spawn \"{}\". {}", command, err);

                ctx.spawn_exit(async move {
                    if !instance3.borrow().emitter.has_listener_str("error") {
                        return Err(Exception::throw_message(&ctx3, &err_message));
                    }

                    let ex = Exception::from_message(ctx3.clone(), &err_message)?;
                    ChildProcess::emit_str(
                        This(instance3),
                        &ctx3,
                        "error",
                        vec![ex.into()],
                        false,
                    )?;
                    Ok(())
                })?;
            },
        }
        Ok(instance)
    }

    /// Create a ChildProcess with accumulation mode for exec/execFile.
    /// Streams are set to null, and the callback is invoked with (error, stdout, stderr).
    fn new_with_callback(
        ctx: Ctx<'js>,
        command: String,
        child: IoResult<Child>,
        config: ExecConfig,
    ) -> Result<Class<'js, Self>> {
        let (kill_tx, kill_rx) = broadcast_channel::<()>(1);

        let instance = Self {
            emitter: EventEmitter::new(),
            pid: None,
            kill_tx: Some(kill_tx),
        };

        let instance = Class::instance(ctx.clone(), instance)?;

        // Set streams to null for accumulation mode (matches Node.js exec behavior)
        instance.set("stdout", rquickjs::Null)?;
        instance.set("stderr", rquickjs::Null)?;
        instance.set("stdin", rquickjs::Null)?;

        match child {
            Ok(mut child) => {
                instance.borrow_mut().pid = child.id();

                // Take ownership of stdout/stderr for direct reading
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                let instance2 = instance.clone();

                ctx.spawn_exit(async move {
                    let result = Self::run_with_accumulation(
                        child,
                        stdout,
                        stderr,
                        config.max_buffer,
                        config.timeout_ms,
                        config.kill_signal,
                        kill_rx,
                    )
                    .await;

                    // Restore context from raw pointer
                    // SAFETY: ExecConfig has `unsafe impl Send` (see line ~169) which is safe
                    // because LLRT is single-threaded. The raw_ctx pointer was obtained from
                    // ctx.as_raw() before spawn_exit, and we're restoring it on the same
                    // JS runtime thread. This pattern matches llrt_timers.
                    let ctx2 = unsafe { Ctx::from_raw(config.raw_ctx) };

                    // Invoke the callback
                    if let Ok(callback) = config.callback.restore(&ctx2) {
                        Self::invoke_exec_callback(&ctx2, callback, &result)?;
                    }

                    // Emit exit and close events for consistency
                    let code = result.exit_code.unwrap_or_default().into_js(&ctx2)?;
                    let signal: Value = match &result.signal {
                        Some(s) => s.as_str().into_js(&ctx2)?,
                        None => rquickjs::Null.into_value(ctx2.clone()),
                    };

                    ChildProcess::emit_str(
                        This(instance2.clone()),
                        &ctx2,
                        "exit",
                        vec![code.clone(), signal.clone()],
                        false,
                    )?;

                    ChildProcess::emit_str(
                        This(instance2),
                        &ctx2,
                        "close",
                        vec![code, signal],
                        false,
                    )?;

                    Ok(())
                })?;
            },
            Err(err) => {
                let err_message = format!("Child process failed to spawn \"{}\". {}", command, err);
                let instance3 = instance.clone();

                ctx.spawn_exit(async move {
                    // SAFETY: See comment at line ~417 for full explanation.
                    // Same single-threaded runtime guarantees apply here.
                    let ctx2 = unsafe { Ctx::from_raw(config.raw_ctx) };

                    // Invoke callback with error
                    if let Ok(callback) = config.callback.restore(&ctx2) {
                        let error = Exception::from_message(ctx2.clone(), &err_message)?;
                        let error_obj = error.into_object();
                        error_obj.set("code", "ENOENT")?;
                        callback.call::<_, ()>((error_obj.into_value(), "", ""))?;
                    }

                    // Emit error event if there are listeners
                    if instance3.borrow().emitter.has_listener_str("error") {
                        let ex = Exception::from_message(ctx2.clone(), &err_message)?;
                        ChildProcess::emit_str(
                            This(instance3),
                            &ctx2,
                            "error",
                            vec![ex.into()],
                            false,
                        )?;
                    }

                    Ok(())
                })?;
            },
        }

        Ok(instance)
    }

    /// Run the child process with output accumulation.
    /// Preserves partial data on timeout by killing the process first,
    /// then letting the stream readers complete naturally.
    async fn run_with_accumulation(
        mut child: Child,
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        max_buffer: usize,
        timeout_ms: Option<u64>,
        kill_signal: String,
        mut kill_rx: Receiver<()>,
    ) -> AccumulatedResult {
        // Start reading stdout/stderr as concurrent futures.
        // These will complete when pipes close (process exits or is killed).
        let stdout_future = Self::read_stream(stdout);
        let stderr_future = Self::read_stream(stderr);
        tokio::pin!(stdout_future);
        tokio::pin!(stderr_future);

        let mut timed_out = false;
        let mut exit_code = None;
        let mut exit_signal: Option<String> = None;

        // Set up deadline if timeout is configured
        let deadline = timeout_ms.map(|ms| tokio::time::Instant::now() + Duration::from_millis(ms));

        // Wait for process to exit, handling timeout and kill signals
        loop {
            tokio::select! {
                biased;

                // Handle timeout - kill process with configured signal
                _ = async {
                    if let Some(d) = deadline {
                        tokio::time::sleep_until(d).await
                    } else {
                        std::future::pending::<()>().await
                    }
                }, if deadline.is_some() && !timed_out => {
                    timed_out = true;
                    exit_signal = Some(kill_signal.clone());
                    Self::kill_with_signal(&mut child, &kill_signal).await;
                }

                // Handle kill from JS (.kill() method)
                Ok(()) = kill_rx.recv() => {
                    let _ = child.kill().await;
                }

                // Process exited
                status = child.wait() => {
                    if let Ok(status) = status {
                        exit_code = status.code();
                        #[cfg(unix)]
                        {
                            if let Some(sig) = status.signal() {
                                exit_signal = signal_str_from_i32(sig).map(|s| s.to_string());
                            }
                        }
                    }
                    break;
                }
            }
        }

        // Now collect the data - streams will EOF after process dies.
        // This preserves partial data even on timeout.
        let stdout_data = stdout_future.await;
        let stderr_data = stderr_future.await;

        // Check combined size
        let total = stdout_data.len() + stderr_data.len();
        let max_buffer_exceeded = total > max_buffer;

        AccumulatedResult {
            stdout: stdout_data,
            stderr: stderr_data,
            exit_code,
            signal: exit_signal,
            timed_out,
            max_buffer_exceeded,
        }
    }

    /// Kill process with the specified signal
    async fn kill_with_signal(child: &mut Child, signal: &str) {
        #[cfg(unix)]
        {
            if let Some(pid) = child.id() {
                // Try to send the specified signal
                let sig_num = match signal {
                    "SIGTERM" | "15" => libc::SIGTERM,
                    "SIGKILL" | "9" => libc::SIGKILL,
                    "SIGINT" | "2" => libc::SIGINT,
                    "SIGHUP" | "1" => libc::SIGHUP,
                    "SIGQUIT" | "3" => libc::SIGQUIT,
                    _ => libc::SIGTERM, // Default to SIGTERM
                };
                unsafe {
                    libc::kill(pid as i32, sig_num);
                }
            }
        }
        #[cfg(not(unix))]
        {
            // Windows doesn't have signals - just kill
            let _ = signal;
            let _ = child.kill().await;
        }
    }

    /// Read from a stream until EOF
    async fn read_stream<R: AsyncRead + Unpin>(reader: Option<R>) -> Vec<u8> {
        let Some(mut reader) = reader else {
            return Vec::new();
        };

        let mut buffer = Vec::new();
        let mut chunk = [0u8; 8192];

        loop {
            match reader.read(&mut chunk).await {
                Ok(0) => break, // EOF
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }

        buffer
    }

    /// Invoke the exec callback with (error, stdout, stderr)
    fn invoke_exec_callback(
        ctx: &Ctx<'js>,
        callback: Function<'js>,
        result: &AccumulatedResult,
    ) -> Result<()> {
        // Build error object if needed
        let error: Value<'js> = if result.max_buffer_exceeded {
            let err = Exception::from_message(ctx.clone(), "stdout maxBuffer length exceeded")?;
            let obj = err.into_object();
            obj.set("code", "ERR_CHILD_PROCESS_STDIO_MAXBUFFER")?;
            obj.set("killed", true)?;
            obj.into_value()
        } else if result.timed_out {
            let err = Exception::from_message(ctx.clone(), "Process timed out")?;
            let obj = err.into_object();
            obj.set("code", "ETIMEDOUT")?;
            obj.set("killed", true)?;
            if let Some(sig) = &result.signal {
                obj.set("signal", sig.as_str())?;
            }
            obj.into_value()
        } else if result.exit_code.map(|c| c != 0).unwrap_or(false) {
            let err = Exception::from_message(ctx.clone(), "Command failed")?;
            let obj = err.into_object();
            obj.set("code", result.exit_code)?;
            if let Some(sig) = &result.signal {
                obj.set("signal", sig.as_str())?;
            }
            obj.into_value()
        } else {
            rquickjs::Null.into_value(ctx.clone())
        };

        // Convert bytes to strings (UTF-8 lossy)
        let stdout_str = String::from_utf8_lossy(&result.stdout).into_owned();
        let stderr_str = String::from_utf8_lossy(&result.stderr).into_owned();

        // Call: callback(error, stdout, stderr)
        callback.call::<_, ()>((error, stdout_str, stderr_str))?;

        Ok(())
    }
}

async fn wait_for_process(
    mut child: Child,
    ctx: &Ctx<'_>,
    mut kill_rx: Receiver<()>,
    exit_code: &mut Option<i32>,
    _exit_signal: &mut Option<i32>,
) -> Result<()> {
    #[cfg(not(unix))]
    let mut was_killed = false;
    loop {
        tokio::select! {
            status = child.wait() => {
                let exit_status = status.or_throw(ctx)?;

                #[cfg(unix)]
                {
                    if let Some(sig) = exit_status.signal() {
                        _exit_signal.replace(sig);
                        // code is null when terminated by signal
                    } else {
                        exit_code.replace(exit_status.code().unwrap_or_default());
                    }
                }
                #[cfg(not(unix))]
                {
                    if !was_killed {
                        exit_code.replace(exit_status.code().unwrap_or_default());
                    }
                }
                break;
            }

            Ok(()) = kill_rx.recv() => {
                #[cfg(not(unix))]
                {
                    was_killed = true;
                }
                child.kill().await.or_throw(ctx)?;
            }
        }
    }

    Ok(())
}

impl<'js> Emitter<'js> for ChildProcess<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }
}

/// Core function that spawns a child process.
/// This is the shared implementation used by spawn, exec, and execFile.
///
/// When `callback` is Some, uses accumulation mode (for exec/execFile with callback).
/// When `callback` is None, uses streaming mode (for spawn or exec/execFile without callback).
fn spawn_child_process<'js>(
    ctx: Ctx<'js>,
    cmd: String,
    command_args: Option<Vec<String>>,
    opts: Option<Object<'js>>,
    callback: Option<Function<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {
    let mut windows_verbatim_arguments = if let Some(opts) = &opts {
        opts.get_optional::<&str, bool>("windowsVerbatimArguments")?
            .unwrap_or_default()
    } else {
        false
    };

    // Handle shell option
    let (cmd, command_args) = if let Some(opts) = &opts {
        if opts
            .get_optional::<&str, bool>("shell")?
            .unwrap_or_default()
        {
            #[cfg(windows)]
            let shell = "cmd.exe".to_string();
            #[cfg(not(windows))]
            let shell = "/bin/sh".to_string();
            let shell_args =
                prepare_shell_args(&shell, &mut windows_verbatim_arguments, cmd, command_args);
            (shell, Some(shell_args))
        } else if let Some(shell) = opts.get_optional::<&str, String>("shell")? {
            let shell_args =
                prepare_shell_args(&shell, &mut windows_verbatim_arguments, cmd, command_args);
            (shell, Some(shell_args))
        } else {
            (cmd, command_args)
        }
    } else {
        (cmd, command_args)
    };

    let mut command = StdCommand::new(cmd.clone());
    if let Some(args) = &command_args {
        #[cfg(windows)]
        if windows_verbatim_arguments {
            command.raw_arg(args.join(" "));
        } else {
            command.args(args);
        }
        #[cfg(not(windows))]
        command.args(args);
    }

    let mut stdin = StdioEnum::Piped;
    let mut stdout = StdioEnum::Piped;
    let mut stderr = StdioEnum::Piped;
    let mut detached = false;

    if let Some(opts) = &opts {
        #[cfg(unix)]
        {
            if let Some(gid) = opts.get_optional("gid")? {
                command.gid(gid);
            }

            if let Some(uid) = opts.get_optional("uid")? {
                command.uid(uid);
            }
        }

        detached = opts.get_optional("detached")?.unwrap_or(false);

        if let Some(cwd) = opts.get_optional::<_, String>("cwd")? {
            command.current_dir(&cwd);
        }

        if let Some(env) = opts.get_optional::<_, HashMap<String, Coerced<String>>>("env")? {
            let env: HashMap<String, String> = env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            command.env_clear();
            command.envs(env);
        }

        if let Some(stdio) = opts.get_optional::<_, Value<'js>>("stdio")? {
            if let Some(stdio_str) = stdio.as_string() {
                let stdio = str_to_stdio(&ctx, &stdio_str.to_string()?)?;
                stdin = stdio.clone();
                stdout = stdio.clone();
                stderr = stdio;
            } else if let Some(stdio) = stdio.as_array() {
                for (i, item) in stdio.iter::<Value>().enumerate() {
                    let item = item?;
                    let stdio = if item.is_undefined() || item.is_null() {
                        StdioEnum::Piped
                    } else if let Some(std_io_str) = item.as_string() {
                        str_to_stdio(&ctx, &std_io_str.to_string()?)?
                    } else if let Some(fd) = item.as_number() {
                        StdioEnum::Fd(fd as i32)
                    } else {
                        StdioEnum::Piped
                    };
                    match i {
                        0 => stdin = stdio,
                        1 => stdout = stdio,
                        2 => stderr = stdio,
                        _ => {
                            break;
                        },
                    }
                }
            }
        }
    }

    // For callback mode, force piped stdout/stderr for accumulation
    if callback.is_some() {
        stdout = StdioEnum::Piped;
        stderr = StdioEnum::Piped;
    }

    command.stdin(stdin.to_stdio());
    command.stdout(stdout.to_stdio());
    command.stderr(stderr.to_stdio());

    if detached {
        #[cfg(unix)]
        {
            command.process_group(0);
        }
        #[cfg(windows)]
        {
            // DETACHED_PROCESS = 0x00000008
            command.creation_flags(0x00000008);
        }
    }

    // tokio command does not have all std command features stabilized
    let mut command = Command::from(command);
    let spawn_result = command.spawn();

    match callback {
        Some(cb) => {
            // With callback: use accumulation mode
            let (max_buffer, timeout_ms, kill_signal) = extract_exec_options(opts.as_ref());
            let config = ExecConfig::new(&ctx, cb, max_buffer, timeout_ms, kill_signal);
            ChildProcess::new_with_callback(ctx, cmd, spawn_result, config)
        },
        None => {
            // Without callback: use streaming mode
            ChildProcess::new(ctx, cmd, spawn_result)
        },
    }
}

/// spawn(command[, args][, options])
/// Spawns a new process using the given command.
fn spawn<'js>(
    ctx: Ctx<'js>,
    cmd: String,
    args_and_opts: Rest<Value<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {
    let (command_args, opts) = parse_spawn_args(&ctx, &args_and_opts)?;
    spawn_child_process(ctx, cmd, command_args, opts, None)
}

/// Parse spawn's argument combinations: [args], [options]
fn parse_spawn_args<'js>(
    ctx: &Ctx<'js>,
    args: &Rest<Value<'js>>,
) -> Result<(Option<Vec<String>>, Option<Object<'js>>)> {
    let args_0 = args.first();
    let args_1 = args.get(1);

    let mut opts = None;
    let mut command_args = None;

    if let Some(arg) = args_0 {
        if let Some(arr) = arg.as_array() {
            command_args = Some(array_to_vec_string(ctx, arr)?);
        } else if let Some(o) = arg.as_object() {
            opts = Some(o.clone());
        }
    }

    if let Some(arg) = args_1 {
        if let Some(o) = arg.as_object() {
            opts = Some(o.clone());
        }
    }

    Ok((command_args, opts))
}

/// execFile(file[, args][, options][, callback])
/// Executes a file directly without spawning a shell (unless shell option is set).
/// This is a higher-level API that builds on spawn_child_process.
fn exec_file<'js>(
    ctx: Ctx<'js>,
    file: String,
    rest: Rest<Value<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {
    let (command_args, opts, callback) = parse_exec_file_args(&ctx, &rest)?;
    spawn_child_process(ctx, file, command_args, opts, callback)
}

/// exec(command[, options][, callback])
/// Executes a command in a shell.
/// This is the highest-level API: exec → execFile (with shell:true) → spawn_child_process.
fn exec<'js>(
    ctx: Ctx<'js>,
    command: String,
    rest: Rest<Value<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {
    let (opts, callback) = parse_exec_args(&rest);

    // exec() always uses shell - ensure shell:true is set in options
    let opts = ensure_shell_option(&ctx, opts)?;

    // Delegate to spawn_child_process with shell:true
    // exec passes the command as a single string, no args
    spawn_child_process(ctx, command, None, Some(opts), callback)
}

/// Ensure shell option is set to true in options object.
/// Creates a new options object if none exists.
fn ensure_shell_option<'js>(ctx: &Ctx<'js>, opts: Option<Object<'js>>) -> Result<Object<'js>> {
    match opts {
        Some(opts) => {
            // Only set shell if not already set
            if opts.get::<_, Value>("shell")?.is_undefined() {
                opts.set("shell", true)?;
            }
            Ok(opts)
        },
        None => {
            let opts = Object::new(ctx.clone())?;
            opts.set("shell", true)?;
            Ok(opts)
        },
    }
}

/// Parse exec's argument combinations: [options], [callback]
fn parse_exec_args<'js>(args: &Rest<Value<'js>>) -> (Option<Object<'js>>, Option<Function<'js>>) {
    let args_0 = args.first();
    let args_1 = args.get(1);

    let mut opts = None;
    let mut callback = None;

    if let Some(arg) = args_0 {
        if let Some(f) = arg.as_function() {
            callback = Some(f.clone());
        } else if let Some(o) = arg.as_object().filter(|o| !o.is_null()) {
            opts = Some(o.clone());
        }
    }

    if let Some(arg) = args_1 {
        if let Some(f) = arg.as_function() {
            callback = Some(f.clone());
        }
    }

    (opts, callback)
}

/// Extract exec/execFile options: maxBuffer, timeout, killSignal
fn extract_exec_options(opts: Option<&Object<'_>>) -> (usize, Option<u64>, String) {
    let max_buffer = opts
        .and_then(|o| o.get_optional("maxBuffer").ok().flatten())
        .unwrap_or(1024 * 1024); // 1MB default

    let timeout_ms = opts.and_then(|o| o.get_optional("timeout").ok().flatten());

    let kill_signal = opts
        .and_then(|o| o.get_optional::<_, String>("killSignal").ok().flatten())
        .unwrap_or_else(|| "SIGTERM".to_string());

    (max_buffer, timeout_ms, kill_signal)
}

/// Parse execFile's flexible argument combinations: [args], [options], [callback]
#[allow(clippy::type_complexity)]
fn parse_exec_file_args<'js>(
    ctx: &Ctx<'js>,
    args: &Rest<Value<'js>>,
) -> Result<(
    Option<Vec<String>>,
    Option<Object<'js>>,
    Option<Function<'js>>,
)> {
    let args_0 = args.first();
    let args_1 = args.get(1);
    let args_2 = args.get(2);

    let mut command_args = None;
    let mut opts = None;
    let mut callback = None;

    // args_0: Array | Object | Function
    if let Some(arg) = args_0 {
        if let Some(arr) = arg.as_array() {
            command_args = Some(array_to_vec_string(ctx, arr)?);
        } else if let Some(f) = arg.as_function() {
            callback = Some(f.clone());
        } else if let Some(o) = arg.as_object().filter(|o| !o.is_null()) {
            opts = Some(o.clone());
        }
    }

    // args_1: Object | Function
    if let Some(arg) = args_1 {
        if let Some(f) = arg.as_function() {
            callback = Some(f.clone());
        } else if let Some(o) = arg.as_object().filter(|o| !o.is_null()) {
            opts = Some(o.clone());
        }
    }

    // args_2: Function
    if let Some(arg) = args_2 {
        if let Some(f) = arg.as_function() {
            callback = Some(f.clone());
        }
    }

    Ok((command_args, opts, callback))
}

/// Convert a JS array to a Vec<String>
fn array_to_vec_string<'js>(ctx: &Ctx<'js>, arr: &Array<'js>) -> Result<Vec<String>> {
    let mut result = Vec::with_capacity(arr.len());
    for item in arr.iter::<Value>() {
        let item = item?;
        let s = item
            .as_string()
            .or_throw_msg(ctx, "argument must be a string")?
            .to_string()?;
        result.push(s);
    }
    Ok(result)
}

fn str_to_stdio(ctx: &Ctx<'_>, input: &str) -> Result<StdioEnum> {
    match input {
        "pipe" => Ok(StdioEnum::Piped),
        "ignore" => Ok(StdioEnum::Ignore),
        "inherit" => Ok(StdioEnum::Inherit),
        _ => Err(Exception::throw_type(
            ctx,
            &format!(
                "Invalid stdio \"{}\". Expected one of: pipe, ignore, inherit",
                input
            ),
        )),
    }
}

fn create_output<'js, T>(
    ctx: &Ctx<'js>,
    output: Option<T>,
    native_readable_stream: Class<'js, DefaultReadableStream<'js>>,
) -> Result<Option<OneshotReceiver<bool>>>
where
    T: AsyncRead + Unpin + Send + 'static,
{
    if let Some(output) = output {
        let receiver = DefaultReadableStream::process(native_readable_stream, ctx, output)?;
        return Ok(Some(receiver));
    }

    Ok(None)
}

pub struct ChildProcessModule;

impl ModuleDef for ChildProcessModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("spawn")?;
        declare.declare("exec")?;
        declare.declare("execFile")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        ChildProcess::add_event_emitter_prototype(ctx)?;
        DefaultWritableStream::add_writable_stream_prototype(ctx)?;
        DefaultWritableStream::add_event_emitter_prototype(ctx)?;
        DefaultReadableStream::add_readable_stream_prototype(ctx)?;
        DefaultReadableStream::add_event_emitter_prototype(ctx)?;

        export_default(ctx, exports, |default| {
            default.set("spawn", Func::from(spawn))?;
            default.set("exec", Func::from(exec))?;
            default.set("execFile", Func::from(exec_file))?;
            Ok(())
        })?;

        Ok(())
    }
}

impl From<ChildProcessModule> for ModuleInfo<ChildProcessModule> {
    fn from(val: ChildProcessModule) -> Self {
        ModuleInfo {
            name: "child_process",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llrt_buffer as buffer;
    use llrt_test::{test_async_with, ModuleEvaluator};
    use rquickjs::CatchResultExt;

    #[tokio::test]
    async fn test_spawn() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                   import {spawn} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    spawn("echo", ["hello"]).stdout.on("data", (data) => {
                        resolve(data.toString().trim())
                    });

                    export default await deferred;

                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "hello");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_spawn_shell() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {spawn} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    spawn("echo", ["hello"], {
                        shell: true
                    }).stdout.on("data", (data) => {
                        resolve(data.toString().trim())
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "hello");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_exec_file() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {execFile} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    execFile("echo", ["hello"], (error, stdout, stderr) => {
                        resolve(stdout.trim());
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "hello");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_exec_file_no_args() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {execFile} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    execFile("pwd", (error, stdout, stderr) => {
                        resolve(stdout.trim().length > 0 ? "has_output" : "no_output");
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "has_output");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_exec() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {exec} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    exec("echo hello", (error, stdout, stderr) => {
                        resolve(stdout.trim());
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "hello");
            })
        })
        .await;
    }

    #[tokio::test]
    #[cfg(unix)] // Uses Unix-specific paths and commands
    async fn test_exec_with_options() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {exec} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    exec("pwd", { cwd: "/tmp" }, (error, stdout, stderr) => {
                        resolve(stdout.trim());
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                // On macOS, /tmp is a symlink to /private/tmp
                assert!(message == "/tmp" || message == "/private/tmp");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_exec_file_error() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let result: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {execFile} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    execFile("false", (error, stdout, stderr) => {
                        if (error) {
                            resolve("error_received");
                        } else {
                            resolve("no_error");
                        }
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(result, "error_received");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_exec_file_with_shell() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {execFile} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    // Use shell to run a command with shell features (globbing, pipes)
                    execFile("echo", ["hello", "world"], { shell: true }, (error, stdout, stderr) => {
                        resolve(stdout.trim());
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "hello world");
            })
        })
        .await;
    }

    #[tokio::test]
    #[cfg(unix)] // Uses Unix shell variable syntax ($VAR)
    async fn test_exec_with_env() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {exec} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    exec("echo $MY_TEST_VAR", { env: { MY_TEST_VAR: "custom_value" } }, (error, stdout, stderr) => {
                        resolve(stdout.trim());
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "custom_value");
            })
        })
        .await;
    }

    #[tokio::test]
    #[cfg(unix)] // Uses sh and Unix shell variable syntax
    async fn test_exec_file_with_env() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {execFile} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    // Use shell: true to enable variable expansion
                    execFile("sh", ["-c", "echo $MY_VAR"], { env: { MY_VAR: "env_test" } }, (error, stdout, stderr) => {
                        resolve(stdout.trim());
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(message, "env_test");
            })
        })
        .await;
    }

    #[tokio::test]
    #[cfg(unix)] // Uses sleep and && shell operator
    async fn test_exec_timeout_preserves_partial_data() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let result: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {exec} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    // Process that outputs data then sleeps - we should get partial data on timeout
                    exec("echo partial_output && sleep 10", { timeout: 100 }, (error, stdout, stderr) => {
                        const hasError = error !== null;
                        const hasPartialData = stdout.includes("partial_output");
                        resolve(hasError && hasPartialData ? "timeout_with_data" : "unexpected:" + stdout);
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(result, "timeout_with_data");
            })
        })
        .await;
    }

    #[tokio::test]
    #[cfg(unix)] // Uses sh, yes, and head commands
    async fn test_exec_file_max_buffer() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let result: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {execFile} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    // Generate output larger than maxBuffer
                    execFile("sh", ["-c", "yes | head -c 200"], { maxBuffer: 100 }, (error, stdout, stderr) => {
                        if (error && error.code === "ERR_CHILD_PROCESS_STDIO_MAXBUFFER") {
                            resolve("maxbuffer_error");
                        } else if (error) {
                            resolve("other_error:" + error.code);
                        } else {
                            resolve("no_error");
                        }
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(result, "maxbuffer_error");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_exec_error_nonexistent_command() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "node:child_process")
                    .await
                    .unwrap();

                let result: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {exec} from "node:child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    exec("this_command_does_not_exist_xyz123", (error, stdout, stderr) => {
                        if (error) {
                            resolve("error_received");
                        } else {
                            resolve("no_error");
                        }
                    });

                    export default await deferred;
                "#,
                )
                .await
                .catch(&ctx)
                .unwrap()
                .get("default")
                .unwrap();

                assert_eq!(result, "error_received");
            })
        })
        .await;
    }
}
