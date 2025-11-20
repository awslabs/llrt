use rquickjs::{
    prelude::Opt, Array, Ctx, Exception, Function, IntoJs, Object, Result, Value,
    convert::Coerced,
};
use tokio::process::Command;

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
use either::Either;

// use rquickjs::{
//     class::{Trace, Tracer}, convert::Coerced, module::{Declarations, Exports, ModuleDef}, prelude::{Func, Opt, Rest, This}, Array, Class, Ctx, Error, Exception, Function, IntoJs, Null, Object, Result, Undefined, Value
// };

use std::{
    io::Result as IoResult,
    process::Command as StdCommand,
    sync::{Arc, Mutex, RwLock},
    collections::HashMap,
};

use llrt_utils::{
    object::ObjectExt,
    result::ResultExt,
};

use crate::helpers::{get_cmd,prepare_shell_args,str_to_stdio, get_cwd,set_command_args,get_gid,get_uid,get_env,get_stdio,  extract_args_array, to_path_if_file_url,get_validated_path, is_url_str, validate_argument_null_check, validate_arguments_null_check, validate_string_length, StdioEnum};

pub fn normalize_exec_file_args<'js>(
    ctx: &Ctx<'js>,
    args_0: Opt<Either<Either<Array<'js>, Object<'js>>, Function<'js>>>,
    args_1: Opt<Either<Object<'js>, Function<'js>>>,
    args_2: Opt<Function<'js>>,
) -> Result<(Option<Vec<String>>, Option<Object<'js>>, Option<Function<'js>>)> {
    let mut command_args = None;
    let mut opts = None;
    let mut cb = None;

    // Handle args_0: can be [args], {options}, or callback
    if let Some(arg) = args_0.0 {
        match arg {
            Either::Left(inner) => match inner {
                Either::Left(array) => {
                    command_args = Some(extract_args_array(ctx, array)?);
                }
                Either::Right(obj) => {
                    opts = obj.as_object().map(|o| o.to_owned());
                }
            },
            Either::Right(func) => {
                cb = Some(func);
            }
        }
    }

    // Handle args_1: can be {options} or callback
    if let Some(arg) = args_1.0 {
        match arg {
            Either::Left(obj) => {
                if opts.is_none() {
                    opts = obj.as_object().map(|o| o.to_owned());
                } else {
                    return Err(Exception::throw_message(
                        ctx,
                        "The \"callback\" argument must be of type function.",
                    ));
                }
            }
            Either::Right(func) => {
                if cb.is_none() {
                    cb = Some(func);
                }
            }
        }
    }

    // Handle args_2: always a callback if present
    if let Some(func) = args_2.0 {
        cb = Some(func);
    }

    Ok((command_args, opts, cb))
}


// fn print_opts(ctx: &Ctx, opts: Option<Value>) -> Result<()> {
//     if let Some(value) = opts {
//         if let Ok(obj) = Object::from_value(ctx.clone(), value.clone()) {
//             for entry in obj.entries::<String, Value>()? {
//                 let (key, val) = entry?;
//                 println!("{}: {}", key, val.to_string(ctx)?);
//             }
//         } else {
//             println!("Value is not an object.");
//         }
//     } else {
//         println!("opts is None");
//     }
//     Ok(())
// }

pub fn normalize_spawn_args<'js>(
    ctx: &Ctx<'js>,
    cmd: String,
    args_0: Opt<Either<Array<'js>, Object<'js>>>,
    args_1: Opt<Object<'js>>,
) -> Result<(String, Option<Vec<String>>, StdCommand, StdioEnum, StdioEnum, StdioEnum )> {
    let mut command_args = None;
    let mut opts = None;

    // String length should be greater than 0
    validate_string_length(&ctx, &cmd)?;

    // Validation for \u0000
    validate_argument_null_check(&ctx, &cmd)?;

    match args_0.0 {
        Some(Either::Left(array)) => {
            command_args = Some(extract_args_array(ctx, array)?);
        }
        Some(Either::Right(obj)) => {
            opts = obj.as_object().map(|o| o.to_owned());
        }
        None => {}
    }

    if let Some(obj) = args_1.0 {
        opts = obj.as_object().map(|o| o.to_owned());
    }

    let mut windows_verbatim_arguments: bool = if let Some(opts) = &opts {
        opts.get_optional::<&str, bool>("windowsVerbatimArguments")?
            .unwrap_or_default()
    } else {
        false
    };

    let cmd = if let Some(opts) = &opts {
        if opts
            .get_optional::<&str, bool>("shell")?
            .unwrap_or_default()
        {
            #[cfg(windows)]
            let shell = "cmd.exe".to_string();
            #[cfg(not(windows))]
            let shell = "/bin/sh".to_string();
            command_args = Some(prepare_shell_args(
                &shell,
                &mut windows_verbatim_arguments,
                cmd,
                command_args,
            ));
            shell
        } else if let Some(shell) = opts.get_optional::<&str, String>("shell")? {
            command_args = Some(prepare_shell_args(
                &shell,
                &mut windows_verbatim_arguments,
                cmd,
                command_args,
            ));
            shell
        } else {
            cmd
        }
    } else {
        cmd
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

    validate_arguments_null_check(&ctx, command_args.as_ref())?;

    let mut stdin = StdioEnum::Piped;
    let mut stdout = StdioEnum::Piped;
    let mut stderr = StdioEnum::Piped;

    if let Some(opts) = opts {
        #[cfg(unix)]
        if let Some(gid) = opts.get_optional("gid")? {
            command.gid(gid);
        }

        #[cfg(unix)]
        if let Some(uid) = opts.get_optional("uid")? {
            command.gid(uid);
        }

        if let Some(cwd) = opts.get_optional::<_, String>("cwd")? {
            get_validated_path(&ctx, &cwd, "cwd")?;
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

    Ok((cmd, command_args, command, stdin, stdout, stderr))
}
