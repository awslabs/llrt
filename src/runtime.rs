// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::http::fetch::get_pool_idle_timeout;
use crate::json::parse::json_parse;
use crate::json::stringify::{self, json_stringify};
use crate::net::TLS_CONFIG;
use crate::utils::result::ResultExt;
use crate::vm::{ErrorDetails, Vm};
use bytes::Bytes;
use chrono::Utc;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Body;
use hyper::header::{HeaderMap, CONTENT_TYPE};
use hyper::http::header::HeaderName;
use hyper::{Request, StatusCode};
use hyper_util::{
    client::legacy::Client,
    rt::{TokioExecutor, TokioTimer},
};
use rquickjs::atom::PredefinedAtom;
use rquickjs::promise::Promise;
use rquickjs::{prelude::Func, Array, CatchResultExt, Ctx, Function, Module, Object, Value};
use rquickjs::{CaughtError, IntoJs, ThrowResultExt};
use zstd::zstd_safe::WriteBuf;

use std::env;
use std::time::{Duration, Instant};

const AWS_LAMBDA_FUNCTION_NAME: &str = "AWS_LAMBDA_FUNCTION_NAME";
const AWS_LAMBDA_FUNCTION_VERSION: &str = "AWS_LAMBDA_FUNCTION_VERSION";
const AWS_LAMBDA_FUNCTION_MEMORY_SIZE: &str = "AWS_LAMBDA_FUNCTION_MEMORY_SIZE";
const AWS_LAMBDA_LOG_GROUP_NAME: &str = "AWS_LAMBDA_LOG_GROUP_NAME";
const AWS_LAMBDA_LOG_STREAM_NAME: &str = "AWS_LAMBDA_LOG_STREAM_NAME";
const LAMBDA_TASK_ROOT: &str = "LAMBDA_TASK_ROOT";
const _HANDLER: &str = "_HANDLER";
const LAMBDA_HANDLER: &str = "LAMBDA_HANDLER";
const AWS_LAMBDA_RUNTIME_API: &str = "AWS_LAMBDA_RUNTIME_API";
const _EXIT_ITERATIONS: &str = "_EXIT_ITERATIONS";
const _AWS_REGION: &str = "AWS_REGION";
const RUNTIME_PATH: &str = "2018-06-01/runtime";
const _X_AMZN_TRACE_ID: &str = "_X_AMZN_TRACE_ID";

static TRACE_ID: HeaderName = HeaderName::from_static("lambda-runtime-trace-id");
static DEADLINE_MS: HeaderName = HeaderName::from_static("lambda-runtime-deadline-ms");
static REQUEST_ID: HeaderName = HeaderName::from_static("lambda-runtime-aws-request-id");
static INVOKED_FUNCTION_ARN: HeaderName =
    HeaderName::from_static("lambda-runtime-invoked-function-arn");
static CLIENT_CONTEXT: HeaderName = HeaderName::from_static("lambda-runtime-client-context");
static COGNITO_IDENTITY: HeaderName = HeaderName::from_static("lambda-runtime-cognito-identity");

type HyperClient<T> =
    Client<hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, T>;

#[derive(Clone)]
struct LambdaContext<'js> {
    pub aws_request_id: String,
    pub invoked_function_arn: String,
    pub log_group_name: String,
    pub log_stream_name: String,
    pub function_name: String,
    pub function_version: String,
    pub callback_waits_for_empty_event_loop: bool,
    pub memory_limit_in_mb: usize,
    pub get_remaining_time_in_millis: Function<'js>,
    pub client_context: Value<'js>,
    pub cognito_identity_json: Value<'js>,
}

struct NextInvocationResponse<'js> {
    event: Value<'js>,
    context: LambdaContext<'js>,
}

pub async fn runtime(ctx: &Ctx<'_>) -> Result<(), rquickjs::Error> {
    let aws_lambda_runtime_api = get_env(AWS_LAMBDA_RUNTIME_API).or_throw(ctx)?;

    let (module_name, handler_name): (String, String) =
        get_module_and_handler_name().or_throw(ctx)?;
    let task_root = get_task_root();

    let js_handler_module: Object = Module::import(ctx, format!("{}/{}", task_root, module_name))?;
    let js_init = js_handler_module.get::<_, Value>("init")?;
    let js_bootstrap: Object = ctx.globals().get("__bootstrap")?;
    let js_init_tasks: Array = js_bootstrap.get("initTasks")?;

    if js_init.is_function() {
        let idx = js_init_tasks.len();
        let js_call: Object = js_init.as_function().unwrap().call(())?;
        js_init_tasks.set(idx, js_call)?;
    }

    let init_tasks_sze = js_init_tasks.len();
    #[allow(clippy::comparison_chain)]
    if init_tasks_sze == 1 {
        let init_promise = js_init_tasks.get::<Promise<()>>(0)?;
        init_promise.await.catch(ctx).throw(ctx)?;
    } else if init_tasks_sze > 1 {
        let promise_actor: Object = ctx.globals().get(PredefinedAtom::Promise)?;
        let init_promise: Promise<()> = promise_actor
            .get::<_, Function>("all")?
            .call((js_init_tasks.clone(),))?;
        init_promise.await.catch(ctx).throw(ctx)?;
    }

    let handler: Value = js_handler_module.get(handler_name.as_str())?;

    if !handler.is_function() {
        let msg = format!(
            "\"{}\" is not a function in \"{}\"",
            handler_name, module_name
        );
        return Err(msg).or_throw(ctx)?;
    }

    let base_url = format!("http://{}/{}", aws_lambda_runtime_api, RUNTIME_PATH);
    let handler = handler.as_function().unwrap();
    if let Err(err) = start_process_events(handler, base_url.as_str(), ctx)
        .await
        .map_err(|e| CaughtError::from_error(ctx, e))
    {
        let client_full_body = get_hyper_client::<Full<Bytes>>();
        let err_uri = format!("{}/init/error", base_url,);
        post_error(&err_uri, &err, None, ctx, &client_full_body).await?;
        Vm::print_error_and_exit(ctx, err);
    }
    Ok(())
}

async fn next_invocation<'js>(
    client: &HyperClient<Empty<Bytes>>,
    uri: &str,
    ctx: &Ctx<'js>,
) -> rquickjs::Result<NextInvocationResponse<'js>> {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .body(Empty::new())
        .or_throw(ctx)?;

    let res = client.request(req).await.or_throw(ctx)?;

    if res.status() != StatusCode::OK {
        todo!()
    }

    match res.headers().get(&TRACE_ID) {
        Some(trace_id_value) => {
            let trace_id_value = String::from_utf8_lossy(trace_id_value.as_bytes());
            env::set_var(_X_AMZN_TRACE_ID, trace_id_value.as_ref());
        }
        None => {
            env::remove_var(_X_AMZN_TRACE_ID);
        }
    };

    let headers = res.headers();

    let deadline_ms = get_header_value(headers, &DEADLINE_MS)
        .or_throw(ctx)?
        .parse::<i64>()
        .or_throw(ctx)?;

    let get_remaining_time_in_millis = Func::from(move || {
        let now = Utc::now();
        deadline_ms - now.timestamp_millis()
    });
    let get_remaining_time_in_millis = get_remaining_time_in_millis
        .into_js(ctx)?
        .into_function()
        .unwrap();

    let client_context = match headers.get(&CLIENT_CONTEXT) {
        Some(json) => json_parse(ctx, json.as_bytes().into()),
        None => rquickjs::Undefined.into_js(ctx),
    }?;
    let cognito_identity_json = match headers.get(&COGNITO_IDENTITY) {
        Some(json) => json_parse(ctx, json.as_bytes().into()),
        None => rquickjs::Undefined.into_js(ctx),
    }?;
    let context = LambdaContext {
        aws_request_id: get_header_value(headers, &REQUEST_ID).or_throw(ctx)?,
        invoked_function_arn: get_header_value(headers, &INVOKED_FUNCTION_ARN).or_throw(ctx)?,
        log_group_name: get_env(AWS_LAMBDA_LOG_GROUP_NAME).or_throw(ctx)?,
        log_stream_name: get_env(AWS_LAMBDA_LOG_STREAM_NAME).or_throw(ctx)?,
        function_name: get_env(AWS_LAMBDA_FUNCTION_NAME).or_throw(ctx)?,
        function_version: get_env(AWS_LAMBDA_FUNCTION_VERSION).or_throw(ctx)?,
        memory_limit_in_mb: get_env(AWS_LAMBDA_FUNCTION_MEMORY_SIZE)
            .unwrap_or_else(|_| String::from("128"))
            .parse::<usize>()
            .or_throw(ctx)?,
        callback_waits_for_empty_event_loop: true,
        get_remaining_time_in_millis,
        client_context,
        cognito_identity_json,
    };
    let bytes = res.collect().await.or_throw(ctx)?.to_bytes();
    let event: Value<'js> = json_parse(ctx, bytes.into())?;

    Ok(NextInvocationResponse { event, context })
}

async fn invoke_response<'js>(
    base_url: &str,
    ctx: &Ctx<'js>,
    result: Value<'js>,
    lambda_context: &LambdaContext<'js>,
    client: &HyperClient<Full<Bytes>>,
) -> Result<(), rquickjs::Error> {
    let result_string = stringify::json_stringify(
        ctx,
        if result.is_undefined() {
            Value::new_null(ctx.clone())
        } else {
            result
        },
    )?
    .unwrap_or(String::from(""));
    let req = Request::builder()
        .method("POST")
        .uri(format!(
            "{}/invocation/{}/response",
            base_url, lambda_context.aws_request_id
        ))
        .header(CONTENT_TYPE, "application/json")
        .body(Full::from(bytes::Bytes::from(result_string)))
        .or_throw(ctx)?;

    let res = client.request(req).await.or_throw(ctx)?;
    match res.status() {
        StatusCode::ACCEPTED => Ok(()),
        _ => {
            let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
            let res_str = String::from_utf8_lossy(res_bytes.as_slice());
            Err(format!(
                "Unexpected /invocation/response response: {}",
                res_str
            ))
            .or_throw(ctx)?
        }
    }
}

// handler: (event: any, context: Context) => Promise<any>
async fn start_process_events<'js>(
    handler: &Function<'js>,
    base_url: &str,
    ctx: &Ctx<'js>,
) -> rquickjs::Result<()> {
    let exit_iterations = match get_env(_EXIT_ITERATIONS) {
        Ok(iterations) => iterations.parse::<i64>().unwrap_or(-1),
        Err(_) => -1,
    };
    let mut iterations = 0;

    let client_empty_body = get_hyper_client::<Empty<Bytes>>();
    let client_full_body = get_hyper_client::<Full<Bytes>>();

    let mut request_id: Option<String> = None;
    let mut context: Option<LambdaContext> = None;
    let mut event: Option<Value> = None;
    loop {
        let now = Instant::now();

        if let Err(err) = process_event(
            ctx,
            base_url,
            handler,
            &mut context,
            &mut event,
            &client_empty_body,
            &client_full_body,
            &mut request_id,
        )
        .await
        .map_err(|e| CaughtError::from_error(ctx, e))
        {
            match context {
                None => Vm::print_error_and_exit(ctx, err),
                Some(ref context) => {
                    let error_uri =
                        format!("{}/invocation/{}/error", base_url, context.aws_request_id);
                    if let Err(err) = post_error(
                        &error_uri,
                        &err,
                        request_id.as_ref(),
                        ctx,
                        &client_full_body,
                    )
                    .await
                    {
                        Vm::print_error_and_exit(ctx, CaughtError::from_error(ctx, err))
                    }
                }
            }
        }
        if exit_iterations > -1 {
            if iterations >= exit_iterations - 1 {
                println!("Done in {} ms", now.elapsed().as_millis());
                break Ok(());
            }
            iterations += 1;
        }
        context = None;
        event = None;
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_event<'js>(
    ctx: &Ctx<'js>,
    base_url: &str,
    handler: &Function<'js>,
    context_lambda: &mut Option<LambdaContext<'js>>,
    event_: &mut Option<Value<'js>>,
    client_empty_body: &HyperClient<Empty<Bytes>>,
    client_full_body: &HyperClient<Full<Bytes>>,
    request_id: &mut Option<String>,
) -> rquickjs::Result<()> {
    let next_invocation_uri = format!("{base_url}/invocation/next");
    let NextInvocationResponse { event, context } =
        next_invocation(client_empty_body, next_invocation_uri.as_str(), ctx).await?;
    if request_id.is_none() {
        *request_id = Some(context.aws_request_id.clone())
    };
    let js_context = convert_into_js_value(ctx.clone(), context.clone())?;

    let js_bootstrap = ctx.globals().get::<_, Object>("__bootstrap")?;
    let js_set_request_id = js_bootstrap.get::<_, Function>("setRequestId")?;
    let _ = js_set_request_id.call::<_, ()>(());

    let promise =
        handler.call::<_, Promise<Value>>((event.clone(), js_context.as_value().clone()))?;
    let result: Value = promise.await.catch(ctx).throw(ctx)?;
    invoke_response(base_url, ctx, result, &context, client_full_body).await?;
    *context_lambda = Some(context);
    *event_ = Some(event);
    Ok(())
}

async fn post_error<'js>(
    path: &str,
    error: &CaughtError<'js>,
    request_id: Option<&String>,
    ctx: &Ctx<'js>,
    client: &HyperClient<Full<Bytes>>,
) -> Result<(), rquickjs::Error> {
    let ErrorDetails { msg, r#type, stack } = Vm::error_details(ctx, error);

    let obj = Object::new(ctx.clone())?;
    obj.prop("errorType", r#type.clone())?;
    obj.prop("errorMessage", msg)?;
    obj.prop("stackTrace", stack)?;
    obj.prop("requestId", request_id.cloned().unwrap_or_default())?;
    obj.prop("cause", String::default())?;
    let error_body = json_stringify(ctx, obj.as_value().clone())?.unwrap_or_default();

    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header(CONTENT_TYPE, "application/json")
        .header("Lambda-Runtime-Function-Error-Type", r#type)
        .body(Full::from(bytes::Bytes::from(error_body)))
        .or_throw(ctx)?;
    let res = client.request(req).await.or_throw(ctx)?;
    match res.status() {
        StatusCode::ACCEPTED => Ok(()),
        _ => {
            let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
            let res_str = String::from_utf8_lossy(res_bytes.as_slice());
            Err(format!("Unexpected /{} response: {}", path, res_str)).or_throw(ctx)?
        }
    }
}

fn get_env(env: &str) -> Result<String, String> {
    match env::var(env).ok() {
        Some(env) => Ok(env),
        None => Err(format!("Environment variable {} is not defined.", env)),
    }
}

fn get_handler_env() -> Result<String, String> {
    match get_env(LAMBDA_HANDLER) {
        Ok(lambda_handler) => Ok(lambda_handler),
        Err(_) => match get_env(_HANDLER) {
            Ok(handler) => Ok(handler),
            Err(e) => Err(e),
        },
    }
}

fn convert_into_js_value<'js>(
    ctx: Ctx<'js>,
    lambda_context: LambdaContext<'js>,
) -> rquickjs::Result<Object<'js>> {
    let obj = Object::new(ctx)?;
    obj.prop("awsRequestId", lambda_context.aws_request_id)?;
    obj.prop("invokedFunctionArn", lambda_context.invoked_function_arn)?;
    obj.prop("logGroupName", lambda_context.log_group_name)?;
    obj.prop("logStreamName", lambda_context.log_stream_name)?;
    obj.prop("functionName", lambda_context.function_name)?;
    obj.prop("functionVersion", lambda_context.function_version)?;
    obj.prop("memoryLimitInMB", lambda_context.memory_limit_in_mb)?;
    obj.prop(
        "callbackWaitsForEmptyEventLoop",
        lambda_context.callback_waits_for_empty_event_loop,
    )?;
    obj.prop(
        "getRemainingTimeInMillis",
        lambda_context.get_remaining_time_in_millis,
    )?;
    obj.prop("clientContext", lambda_context.client_context)?;
    obj.prop("cognitoIdentityJson", lambda_context.cognito_identity_json)?;
    Ok(obj)
}

fn get_module_and_handler_name() -> Result<(String, String), String> {
    match get_handler_env() {
        Ok(handler) => {
            let split: Vec<Option<String>> = handler
                .split('.')
                .map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(String::from(s))
                    }
                })
                .collect();
            match split.len() {
                2 => match (&split[0], &split[1]) {
                    (Some(module_name), Some(handler_name)) => {
                        Ok((module_name.into(), handler_name.into()))
                    }
                    _ => {
                        Err(format!("Invalid handler name or LAMBDA_HANDLER env value: \"{}\": Should be in format {{filename}}.{{method_name}}", handler))
                    }
                },
                _ => {
                    Err(format!("Invalid handler name or LAMBDA_HANDLER env value: \"{}\": Should be in format {{filename}}.{{method_name}}", handler))
                }
            }
        }
        Err(e) => Err(e),
    }
}

fn get_task_root() -> String {
    match get_env(LAMBDA_TASK_ROOT) {
        Ok(lambda_task_root) => lambda_task_root,
        Err(_) => env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap(),
    }
}

fn get_hyper_client<T>() -> HyperClient<T>
where
    T: Body + Send + 'static + Unpin,
    T::Data: Send,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let pool_idle_timeout = get_pool_idle_timeout();
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(TLS_CONFIG.clone())
        .https_or_http()
        .enable_http1()
        .build();

    Client::builder(TokioExecutor::new())
        .pool_idle_timeout(Duration::from_secs(pool_idle_timeout))
        .pool_timer(TokioTimer::new())
        .build(https)
}

fn get_header_value(headers: &HeaderMap, header: &HeaderName) -> Result<String, String> {
    match headers.get(header) {
        Some(header) => Ok(header.to_str().map_err(|e| e.to_string())?.to_string()),
        None => Err(format!("The header {} is not valid.", header)),
    }
}
