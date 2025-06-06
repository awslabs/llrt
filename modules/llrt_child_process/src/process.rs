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

use crate::helpers;

use self::helpers::{get_env, create_error_object, create_output, get_cmd, wait_for_process, get_command_args, get_cwd, get_gid, get_output, get_signal, get_stdio, get_uid, get_windows_verbatim_arguments, process_signal_from_str, set_command_args, StdioEnum};

#[allow(dead_code)]
#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
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

#[rquickjs::methods]
impl<'js> ChildProcess<'js> {
    #[qjs(get)]
    fn pid(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.pid.into_js(&ctx)
    }

    fn kill(&mut self, signal: Opt<Value<'js>>) -> Result<bool> {
        #[cfg(unix)]
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
            process_signal_from_str("SIGTERM")
        };

        #[cfg(not(unix))]
        {
            _ = signal;
        }
        #[cfg(not(unix))]
        let signal = Some(9); // SIGKILL

        if let Some(kill_signal_tx) = self.kill_signal_tx.take() {
            return Ok(kill_signal_tx.send(signal).is_ok());
        }

        Ok(false)
    }
}

impl<'js> ChildProcess<'js> {
    pub fn new_spawn(
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
        println!("new spawn ooo");

        let stdout_instance = DefaultReadableStream::new(ctx.clone())?;
        let stderr_instance = DefaultReadableStream::new(ctx.clone())?;
        let stdin_instance = DefaultWritableStream::new(ctx.clone())?;
        let instance = Class::instance(ctx.clone(), instance)?;
        let instance2 = instance.clone();
        let instance3 = instance.clone();
        let instance4 = instance.clone();

        instance.set("stderr", stderr_instance.clone())?;
        println!("Set stdout");
        instance.set("stdout", stdout_instance.clone())?;
        println!("Set stderr");
        instance.set("stdin", stdin_instance.clone())?;
        println!("Set stdin");

        println!("new spawn");

        match child {
            Ok(mut child) => {
                instance2.borrow_mut().pid = child.id();

                if let Some(child_stdin) = child.stdin.take() {
                    DefaultWritableStream::process(stdin_instance.clone(), &ctx, child_stdin)?;
                };

                println!("abcdddd");

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
                            &mut false,
                        )
                        .await?;

                        let code = exit_code.unwrap_or_default().into_js(&ctx3)?;

                        let signal = get_signal(&ctx3, exit_signal)?;

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

        println!("Its hereeee");

        Ok(instance)
    }

    // pub fn exec_file(
    //     ctx: Ctx<'js>,
    //     command: String,
    //     args: Option<Vec<String>>,
    //     child: IoResult<Child>,
    //     cb: Option<Function<'js>>,
    // ) -> Result<Class<'js, Self>> {
    //     let (kill_signal_tx, kill_signal_rx) = broadcast_channel::<Option<i32>>(1);

    //     let instance = Self {
    //         emitter: EventEmitter::new(),
    //         command: command.clone(),
    //         args: args.clone(),
    //         pid: None,
    //         kill_signal_tx: Some(kill_signal_tx),
    //     };

    //     let stdout_instance = DefaultReadableStream::new(ctx.clone())?;
    //     let stderr_instance = DefaultReadableStream::new(ctx.clone())?;
    //     let stdin_instance = DefaultWritableStream::new(ctx.clone())?;

    //     let instance = Class::instance(ctx.clone(), instance)?;
    //     let instance2 = instance.clone();
    //     let instance3 = instance.clone();
    //     let instance4 = instance.clone();

    //     instance.set("stderr", stderr_instance.clone())?;
    //     instance.set("stdout", stdout_instance.clone())?;
    //     instance.set("stdin", stdin_instance.clone())?;

    //     match child {
    //         Ok(mut child) => {
    //             instance2.borrow_mut().pid = child.id();

    //             if let Some(child_stdin) = child.stdin.take() {
    //                 DefaultWritableStream::process(stdin_instance.clone(), &ctx, child_stdin)?;
    //             };

    //             let stdout_new: Vec<u8> = Vec::new();
    //             let stderr_new: Vec<u8> = Vec::new();
    //             let stdout_arc = Arc::new(Mutex::new(stdout_new));
    //             let stderr_arc = Arc::new(Mutex::new(stderr_new));
    //             let combined_stdout_buffer = Some(Arc::clone(&stdout_arc));
    //             let combined_stderr_buffer = Some(Arc::clone(&stderr_arc));

    //             let stdout_join_receiver = get_output(
    //                 &ctx,
    //                 child.stdout.take(),
    //                 stdout_instance.clone()
    //             )?;

    //             let stderr_join_receiver = get_output(
    //                 &ctx,
    //                 child.stderr.take(),
    //                 stderr_instance.clone()
    //             )?;

    //             let ctx2 = ctx.clone();
    //             let ctx3 = ctx.clone();

    //             ctx.clone().spawn_exit(async move {
    //                 let spawn_proc = async move {
    //                     let mut exit_code = None;
    //                     let mut exit_signal = None;
    //                     let mut killed = false;

    //                     wait_for_process(
    //                         child,
    //                         &ctx3,
    //                         kill_signal_rx,
    //                         &mut exit_code,
    //                         &mut exit_signal,
    //                         &mut killed,
    //                     )
    //                     .await?;

    //                     let code = exit_code.unwrap_or_default().into_js(&ctx3)?;
    //                     let signal = get_signal(&ctx3, exit_signal)?;

    //                     ChildProcess::emit_str(
    //                         This(instance2.clone()),
    //                         &ctx3,
    //                         "exit",
    //                         vec![code.clone(), signal.clone()],
    //                         false,
    //                     )?;

    //                     if let Some(stderr_join_receiver) = stderr_join_receiver {
    //                         //ok if sender drops
    //                         let _ = stderr_join_receiver.await;
    //                     }
    //                     if let Some(stdout_join_receiver) = stdout_join_receiver {
    //                         //ok if sender drops
    //                         let _ = stdout_join_receiver.await;
    //                     }

    //                     WritableStream::end(This(stdin_instance));

    //                     ChildProcess::emit_str(
    //                         This(instance2.clone()),
    //                         &ctx3,
    //                         "close",
    //                         vec![code.clone(), signal.clone()],
    //                         false,
    //                     )?;

    //                     if let Some(cb) = cb {
    //                         match killed {
    //                             true => {
    //                                 // Even though we killed the process we need to display whatever we collected to buffer.
    //                                 let stdout_data = combined_stdout_buffer
    //                                     .as_ref()
    //                                     .map(|stdout| stdout.lock().unwrap());

    //                                 let stdout: Result<Value<'js>> = match stdout_data {
    //                                     Some(data) if !data.is_empty() => {
    //                                         let message = String::from_utf8_lossy(&data);
    //                                         message.into_js(&ctx3)
    //                                     },
    //                                     _ => "".into_js(&ctx3),
    //                                 };

    //                                 let error_object = create_error_object(
    //                                     &ctx3, args, command, code, killed, signal, None,
    //                                 )?;

    //                                 () = cb.call((
    //                                     error_object.into_js(&ctx3),
    //                                     stdout,
    //                                     "".into_js(&ctx3),
    //                                 ))?;
    //                             },
    //                             false => {
    //                                 if let Some(stdout) = combined_stdout_buffer {
    //                                     let data = stdout.lock().unwrap();
    //                                     if !data.is_empty() {
    //                                         let message = String::from_utf8_lossy(&data);
    //                                         () = cb.call((
    //                                             Null.into_js(&ctx3),
    //                                             message.into_js(&ctx3),
    //                                             "".into_js(&ctx3),
    //                                         ))?;
    //                                     }
    //                                 }

    //                                 if let Some(stderr) = combined_stderr_buffer {
    //                                     let data = stderr.lock().unwrap();
    //                                     if !data.is_empty() {
    //                                         let error_object = create_error_object(
    //                                             &ctx3,
    //                                             args,
    //                                             command,
    //                                             code,
    //                                             killed,
    //                                             signal,
    //                                             Some(data),
    //                                         )?;
    //                                         let err_message: Value<'js> =
    //                                             error_object.get("message")?;

    //                                         () = cb.call((
    //                                             error_object.into_js(&ctx3),
    //                                             "".into_js(&ctx3),
    //                                             err_message,
    //                                         ))?;
    //                                     }
    //                                 }
    //                             },
    //                         }
    //                     }

    //                     Ok::<_, Error>(())
    //                 };

    //                 spawn_proc
    //                     .await
    //                     .emit_error("child_process", &ctx2, instance4)?;

    //                 Ok(())
    //             })?;
    //         },
    //         Err(err) => {
    //             let ctx3 = ctx.clone();

    //             let err_message = format!("Child process failed to spawn \"{}\". {}", command, err);

    //             ctx.spawn_exit(async move {
    //                 let ex = Exception::from_message(ctx3.clone(), &err_message)?;
    //                 ChildProcess::emit_str(
    //                     This(instance3),
    //                     &ctx3,
    //                     "error",
    //                     vec![ex.clone().into()],
    //                     false,
    //                 )?;

    //                 if let Some(cb) = cb {
    //                     () = cb.call((ex, "".into_js(&ctx3), "".into_js(&ctx3)))?;
    //                 }
    //                 Ok(())
    //             })?;
    //         },
    //     }
    //     Ok(instance)
    // }
}

impl<'js> Emitter<'js> for ChildProcess<'js> {
    fn get_event_list(&self) -> Arc<RwLock<EventList<'js>>> {
        self.emitter.get_event_list()
    }
}