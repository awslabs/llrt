// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(windows)]
use std::os::windows::{
    io::{FromRawHandle, RawHandle},
    process::CommandExt,
};
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

use either::Either;
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
    Array, Class, Ctx, Error, Exception, Function, IntoJs, Null, Object, Result, Value,
};

use rquickjs::object::ObjectKeysIter;
use tokio::{
    process::{Child, Command},
    sync::broadcast::{channel as broadcast_channel, Sender}
};

pub mod helpers;
pub mod process;
pub mod normalize_args;

use self::helpers::{get_env, create_error_object, create_output, get_windows_verbatim_arguments,get_cmd, wait_for_process, get_command_args, get_cwd, get_gid, get_output, get_signal, get_stdio, get_uid, process_signal_from_str, set_command_args, StdioEnum};
use self::process::ChildProcess;
use self::normalize_args::{normalize_spawn_args, normalize_exec_file_args};

fn spawn<'js>(
    ctx: Ctx<'js>,
    cmd: String,
    args_0: Opt<Either<Array<'js>, Object<'js>>>,
    args_1: Opt<Object<'js>>,
) -> Result<Class<'js, ChildProcess<'js>>> {

    let (
        cmd, 
        command_args, 
        mut command, 
        stdin, 
        stdout, 
        stderr
    ) = normalize_spawn_args(&ctx, cmd.clone(), args_0, args_1)?;

    command.stdin(stdin.to_stdio());
    command.stdout(stdout.to_stdio());
    command.stderr(stderr.to_stdio());

    #[cfg(unix)]
    {
        command.process_group(0);
    }

    //tokio command does not have all std command features stabilized
    let mut command = Command::from(command);
    ChildProcess::new_spawn(ctx, cmd, command_args, command.spawn())
}

fn print_js_value<'js>(ctx: Ctx<'js>, val: &Value<'js>) -> Result<()> {
    println!("printing js value");
    let global = ctx.globals();
    let json: Function<'js> = global
        .get::<_, Object>("JSON")?
        .get::<_, Function>("stringify")?;
    
    let pretty = json.call::<_, String>((val.clone(), None::<Value>, 2))?;
    println!("{}", pretty);
    Ok(())
}

fn exec_file<'js>(
    ctx: Ctx<'js>,
    cmd: String,
    args_0: Opt<Either<Either<Array<'js>, Object<'js>>, Function<'js>>>,
    args_1: Opt<Either<Object<'js>, Function<'js>>>,
    args_2: Opt<Function<'js>>
) -> Result<Class<'js, ChildProcess<'js>>> {
    // let args_0 = args_and_opts.first();
    // let args_1 = args_and_opts.get(1);
    // let args_3 = args_and_opts.get(2);

    // let mut command_args: Option<Vec<String>>=None;
    // let mut opts: Option<Object<'_>> = None;
    // let mut cb:Option<Function<'_>>=None;

    let (command_args, opts, cb) = normalize_exec_file_args(&ctx, args_0, args_1, args_2)?;

    let args_0 = match command_args {
        Some(args_vec) => {
            let array = Array::new(ctx.clone())?;
            for (i, arg) in args_vec.iter().enumerate() {
                array.set(i as usize, arg.clone())?;
            }
            Opt(Some(Either::Left(array)))
        }
        None => Opt(None),
    };

    // if let Some(data) = command_args {
    //     Opt(Some(Either::Left(data)))
    // }
    println!("tryignnekrjrfgeuirfgeu");
    

    let res=spawn(ctx.clone(), cmd, args_0, Opt(opts))?;
    ChildProcess::add_event_emitter_prototype(&ctx)?;
        DefaultWritableStream::add_writable_stream_prototype(&ctx)?;
        DefaultWritableStream::add_event_emitter_prototype(&ctx)?;
        DefaultReadableStream::add_readable_stream_prototype(&ctx)?;
        DefaultReadableStream::add_event_emitter_prototype(&ctx)?;

    
    let rr= res.clone();
    let cc=rr.as_value();
    // println!("cc{:#?}",cc);
    if let Some(obj) = cc.as_object() {
    println!("--- Inspecting ChildProcess instance ---");

    let keys_iter = obj.keys();

    for key in keys_iter {
        let key: String = key?;
        let val = obj.get::<_, Value>(&key)?;

        let vobj = val.clone().try_into_object();
        match vobj {
            Ok(obj) => {
                println!("obj is printing {:?}", obj.clone());
                
                // let keyss:ObjectKeysIter<'_, String>=obj.keys();
                // for keyy in keyss {
                //     let key: String = keyy?;
                //     let val = obj.get::<_, Value>(&key)?;
                //     println!("aaaaa{:#?}: {:?}", key, val)
                // }
                // for key in obj.keys() {
                //     let key_str = key;
                //     let val = obj.get::<_, Value>(&key)?;
                //     println!("{:#?}: {:?}", key_str, val);
                // }
                let on: Function<'_> = obj.get("on")?;
                let _ = print_js_value(ctx.clone(), &on);

                let ctx_clone1 = ctx.clone();
                let data_callback_stdout = Function::new(ctx_clone1, |chunk: Value| {
                    if let Some(s) = chunk.as_string() {
                        println!("[stdoutres] {:#?}", s);
                    } else {
                        println!("[stdoutres] <non-string>: {:?}", chunk);
                    }
                });

                println!("on is printing {:?}", on);
                println!("try obj type: {:?}", obj.type_of());
                println!("try obj typename: {}", obj.type_name());

                // ✅ Correct stream passed to 'on'
                on.call::<_,Function>(("data",data_callback_stdout))?;
            },
            Err(err) => {
                println!("err,{:#?}", err)
            }
        }

        println!("typeee{:#?}", val.clone().type_of());
        println!("{} => {:?}", key, val);
    }
}



// if let Some(obj) = cc.as_object() {
//     for key in obj.keys() {
//         let key: String = key?;
//         let val = obj.get::<_, Value>(&key)?;

//         if let Some(val_obj) = val.as_object() {
//             let type_name = val_obj.get_type_name();
//             println!("{} => class type: {}", key, type_name);
//         } else {
//             println!("{} => primitive: {:?}", key, val);
//         }
//     }
// }


    // let stdout: Object = rr.get("stdout")?;
    // let stderr: Object = rr.get("stderr")?;
    // // let val = rr.get("stdout")?;
    // let val = rr.as_object();
    // println!("stdout value: {:?}", val);

    // let on_stdout: Function = stdout.get("on")?;
    // println!("on type: {:?}", on_stdout);

    // let on_stderr: Function = stderr.get("on")?;

    // let ctx_clone1 = ctx.clone();
    // let data_callback_stdout = Function::new(ctx_clone1, |chunk: Value| {
    //     if let Some(s) = chunk.as_string() {
    //         println!("[stdoutres] {:#?}", s);
    //     } else {
    //         println!("[stdoutres] <non-string>: {:?}", chunk);
    //     }
    // });

    // let ctx_clone2 = ctx.clone();
    // let data_callback_stderr = Function::new(ctx_clone2, |chunk: String| {
    //     eprintln!("[stderrres] {}", chunk);
    // });

    // on_stdout.call::<_, ()>((stdout.clone(), "data", data_callback_stdout))?;
    // on_stderr.call::<_, ()>((stderr.clone(), "data", data_callback_stderr))?;

    Ok(res)
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

    // #[tokio::test]
    // async fn test_spawn() {
    //     test_async_with(|ctx| {
    //         Box::pin(async move {
    //             buffer::init(&ctx).unwrap();

    //             ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "child_process")
    //                 .await
    //                 .unwrap();

    //             let message: String = ModuleEvaluator::eval_js(
    //                 ctx.clone(),
    //                 "test",
    //                 r#"
    //                import {spawn} from "child_process";

    //                 let resolve = null;
    //                 const deferred = new Promise(res => {
    //                     resolve = res;
    //                 });

    //                 spawn("echo", ["hello"]).stdout.on("data", (data) => {
    //                     resolve(data.toString().trim())
    //                 });

    //                 export default await deferred;

    //             "#,
    //             )
    //             .await
    //             .catch(&ctx)
    //             .unwrap()
    //             .get("default")
    //             .unwrap();

    //             assert_eq!(message, "hello");
    //         })
    //     })
    //     .await;
    // }

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

    // #[tokio::test]
    // async fn test_spawn_shell() {
    //     test_async_with(|ctx| {
    //         Box::pin(async move {
    //             buffer::init(&ctx).unwrap();

    //             ModuleEvaluator::eval_rust::<ChildProcessModule>(ctx.clone(), "child_process")
    //                 .await
    //                 .unwrap();

    //             let message: String = ModuleEvaluator::eval_js(
    //                 ctx.clone(),
    //                 "test",
    //                 r#"
    //                 import {spawn} from "child_process";

    //                 let resolve = null;
    //                 const deferred = new Promise(res => {
    //                     resolve = res;
    //                 });

    //                 spawn("echo", ["hello"], {
    //                     shell: true
    //                 }).stdout.on("data", (data) => {
    //                     resolve(data.toString().trim())
    //                 });

    //                 export default await deferred;
    //             "#,
    //             )
    //             .await
    //             .catch(&ctx)
    //             .unwrap()
    //             .get("default")
    //             .unwrap();

    //             assert_eq!(message, "hello");
    //         })
    //     })
    //     .await;
    // }
}
