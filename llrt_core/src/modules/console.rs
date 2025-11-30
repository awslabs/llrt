// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::{
    env,
    fmt::Write as FormatWrite,
    io::{stderr, stdout, IsTerminal, Write},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use jiff::Timestamp;
use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    object::Property,
    prelude::{Func, Rest},
    Array, Class, Ctx, Object, Result, Value,
};

use crate::libs::{
    json::{escape::escape_json, stringify::json_stringify},
    logging::{
        build_formatted_string, replace_newline_with_carriage_return, FormatOptions, LogLevel,
        NEWLINE, TIME_FORMAT,
    },
    utils::{
        class::get_class_name,
        module::{export_default, ModuleInfo},
    },
};
use crate::runtime_client;

static AWS_LAMBDA_MODE: AtomicBool = AtomicBool::new(false);
static AWS_LAMBDA_JSON_LOG_FORMAT: AtomicBool = AtomicBool::new(false);
static AWS_LAMBDA_JSON_LOG_LEVEL: AtomicUsize = AtomicUsize::new(LogLevel::Info as usize);

fn lambda_mode_initializer() {
    let aws_lambda_json_log_format = env::var("AWS_LAMBDA_LOG_FORMAT") == Ok("JSON".to_string());
    let aws_lambda_log_level = env::var("AWS_LAMBDA_LOG_LEVEL").unwrap_or_default();
    let log_level = LogLevel::from_str(&aws_lambda_log_level);

    AWS_LAMBDA_JSON_LOG_LEVEL.store(log_level as usize, Ordering::Relaxed);
    AWS_LAMBDA_MODE.store(true, Ordering::Relaxed);
    AWS_LAMBDA_JSON_LOG_FORMAT.store(aws_lambda_json_log_format, Ordering::Relaxed);
}

#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
#[rquickjs::class]
pub struct Console {}

#[rquickjs::methods(rename_all = "camelCase")]
impl Console {
    #[qjs(constructor)]
    pub fn new() -> Self {
        // We ignore the parameters for now since we don't support stream
        Self {}
    }

    pub fn log<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log(ctx, args)
    }
    pub fn clear(&self) {
        clear()
    }
    pub fn debug<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_debug(ctx, args)
    }
    pub fn info<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log(ctx, args)
    }
    pub fn trace<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_trace(ctx, args)
    }
    pub fn error<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_error(ctx, args)
    }
    pub fn warn<'js>(&self, ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
        log_warn(ctx, args)
    }
    pub fn assert<'js>(
        &self,
        ctx: Ctx<'js>,
        expression: bool,
        args: Rest<Value<'js>>,
    ) -> Result<()> {
        log_assert(ctx, expression, args)
    }
}

pub fn log_fatal<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stderr(), &ctx, args, LogLevel::Fatal)
}

pub fn log_error<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stderr(), &ctx, args, LogLevel::Error)
}

fn log_warn<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stderr(), &ctx, args, LogLevel::Warn)
}

fn log_debug<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stdout(), &ctx, args, LogLevel::Debug)
}

fn log_trace<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stdout(), &ctx, args, LogLevel::Trace)
}

fn log_assert<'js>(ctx: Ctx<'js>, expression: bool, args: Rest<Value<'js>>) -> Result<()> {
    if !expression {
        write_log(stderr(), &ctx, args, LogLevel::Error)?
    }
    Ok(())
}

fn log<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<()> {
    write_log(stdout(), &ctx, args, LogLevel::Info)
}

fn clear() {
    if stdout().is_terminal() {
        let _ = stdout().write_all(b"\x1b[1;1H\x1b[0J");
    }
}

#[allow(clippy::unused_io_amount)]
fn write_log<'js, T>(
    mut output: T,
    ctx: &Ctx<'js>,
    args: Rest<Value<'js>>,
    level: LogLevel,
) -> Result<()>
where
    T: Write + IsTerminal,
{
    let is_tty = output.is_terminal();
    let mut result = String::new();
    let mut is_lambda_mode = AWS_LAMBDA_MODE.load(Ordering::Relaxed);

    if is_lambda_mode && is_tty {
        is_lambda_mode = false;
    }

    if is_lambda_mode {
        let is_json_log_format = AWS_LAMBDA_JSON_LOG_FORMAT.load(Ordering::Relaxed);
        let max_log_level = AWS_LAMBDA_JSON_LOG_LEVEL.load(Ordering::Relaxed);
        if !write_lambda_log(
            ctx,
            &mut result,
            args,
            level,
            is_tty,
            is_json_log_format,
            max_log_level,
            TIME_FORMAT,
        )? {
            return Ok(());
        }
    } else {
        let mut options = FormatOptions::new(ctx, is_tty, true)?;
        build_formatted_string(&mut result, ctx, args, &mut options)?;
    }

    result.push(NEWLINE);

    //we don't care if output is interrupted
    let _ = output.write_all(result.as_bytes());

    Ok(())
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn write_lambda_log<'js>(
    ctx: &Ctx<'js>,
    result: &mut String,
    args: Rest<Value<'js>>,
    level: LogLevel,
    is_tty: bool,
    is_json_log_format: bool,
    max_log_level: usize,
    time_format: &str,
) -> Result<bool> {
    let mut is_newline = true;

    //do not log if we don't meet the log level
    if is_json_log_format && (level.clone() as usize) < max_log_level {
        return Ok(false);
    }
    result.reserve(64);
    if !is_tty {
        is_newline = false;
    }

    let current_time = Timestamp::now();
    let formatted_time = current_time.strftime(time_format);
    let request_id = runtime_client::LAMBDA_REQUEST_ID.read().unwrap();

    if is_json_log_format {
        result.push('{');
        //time
        result.push_str("\"time\":\"");
        write!(result, "{}", formatted_time).unwrap();
        result.push_str("\",");

        //request id
        if let Some(id) = request_id.as_ref() {
            result.push_str("\"requestId\":\"");
            result.push_str(id);
            result.push_str("\",");
        }

        //level
        result.push_str("\"level\":\"");
        result.push_str(&level.to_string());
        result.push('\"');
    } else {
        write!(result, "{}", formatted_time).unwrap();
        result.push('\t');

        match request_id.as_ref() {
            Some(id) => result.push_str(id),
            None => result.push_str("n/a"),
        }

        result.push('\t');
        result.push_str(&level.to_string());
        result.push('\t');
    }

    if is_json_log_format {
        let mut values_string = String::with_capacity(64);

        if args.0.len() == 1 {
            let mut first_arg = unsafe { args.0.first().unwrap_unchecked() }.clone();

            if first_arg.is_error() || first_arg.is_exception() {
                if let Some(exception) = first_arg.as_exception() {
                    let obj = Object::new(ctx.clone())?;
                    obj.set("errorType", get_class_name(exception.as_value()))?;
                    if let Some(message) = exception.message() {
                        obj.set("errorMessage", message)?;
                    }
                    if let Some(stack) = exception.stack() {
                        let stack_object = Array::new(ctx.clone())?;

                        for (i, trace) in stack.split('\n').enumerate() {
                            stack_object.set(i, String::from(trace))?;
                        }
                        obj.set("stackTrace", stack_object)?;
                    }
                    first_arg = obj.into_value();
                }
            }
            if let Some(json_string) = json_stringify(ctx, first_arg)? {
                //message
                result.push(',');
                result.push_str("\"message\":");
                result.push_str(&json_string);
            }
        } else {
            //message
            result.push(',');
            result.push_str("\"message\":\"");

            let mut exception = None;

            let mut options = FormatOptions::new(ctx, is_tty, true)?;

            for arg in args.0.iter() {
                if arg.is_error() && exception.is_none() {
                    let exception_value = arg.clone();
                    exception = Some(exception_value.into_exception().unwrap());
                    break;
                }
            }

            build_formatted_string(&mut values_string, ctx, args, &mut options)?;

            result.push_str(&escape_json(values_string.as_bytes()));
            result.push('\"');
            if let Some(exception) = exception {
                //error type
                result.push_str(",\"errorType\":\"");
                result
                    .push_str(&get_class_name(exception.as_value())?.unwrap_or("Exception".into()));
                result.push_str("\",");

                //error message
                if let Some(message) = exception.message() {
                    result.push_str("\"errorMessage\":\"");
                    result.push_str(&message);
                    result.push_str("\",");
                }

                //stack trace
                result.push_str("\"stackTrace\":[");
                let mut write_comma = false;
                if let Some(stack) = exception.stack() {
                    if !stack.is_empty() {
                        for trace in stack.split('\n') {
                            if write_comma {
                                result.push(',');
                            }
                            result.push('\"');
                            result.push_str(trace);
                            result.push('\"');
                            write_comma = true;
                        }
                    }
                }

                result.push(']');
            }
        }

        result.push('}');
    } else {
        let mut options = FormatOptions::new(ctx, is_tty && !is_json_log_format, is_newline)?;
        build_formatted_string(result, ctx, args, &mut options)?;

        replace_newline_with_carriage_return(result);
    }

    Ok(true)
}

pub struct ConsoleModule;

impl ModuleDef for ConsoleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(Console))?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<Console>::define(default)?;

            Ok(())
        })
    }
}

impl From<ConsoleModule> for ModuleInfo<ConsoleModule> {
    fn from(val: ConsoleModule) -> Self {
        ModuleInfo {
            name: "console",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    lambda_mode_initializer();

    let globals = ctx.globals();

    // NOTE: Console must be created from an empty object with no prototype.
    // https://console.spec.whatwg.org/#console-namespace
    let console = ctx.eval::<Object, &str>("Object.create({})")?;

    console.set("assert", Func::from(log_assert))?;
    console.set("clear", Func::from(clear))?;
    console.set("debug", Func::from(log_debug))?;
    console.set("error", Func::from(log_error))?;
    console.set("info", Func::from(log))?;
    console.set("log", Func::from(log))?;
    console.set("trace", Func::from(log_trace))?;
    console.set("warn", Func::from(log_warn))?;
    console.prop(
        PredefinedAtom::SymbolToStringTag,
        Property::from("console").configurable(),
    )?;

    globals.prop("console", Property::from(console).writable().configurable())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use llrt_test::test_sync_with;
    use rquickjs::{function::Rest, Error, IntoJs, Null, Object, Undefined, Value};

    use crate::libs::{
        json::stringify::json_stringify_replacer_space,
        logging::LogLevel,
        utils::primordials::{BasePrimordials, Primordial},
    };
    use crate::modules::console::write_lambda_log;

    #[tokio::test]
    async fn json_log_format() {
        test_sync_with(|ctx| {
            BasePrimordials::init(&ctx)?;
            let write_log = |args| {
                let mut result = String::new();

                write_lambda_log(
                    &ctx,
                    &mut result,
                    Rest(args),
                    LogLevel::Info,
                    false,
                    true,
                    LogLevel::Info as usize,
                    "",
                )?;

                //validate json
                ctx.json_parse(result.clone())?;

                Ok::<_, Error>(result)
            };

            assert_eq!(
                write_log(["Hello".into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":"Hello"}"#
            );

            assert_eq!(
                write_log([1.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":1}"#
            );

            assert_eq!(
                write_log([true.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":true}"#
            );

            assert_eq!(
                write_log([Undefined.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO"}"#
            );

            assert_eq!(
                write_log([Null.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":null}"#
            );

            let obj = Object::new(ctx.clone())?;
            obj.set("a", 1)?;
            obj.set("b", "Hello")?;

            assert_eq!(
                write_log([obj.clone().into_value()].into())?,
                r#"{"time":"","level":"INFO","message":{"a":1,"b":"Hello"}}"#
            );

            //validate second argument passed
            assert_eq!(
                write_log([obj.into_value(), true.into_js(&ctx)?].into())?,
                r#"{"time":"","level":"INFO","message":"{\n  a: 1,\n  b: 'Hello'\n} true"}"#
            );

            //single error
            let e1:Value = ctx.eval(r#"new ReferenceError("some reference error")"#)?;
            assert_eq!(
                write_log([e1.clone()].into())?,
                r#"{"time":"","level":"INFO","message":{"errorType":"ReferenceError","errorMessage":"some reference error","stackTrace":["    at <eval> (eval_script:1:4)",""]}}"#
            );

             //validate many args with additional errors
            let e2:Value = ctx.eval(r#"new SyntaxError("some syntax error")"#)?;
            assert_eq!(
                write_log(["errors logged".into_js(&ctx)?, e1, e2].into())?,
                r#"{"time":"","level":"INFO","message":"errors logged ReferenceError: some reference error\n  at <eval> (eval_script:1:4) SyntaxError: some syntax error\n  at <eval> (eval_script:1:4)","errorType":"ReferenceError","errorMessage":"some reference error","stackTrace":["    at <eval> (eval_script:1:4)",""]}"#
            );

            Ok(())
        })
        .await;
    }

    #[tokio::test]
    async fn standard_log_format() {
        test_sync_with(|ctx| {
            BasePrimordials::init(&ctx)?;
            let write_log = |args| {
                let mut result = String::new();

                write_lambda_log(
                    &ctx,
                    &mut result,
                    Rest(args),
                    LogLevel::Info,
                    false,
                    false,
                    LogLevel::Info as usize,
                    "",
                )?;

                Ok::<_, Error>(result)
            };

            assert_eq!(
                write_log(["Hello".into_js(&ctx)?].into())?,
               "\tn/a\tINFO\tHello"
            );

            assert_eq!(
                write_log([1.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\t1"
            );

            assert_eq!(
                write_log([true.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\ttrue"
            );

            assert_eq!(
                write_log([Undefined.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\tundefined"
            );

            assert_eq!(
                write_log([Null.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\tnull"
            );

            let obj = Object::new(ctx.clone())?;
            obj.set("a", 1)?;
            obj.set("b", "Hello")?;

            assert_eq!(
                write_log([obj.clone().into_value()].into())?,
                 "\tn/a\tINFO\t{\r  a: 1,\r  b: 'Hello'\r}"
            );

            //validate second argument passed
            assert_eq!(
                write_log([obj.clone().into_value(), true.into_js(&ctx)?].into())?,
                "\tn/a\tINFO\t{\r  a: 1,\r  b: 'Hello'\r} true"
            );

            //single error
            let e1:Value = ctx.eval(r#"new ReferenceError("some reference error")"#)?;
            assert_eq!(
                write_log([e1.clone()].into())?,
                "\tn/a\tINFO\tReferenceError: some reference error\r  at <eval> (eval_script:1:4)"
            );

             //validate many args with additional errors
            let e2:Value = ctx.eval(r#"new SyntaxError("some syntax error")"#)?;
            assert_eq!(
                write_log(["errors logged".into_js(&ctx)?, e1, e2].into())?,
                "\tn/a\tINFO\terrors logged ReferenceError: some reference error\r  at <eval> (eval_script:1:4) SyntaxError: some syntax error\r  at <eval> (eval_script:1:4)"
            );

            //newline replacement
            assert_eq!(
                write_log([
                    "event:".into_js(&ctx)?,
                    json_stringify_replacer_space(&ctx, obj.into_value(), None, Some("  ".into()))?.into_js(&ctx)?
                ].into())?,
               "\tn/a\tINFO\tevent: {\r  \"a\": 1,\r  \"b\": \"Hello\"\r}"
            );

            Ok(())
        })
        .await;
    }
}
