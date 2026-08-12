// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{convert::Coerced, function::Opt, Ctx, Exception, Result, Value};

fn ansi_codes(format: &str) -> Option<(&'static str, &'static str)> {
    Some(match format {
        "reset" => ("\x1b[0m", "\x1b[0m"),
        "bold" => ("\x1b[1m", "\x1b[22m"),
        "dim" => ("\x1b[2m", "\x1b[22m"),
        "italic" => ("\x1b[3m", "\x1b[23m"),
        "underline" => ("\x1b[4m", "\x1b[24m"),
        "blink" => ("\x1b[5m", "\x1b[25m"),
        "inverse" => ("\x1b[7m", "\x1b[27m"),
        "hidden" => ("\x1b[8m", "\x1b[28m"),
        "strikethrough" => ("\x1b[9m", "\x1b[29m"),
        "doubleunderline" => ("\x1b[21m", "\x1b[24m"),
        "black" => ("\x1b[30m", "\x1b[39m"),
        "red" => ("\x1b[31m", "\x1b[39m"),
        "green" => ("\x1b[32m", "\x1b[39m"),
        "yellow" => ("\x1b[33m", "\x1b[39m"),
        "blue" => ("\x1b[34m", "\x1b[39m"),
        "magenta" => ("\x1b[35m", "\x1b[39m"),
        "cyan" => ("\x1b[36m", "\x1b[39m"),
        "white" => ("\x1b[37m", "\x1b[39m"),
        "gray" => ("\x1b[90m", "\x1b[39m"),
        "bgBlack" => ("\x1b[40m", "\x1b[49m"),
        "bgRed" => ("\x1b[41m", "\x1b[49m"),
        "bgGreen" => ("\x1b[42m", "\x1b[49m"),
        "bgYellow" => ("\x1b[43m", "\x1b[49m"),
        "bgBlue" => ("\x1b[44m", "\x1b[49m"),
        "bgMagenta" => ("\x1b[45m", "\x1b[49m"),
        "bgCyan" => ("\x1b[46m", "\x1b[49m"),
        "bgWhite" => ("\x1b[47m", "\x1b[49m"),
        "bgGray" => ("\x1b[100m", "\x1b[49m"),
        "redBright" => ("\x1b[91m", "\x1b[39m"),
        "greenBright" => ("\x1b[92m", "\x1b[39m"),
        "yellowBright" => ("\x1b[93m", "\x1b[39m"),
        "blueBright" => ("\x1b[94m", "\x1b[39m"),
        "magentaBright" => ("\x1b[95m", "\x1b[39m"),
        "cyanBright" => ("\x1b[96m", "\x1b[39m"),
        "whiteBright" => ("\x1b[97m", "\x1b[39m"),
        "bgRedBright" => ("\x1b[101m", "\x1b[49m"),
        "bgGreenBright" => ("\x1b[102m", "\x1b[49m"),
        "bgYellowBright" => ("\x1b[103m", "\x1b[49m"),
        "bgBlueBright" => ("\x1b[104m", "\x1b[49m"),
        "bgMagentaBright" => ("\x1b[105m", "\x1b[49m"),
        "bgCyanBright" => ("\x1b[106m", "\x1b[49m"),
        "bgWhiteBright" => ("\x1b[107m", "\x1b[49m"),
        "framed" => ("\x1b[51m", "\x1b[54m"),
        "overlined" => ("\x1b[53m", "\x1b[55m"),
        _ => return None,
    })
}

fn format_codes<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> Result<(&'static str, &'static str)> {
    let s = value
        .as_string()
        .ok_or_else(|| invalid_format_type(ctx))?
        .to_string()?;
    ansi_codes(&s).ok_or_else(|| invalid_format_value(ctx, &s))
}

fn invalid_format_type(ctx: &Ctx<'_>) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        "The argument 'format' must be of type string or an array of strings",
    )
}

fn invalid_format_value(ctx: &Ctx<'_>, received: &str) -> rquickjs::Error {
    Exception::throw_type(
        ctx,
        &format!(
            "The argument 'format' must be a valid style. Received '{}'",
            received
        ),
    )
}

pub fn style_text<'js>(
    ctx: Ctx<'js>,
    format: Value<'js>,
    text: Coerced<String>,
    _options: Opt<Value<'js>>,
) -> Result<String> {
    if let Some(arr) = format.as_array() {
        let mut closes = Vec::with_capacity(arr.len());
        let mut result = String::with_capacity(text.0.len() + arr.len() * 10);
        for item in arr.iter::<Value>() {
            let (open, close) = format_codes(&ctx, &item?)?;
            result.push_str(open);
            closes.push(close);
        }
        result.push_str(&text.0);
        for close in closes.into_iter().rev() {
            result.push_str(close);
        }
        Ok(result)
    } else {
        let (open, close) = format_codes(&ctx, &format)?;
        let mut result = String::with_capacity(text.0.len() + open.len() + close.len());
        result.push_str(open);
        result.push_str(&text.0);
        result.push_str(close);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use crate::UtilModule;

    #[tokio::test]
    async fn single_format() {
        test_async_with(|ctx| {
            Box::pin(async move {
                llrt_stream_web::init(&ctx).unwrap();
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<UtilModule>(ctx.clone(), "util")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { styleText } from 'util';

                        export async function test() {
                            return styleText("red", "hi")
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "\x1b[31mhi\x1b[39m");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn array_of_formats_nests() {
        test_async_with(|ctx| {
            Box::pin(async move {
                llrt_stream_web::init(&ctx).unwrap();
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<UtilModule>(ctx.clone(), "util")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { styleText } from 'util';

                        export async function test() {
                            return styleText(["bold", "red"], "hi")
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "\x1b[1m\x1b[31mhi\x1b[39m\x1b[22m");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn unknown_format_throws() {
        test_async_with(|ctx| {
            Box::pin(async move {
                llrt_stream_web::init(&ctx).unwrap();
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<UtilModule>(ctx.clone(), "util")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { styleText } from 'util';

                        export async function test() {
                            try {
                                styleText("notacolor", "hi");
                                return "no-throw";
                            } catch (e) {
                                return e instanceof TypeError ? "threw" : "wrong-error-type";
                            }
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "threw");
            })
        })
        .await;
    }
}
