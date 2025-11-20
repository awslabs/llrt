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

use url::Url;

use llrt_url::{file_url_to_path};

use std::{
    borrow::Cow,
    collections::HashMap,
    io::Result as IoResult,
    process::{Command as StdCommand, Stdio},
    sync::{Arc, Mutex, MutexGuard, RwLock},
};

use either::Either;
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
    class::{Trace, Tracer}, convert::Coerced, module::{Declarations, Exports, ModuleDef}, prelude::{Func, Opt, Rest, This}, Array, Class, Ctx, Error, Exception, Function, IntoJs, Null, Object, Result, Undefined, Value
};
use tokio::{
    io::AsyncRead,
    process::{Child, Command},
    sync::{
        broadcast::{channel as broadcast_channel, Receiver, Sender},
        oneshot::Receiver as OneshotReceiver,
    },
};

#[derive(Clone)]
pub enum StdioEnum {
    Piped,
    Ignore,
    Inherit,
    Fd(i32),
}

impl StdioEnum {
    pub fn to_stdio(&self) -> Stdio {
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

#[cfg(unix)]
macro_rules! generate_signal_from_str_fn {
    ($($signal:path),*) => {
        pub fn process_signal_from_str(signal: &str) -> Option<i32> {
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

             return None;
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

#[allow(unused_variables)]
pub fn prepare_shell_args(
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

pub async fn wait_for_process(
    mut child: Child,
    ctx: &Ctx<'_>,
    mut kill_signal_rx: Receiver<Option<i32>>,
    exit_code: &mut Option<i32>,
    exit_signal: &mut Option<i32>,
    killed: &mut bool,
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
                #[cfg(not(unix))]
                {
                    _ = exit_signal;
                }
                break;
            }
            Ok(signal) = kill_signal_rx.recv() => {
                #[cfg(unix)]
                {
                    if let Some(signal) = signal {
                        if let Some(pid) = child.id() {
                            if unsafe { libc::killpg(pid as i32, signal) } == 0 {
                                *killed=true;
                                continue;
                            } else {
                               return Err(Exception::throw_message(ctx, &["Failed to send signal ",itoa::Buffer::new().format(signal)," to process ", itoa::Buffer::new().format(pid)].concat()));
                            }
                        }
                    } else {
                        child.kill().await.or_throw(ctx)?;
                        *killed=true;
                        break;
                    }
                }
                #[cfg(not(unix))]
                {
                    _ = signal;
                    child.kill().await.or_throw(ctx)?;
                    *killed=true;
                    break;
                }
            },
        }
    }

    Ok(())
}

// pub fn normalize_exec_file_args<'js>(
//     ctx: &Ctx<'js>,
//     args_0: Opt<Either<Either<Array<'js>, Object<'js>>, Function<'js>>>,
//     args_1: Opt<Either<Object<'js>, Function<'js>>>,
//     args_2: Opt<Function<'js>>
// ) -> Result<Option<Function<'js>>> {
//     let callback = match args_0.0 {
//         Some(Either::Right(callback)) => {
//             if callback.is_function() {
//                 callback
//             }
//         },
//         Some(Either::Left(Either::Right(callback))) => {
//             if callback.is_function() {
//                 callback
//             }
//         },
//         Some(Either::Left(Either::Left(callback))) => {
//             if callback.is_function() {
//                 callback
//             }
//         },
//     };

//     // println!("valtest{:#?}",args_0.0);
//     Ok(None)
// }
// pub fn get_callback_fn<'js>(
//     ctx: &Ctx<'js>,
//     args: &[Option<&Value<'js>>],
// ) -> Result<Option<Function<'js>>> {
//     for (i, arg) in args.iter().enumerate() {
//         if let Some(arg) = arg {
//             if let Some(func) = arg.as_function() {
//                 return Ok(Some(func.clone()));
//             }
//             if i == 2 {
//                 return Err(Exception::throw_message(
//                     ctx,
//                     "The \"callback\" argument must be of type function.",
//                 ));
//             }
//         }
//     }
//     Ok(None)
// }

pub fn extract_args_array<'js>(ctx: &Ctx<'js>, array: Array<'js>) -> Result<Vec<String>> {
    let args = array.as_array().or_throw(ctx)?;
    let mut args_vec = Vec::with_capacity(args.len());
    for arg in args.iter() {
        let val: Value = arg?;
        let val = val
            .as_string()
            .or_throw_msg(ctx, "argument is not a string")?;
        args_vec.push(val.to_string()?);
    }
    Ok(args_vec)
}


pub fn get_command_args<'js>(
    ctx: &Ctx<'_>,
    args_0: Option<&Value<'js>>,
    opts: &mut Option<Object<'js>>,
) -> Result<Option<Vec<String>>> {
    let command_args = if let Some(args_0) = args_0 {
        if args_0.is_array() {
            let args = args_0.clone().into_array().or_throw(ctx)?;
            let mut args_vec = Vec::with_capacity(args.len());
            for arg in args.iter() {
                let arg: Value = arg?;
                let arg = arg
                    .as_string()
                    .or_throw_msg(ctx, "argument is not a string")?;
                let arg = arg.to_string()?;
                args_vec.push(arg);
            }
            Some(args_vec)
        } else if args_0.is_object() {
            *opts = args_0.as_object().map(|o| o.to_owned());
            None
        } else if args_0.is_string() {
            return Err(Exception::throw_message(
                ctx,
                "The \"args\" argument must be of type object",
            ));
        } else {
            None
        }
    } else {
        None
    };
    Ok(command_args)
}

pub fn validate_string_length(ctx: &Ctx<'_>, cmd: &str) -> Result<()> {
    if cmd.is_empty() {
        return Err(Exception::throw_message(
                ctx,
                "File cannot be empty",
            ));
    }

    Ok(())
}

pub fn is_url_str(input: &str) -> bool {
    Url::parse(input).is_ok()
}

pub fn to_path_if_file_url(ctx: &Ctx<'_>, url: &str) -> Result<String> {
    if !is_url_str(url) {
        return Ok(url.to_string());
    }

    let url_value = url.into_js(ctx).or_throw(ctx)?;
    file_url_to_path(ctx.clone(), url_value)
}

// pub fn validate_path(ctx: &Ctx<'_>, path: Result<String>, prop_name: &str) -> Result<()> {
//     let message=format!("{} must not contain null bytes", prop_name);
//     if path.is_some() {
//         if path.contains('\0') {
//         return Err(Exception::throw_type(
//                 ctx,
//                 &message,
//             ));
//     }
//     }

//     Ok(())
// }

pub fn validate_path(ctx: &Ctx<'_>, path: &str, prop_name: &str) -> Result<()> {
            let message = format!(
                "{:?} must be without null bytes.",
                prop_name,
            );

            if path.contains('\0') {
                return Err(Exception::throw_type(
                    ctx,
                    &message,
                ));
            }
            Ok(())
        }

pub fn get_validated_path(ctx: &Ctx<'_>, url: &str, prop_name: &str) -> Result<()> {
    let path= to_path_if_file_url(ctx, url);
    let result = path?;
    validate_path(ctx, &result, prop_name)?;
    Ok(())
}

pub fn validate_argument_null_check(ctx: &Ctx<'_>, cmd: &str) -> Result<()> {
    if cmd.contains('\0') {
        return Err(Exception::throw_type(
                ctx,
                "argument must be a string without null bytes",
            ));
    }
    Ok(())
}

pub fn validate_arguments_null_check(ctx: &Ctx<'_>, command_args: Option<&Vec<String>>) -> Result<()> {
    if let Some(cmds) = command_args {
        for arg in cmds.iter() {
            validate_argument_null_check(&ctx, arg)?
        }
    }
    Ok(())
}

pub fn get_windows_verbatim_arguments(opts: Option<&Object<'_>>) -> Result<bool> {
    let windows_verbatim_arguments: bool = if let Some(opts) = &opts {
        opts.get_optional::<&str, bool>("windowsVerbatimArguments")?
            .unwrap_or_default()
    } else {
        false
    };
    Ok(windows_verbatim_arguments)
}

pub fn get_cmd(
    opts: Option<&Object<'_>>,
    command_args: &mut Option<Vec<String>>,
    windows_verbatim_arguments: &mut bool,
    cmd: String,
) -> Result<String> {
    let cmd = if let Some(opts) = opts {
        if opts
            .get_optional::<&str, bool>("shell")?
            .unwrap_or_default()
        {
            #[cfg(windows)]
            let shell = "cmd.exe".to_string();
            #[cfg(not(windows))]
            let shell = "/bin/sh".to_string();
            *command_args = Some(prepare_shell_args(
                &shell,
                windows_verbatim_arguments,
                cmd,
                command_args.take(),
            ));
            shell
        } else if let Some(shell) = opts.get_optional::<&str, String>("shell")? {
            *command_args = Some(prepare_shell_args(
                &shell,
                windows_verbatim_arguments,
                cmd,
                command_args.take(),
            ));
            shell
        } else {
            cmd
        }
    } else {
        cmd
    };
    Ok(cmd)
}

pub fn get_gid(opts: &Object<'_>, command: &mut std::process::Command) -> Result<()> {
    #[cfg(unix)]
    if let Some(gid) = opts.get_optional("gid")? {
        command.gid(gid);
    }
    Ok(())
}

pub fn get_uid(opts: &Object<'_>, command: &mut std::process::Command) -> Result<()> {
    #[cfg(unix)]
    if let Some(uid) = opts.get_optional("uid")? {
        command.gid(uid);
    }
    Ok(())
}

pub fn get_cwd(ctx: &Ctx<'_> ,opts: &Object<'_>, command: &mut std::process::Command) -> Result<()> {
    if let Some(cwd) = opts.get_optional::<_, String>("cwd")? {
        get_validated_path(&ctx, &cwd, "cwd")?;
        command.current_dir(&cwd);
    }
    Ok(())
}

pub fn get_env(opts: &Object<'_>, command: &mut std::process::Command) -> Result<()> {
    if let Some(env) = opts.get_optional::<_, HashMap<String, Coerced<String>>>("env")? {
        let env: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        command.env_clear();
        command.envs(env);
    }
    Ok(())
}

pub fn get_stdio<'js>(
    ctx: &Ctx<'_>,
    opts: &Object<'js>,
    stdin: &mut StdioEnum,
    stdout: &mut StdioEnum,
    stderr: &mut StdioEnum,
) -> Result<()> {
    if let Some(stdio) = opts.get_optional::<_, Value<'js>>("stdio")? {
        if let Some(stdio_str) = stdio.as_string() {
            let stdio = str_to_stdio(ctx, &stdio_str.to_string()?)?;
            *stdin = stdio.clone();
            *stdout = stdio.clone();
            *stderr = stdio;
        } else if let Some(stdio) = stdio.as_array() {
            for (i, item) in stdio.iter::<Value>().enumerate() {
                let item = item?;
                let stdio = if item.is_undefined() || item.is_null() {
                    StdioEnum::Piped
                } else if let Some(std_io_str) = item.as_string() {
                    str_to_stdio(ctx, &std_io_str.to_string()?)?
                } else if let Some(fd) = item.as_number() {
                    StdioEnum::Fd(fd as i32)
                } else {
                    StdioEnum::Piped
                };
                match i {
                    0 => *stdin = stdio,
                    1 => *stdout = stdio,
                    2 => *stderr = stdio,
                    _ => {
                        break;
                    },
                }
            }
        }
    }
    Ok(())
}

#[allow(unused_variables)]
pub fn set_command_args(
    command: &mut std::process::Command,
    args: Option<&Vec<String>>,
    windows_verbatim_arguments: bool,
) {
    if let Some(args) = args {
        #[cfg(windows)]
        {
            if windows_verbatim_arguments {
                command.raw_arg(args.join(" "));
            } else {
                command.args(args);
            }
        }

        #[cfg(not(windows))]
        {
            command.args(args);
        }
    }
}

pub fn get_signal<'js>(ctx3: &Ctx<'js>, exit_signal: Option<i32>) -> Result<Value<'js>> {
    let signal;
    #[cfg(unix)]
    {
        if let Some(s) = exit_signal {
            signal = signal_str_from_i32(s).into_js(ctx3)?;
        } else {
            signal = Undefined.into_value(ctx3.clone());
        }
    }
    #[cfg(not(unix))]
    {
        signal = "SIGKILL".into_js(&ctx3)?;
    }
    Ok(signal)
}

pub fn str_to_stdio(ctx: &Ctx<'_>, input: &str) -> Result<StdioEnum> {
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

pub fn get_output<'js, T>(
    ctx: &Ctx<'js>,
    output: Option<T>,
    native_readable_stream: Class<'js, DefaultReadableStream<'js>>
) -> Result<Option<OneshotReceiver<bool>>>
where
    T: AsyncRead + Unpin + Send + 'static,
{
    if let Some(output) = output {
        let receiver = DefaultReadableStream::process(
            native_readable_stream,
            ctx,
            output
        )?;
        return Ok(Some(receiver));
    }

    Ok(None)
}

pub fn create_output<'js, T>(
    ctx: &Ctx<'js>,
    output: Option<T>,
    native_readable_stream: Class<'js, DefaultReadableStream<'js>>,
) -> Result<Option<OneshotReceiver<bool>>>
where
    T: AsyncRead + Unpin + Send + 'static,
{
    get_output(ctx, output, native_readable_stream)
}

pub fn create_error_object<'js>(
    ctx3: &Ctx<'js>,
    args: Option<Vec<String>>,
    command: String,
    code: Value<'js>,
    killed: bool,
    signal: Value<'js>,
    data: Option<MutexGuard<'_, Vec<u8>>>,
) -> Result<Object<'js>> {
    let arg = args.unwrap_or_default();
    let cmd = format!("{} {}", command, arg.join(" "));
    let message: Cow<'_, str> = if killed {
        format!("Error: Command failed:{} {}", command, arg.join(" ")).into()
    } else if let Some(ref data) = data {
        String::from_utf8_lossy(data)
    } else {
        "".into()
    };

    let error_object = Object::new(ctx3.clone())?;
    error_object.set("message", message.into_js(ctx3))?;
    error_object.set("code", code)?;
    error_object.set("killed", killed)?;
    error_object.set("signal", signal.into_js(ctx3))?;
    error_object.set("cmd", cmd)?;

    Ok(error_object)
}