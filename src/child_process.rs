// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    io::Result as IoResult,
    os::fd::FromRawFd,
    path::Path,
    process::{Command as StdCommand, Stdio},
    sync::{Arc, RwLock},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use rquickjs::{
    class::{Trace, Tracer},
    convert::Coerced,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt, Rest, This},
    Class, Ctx, Error, Exception, IntoJs, Result, Undefined, Value,
};
use tokio::{
    io::AsyncRead,
    process::{Child, Command},
    sync::{
        broadcast::{channel as broadcast_channel, Receiver, Sender},
        oneshot::Receiver as OneshotReceiver,
    },
};

use crate::{
    events::{EmitError, Emitter, EventEmitter, EventList},
    module::export_default,
    stream::{
        readable::{DefaultReadableStream, ReadableStream},
        writable::{DefaultWritableStream, WritableStream},
    },
    utils::{object::ObjectExt, result::ResultExt},
    vm::CtxExtension,
};

macro_rules! generate_signal_from_str_fn {
    ($($signal:path),*) => {
        fn process_signal_from_str(signal: &str) -> Option<i32> {
            let signal = format!("libc::{}", signal);
            match signal.as_str() {
                $(stringify!($signal) => Some($signal),)*
                _ => None,
            }
        }

        fn signal_str_from_i32(signal: i32) -> Option<&'static str> {
            $(if signal == $signal {
                return Some(&stringify!($signal)[6..]);
            })*

             return None;
        }
    };
}

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

#[allow(dead_code)]
#[rquickjs::class]
pub struct ChildProcess<'js> {
    emitter: EventEmitter<'js>,
    args: Option<Vec<String>>,
    command: String,
    kill_signal_tx: Option<Sender<Option<i32>>>,
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
            StdioEnum::Fd(id) => unsafe { Stdio::from_raw_fd(*id) },
        }
    }
}

#[rquickjs::methods]
impl<'js> ChildProcess<'js> {
    #[qjs(get)]
    fn pid(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.pid.into_js(&ctx)
    }

    fn kill(&mut self, _ctx: Ctx<'js>, signal: Opt<Value<'js>>) -> Result<bool> {
        let signal = if let Some(signal) = signal.0 {
            if signal.is_number() {
                Some(signal.as_number().unwrap() as i32)
            } else if signal.is_string() {
                let signal = signal.as_string().unwrap().to_string()?;
                process_signal_from_str(&signal)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(kill_signal_tx) = self.kill_signal_tx.take() {
            return Ok(kill_signal_tx.send(signal).is_ok());
        }

        Ok(false)
    }
}

impl<'js> ChildProcess<'js> {
    fn new(
        ctx: Ctx<'js>,
        command: String,
        args: Option<Vec<String>>,
        child: IoResult<Child>,
    ) -> Result<Class<'js, Self>> {
        let (kill_signal_tx, kill_signal_rx) = broadcast_channel::<Option<i32>>(1);

        let instance = Self {
            emitter: EventEmitter::new(),
            command: command.clone(),
            args,
            pid: None,
            kill_signal_tx: Some(kill_signal_tx),
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

                        wait_for_process(
                            child,
                            &ctx3,
                            kill_signal_rx,
                            &mut exit_code,
                            &mut exit_signal,
                        )
                        .await?;

                        let code = exit_code.unwrap_or_default().into_js(&ctx3)?;
                        let signal;
                        #[cfg(unix)]
                        {
                            if let Some(s) = exit_signal {
                                signal = signal_str_from_i32(s).into_js(&ctx3)?;
                            } else {
                                signal = Undefined.into_value(ctx3.clone());
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            signal = Undefined.into_value(&ctx3);
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

                    spawn_proc.await.emit_error(&ctx2, instance4)?;

                    Ok(())
                })?;
            }
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
            }
        }
        Ok(instance)
    }
}

async fn wait_for_process<'js>(
    mut child: Child,
    ctx: &Ctx<'js>,
    mut kill_signal_rx: Receiver<Option<i32>>,
    exit_code: &mut Option<i32>,
    exit_signal: &mut Option<i32>,
) -> Result<()> {
    loop {
        tokio::select! {
            status = child.wait() => {
                let exit_status = status.or_throw(ctx)?;
                exit_code.replace(exit_status.code().unwrap_or_default());

                #[cfg(unix)]
                {
                    exit_signal.replace(exit_status.signal().unwrap_or_default());
                }
                break;
            }
            Ok(signal) = kill_signal_rx.recv() => {

                #[cfg(unix)]
                {
                    if let Some(signal) = signal {

                        if let Some(pid) = child.id() {

                            if unsafe { libc::killpg(pid as i32, signal) } == 0 {
                                continue;
                            } else {
                               return Err(Exception::throw_message(ctx, &format!("Failed to send signal {} to process {}", signal, pid)));
                            }
                        }
                    }else{
                        child.kill().await.or_throw(ctx)?;
                        break;
                    }

                }
                #[cfg(not(unix))]
                {
                    child.kill().await.or_throw(ctx)?;
                    break;
                }

            },
        }
    }

    Ok(())
}

impl<'js> Emitter<'js> for ChildProcess<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }
}

fn spawn<'js>(
    ctx: Ctx<'js>,
    cmd: String,
    args_and_opts: Rest<Value<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {
    let args_0 = args_and_opts.first();
    let args_1 = args_and_opts.get(1);

    let mut opts = args_0.and_then(|o| o.as_object()).map(|o| o.to_owned());

    if args_1.is_some() {
        opts = args_1.and_then(|o| o.as_object()).map(|o| o.to_owned());
    }

    let mut command_args = if let Some(args_0) = args_0 {
        if args_0.is_array() {
            let args = args_0.clone().into_array().or_throw(&ctx)?;
            let mut args_vec = Vec::with_capacity(args.len());
            for arg in args.iter() {
                let arg: Value = arg?;
                let arg = arg
                    .as_string()
                    .or_throw_msg(&ctx, "argument is not a string")?;
                let arg = arg.to_string()?;
                args_vec.push(arg);
            }
            Some(args_vec)
        } else if args_0.is_object() {
            opts = args_0.as_object().map(|o| o.to_owned());
            None
        } else {
            None
        }
    } else {
        None
    };

    let cmd = if let Some(opts) = &opts {
        if opts
            .get_optional::<&str, bool>("shell")?
            .unwrap_or_default()
        {
            let mut string_args = cmd;
            if let Some(command_args) = command_args {
                string_args.push(' ');
                string_args.push_str(&command_args.join(" "));
            }
            string_args.push(' ');

            command_args = Some(vec![String::from("-c"), string_args]);
            "/bin/sh".to_string()
        } else {
            cmd
        }
    } else {
        cmd
    };

    let mut command = StdCommand::new(cmd.clone());
    if let Some(args) = &command_args {
        command.args(args);
    }

    let mut stdin = StdioEnum::Piped;
    let mut stdout = StdioEnum::Piped;
    let mut stderr = StdioEnum::Piped;

    if let Some(opts) = opts {
        if let Some(gid) = opts.get_optional("gid")? {
            command.gid(gid);
        }
        if let Some(uid) = opts.get_optional("uid")? {
            command.gid(uid);
        }

        if let Some(cwd) = opts.get_optional::<_, String>("cwd")? {
            command.current_dir(&cwd);
        }

        if let Some(env) = opts.get_optional::<_, HashMap<String, Coerced<String>>>("env")? {
            let env: HashMap<String, String> = env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
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
                        StdioEnum::Ignore
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
                        }
                    }
                }
            }
        }
    }

    command.stdin(stdin.to_stdio());
    command.stdout(stdout.to_stdio());
    command.stderr(stderr.to_stdio());

    #[cfg(unix)]
    {
        command.process_group(0);
    }

    //tokio command does not have all std command features stabilized
    let mut command = Command::from(command);

    ChildProcess::new(ctx.clone(), cmd, command_args, command.spawn())
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
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("spawn")?;
        declare.declare("spawnSync")?;
        declare.declare("exec")?;
        declare.declare("execSync")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        Class::<ChildProcess>::register(ctx)?;
        Class::<DefaultWritableStream>::register(ctx)?;
        Class::<DefaultReadableStream>::register(ctx)?;

        ChildProcess::add_event_emitter_prototype(ctx)?;
        DefaultWritableStream::add_writable_stream_prototype(ctx)?;
        DefaultWritableStream::add_event_emitter_prototype(ctx)?;
        DefaultReadableStream::add_readable_stream_prototype(ctx)?;
        DefaultReadableStream::add_event_emitter_prototype(ctx)?;

        export_default(ctx, exports, |default| {
            default.set("spawn", Func::from(spawn))?;
            Ok(())
        })?;

        Ok(())
    }
}
