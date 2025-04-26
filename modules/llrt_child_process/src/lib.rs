// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(windows)]
use std::os::windows::{
    io::{FromRawHandle, RawHandle},
    process::CommandExt,
};
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

use std::{
    io::Result as IoResult,
    process::Command as StdCommand,
    sync::{Arc, Mutex, RwLock},
};

use llrt_context::CtxExtension;
use llrt_events::{EmitError, Emitter, EventEmitter, EventList};
use llrt_stream::{
    readable::{DefaultReadableStream, ReadableStream},
    writable::{DefaultWritableStream, WritableStream},
};
use llrt_utils::module::{export_default, ModuleInfo};

use rquickjs::{
    class::{Trace, Tracer},
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Opt, Rest, This},
    Class, Ctx, Error, Exception, Function, IntoJs, Null, Result, Value,
};
use tokio::{
    process::{Child, Command},
    sync::broadcast::{channel as broadcast_channel, Sender}
};

pub mod helpers;
pub mod process;

use self::helpers::{get_env, create_error_object, create_output, get_callback_fn, get_cmd, wait_for_process, get_command_args, get_cwd, get_gid, get_output, get_signal, get_stdio, get_uid, get_windows_verbatim_arguments, process_signal_from_str, set_command_args, StdioEnum};
use self::process::ChildProcess;

fn spawn<'js>(
    ctx: Ctx<'js>,
    cmd: String,
    args_and_opts: Rest<Value<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {
    let args_0 = args_and_opts.first();
    let args_1 = args_and_opts.get(1);
    let mut opts = None;

    if args_1.is_some() {
        opts = args_1.and_then(|o| o.as_object()).map(|o| o.to_owned());
    }

    let mut command_args = match get_command_args(&ctx, args_0, &mut opts) {
        Ok(Some(args)) => Some(args),
        Ok(None) => None,
        Err(err) => return Err(err),
    };

    let mut windows_verbatim_arguments: bool = get_windows_verbatim_arguments(opts.as_ref())?;

    let cmd = get_cmd(
        opts.as_ref(),
        &mut command_args,
        &mut windows_verbatim_arguments,
        cmd,
    )?;

    let mut command = StdCommand::new(cmd.clone());
    set_command_args(
        &mut command,
        command_args.as_ref(),
        windows_verbatim_arguments,
    );

    let mut stdin = StdioEnum::Piped;
    let mut stdout = StdioEnum::Piped;
    let mut stderr = StdioEnum::Piped;

    if let Some(opts) = opts {
        get_gid(&opts, &mut command)?;
        get_uid(&opts, &mut command)?;
        get_cwd(&opts, &mut command)?;
        get_env(&opts, &mut command)?;
        get_stdio(&ctx, &opts, &mut stdin, &mut stdout, &mut stderr)?;
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
    ChildProcess::spawn(ctx.clone(), cmd, command_args, command.spawn())
}

fn exec_file<'js>(
    ctx: Ctx<'js>,
    cmd: String,
    args_and_opts: Rest<Value<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {
    let args_0 = args_and_opts.first();
    let args_1 = args_and_opts.get(1);
    let args_3 = args_and_opts.get(2);

    let cb = get_callback_fn(&ctx, &[args_0, args_1, args_3])?;

    let mut opts = None;
    if let Some(arg) = &args_1 {
        if !arg.is_function() {
            // is_object() is returning true for array, so checking is_array() aswell
            if !arg.is_array() && arg.is_object() {
                opts = arg.as_object().cloned();
            } else {
                return Err(Exception::throw_message(
                    &ctx,
                    "The \"options\" argument must be of type object.",
                ));
            }
        }
    }

    let mut command_args = match get_command_args(&ctx, args_0, &mut opts) {
        Ok(Some(args)) => Some(args),
        Ok(None) => None,
        Err(err) => return Err(err),
    };

    let mut windows_verbatim_arguments: bool = get_windows_verbatim_arguments(opts.as_ref())?;
    let cmd = get_cmd(
        opts.as_ref(),
        &mut command_args,
        &mut windows_verbatim_arguments,
        cmd,
    )?;

    let mut command = StdCommand::new(cmd.clone());
    set_command_args(
        &mut command,
        command_args.as_ref(),
        windows_verbatim_arguments,
    );

    let stdin = StdioEnum::Piped;
    let stdout = StdioEnum::Piped;
    let stderr = StdioEnum::Piped;

    if let Some(opts) = &opts {
        get_gid(opts, &mut command)?;
        get_uid(opts, &mut command)?;
        get_cwd(opts, &mut command)?;
        get_env(opts, &mut command)?;
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
    ChildProcess::exec_file(ctx.clone(), cmd, command_args, command.spawn(), cb)
}


pub struct ChildProcessModule;

impl ModuleDef for ChildProcessModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("spawn")?;
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

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                   import {spawn} from "child_process";

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
    async fn test_exec_file() {
        test_async_with(|ctx| {
            Box::pin(async move {
                buffer::init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"

                    import {execFile} from "child_process";

                    let resolve = null;
                    const deferred = new Promise(res => {
                        resolve = res;
                    });

                    execFile("echo", ["hello"], (error, stdout, stderr)=>{
                        resolve(stdout.trim())
                    })

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

                ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "child_process")
                    .await
                    .unwrap();

                let message: String = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                    import {spawn} from "child_process";

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
}
