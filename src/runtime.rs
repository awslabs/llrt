use crate::environment;
use crate::json::parse::json_parse;
use crate::net::{DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT_SECONDS, TLS_CONFIG};
use bytes::Bytes;
use chrono::Utc;
use http_body_util::{BodyExt, Empty};
use hyper::header::{HeaderMap, CONTENT_TYPE};
use hyper::http::header::HeaderName;
use hyper::{Request, StatusCode};
use hyper_util::{
    client::legacy::Client,
    rt::{TokioExecutor, TokioTimer},
};
use rquickjs::IntoJs;
use rquickjs::{prelude::Func, Array, Ctx, Function, Module, Object, Value};

use std::env;
use std::time::Duration;

use tracing::warn;

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
const AWS_REGION: &str = "AWS_REGION";
const RUNTIME_PATH: &str = "2018-06-01/runtime";
const _X_AMZN_TRACE_ID: &str = "_X_AMZN_TRACE_ID";

const TRACE_ID: HeaderName = HeaderName::from_static("lambda-runtime-trace-id");
const DEADLINE_MS: HeaderName = HeaderName::from_static("lambda-runtime-deadline-ms");
const REQUEST_ID: HeaderName = HeaderName::from_static("lambda-runtime-aws-request-id");
const INVOKED_FUNCTION_ARN: HeaderName =
    HeaderName::from_static("lambda-runtime-invoked-function-arn");
const CLIENT_CONTEXT: HeaderName = HeaderName::from_static("lambda-runtime-client-context");
const COGNITO_IDENTITY: HeaderName = HeaderName::from_static("lambda-runtime-cognito-identity");

type HyperClient = Client<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    Empty<Bytes>,
>;

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
    static mut START_TIME: f64 = 0.0;

    let aws_lambda_runtime_api = match get_env(AWS_LAMBDA_RUNTIME_API) {
        Some(env) => env,
        None => {
            let msg = "Environment variable 'AWS_LAMBDA_RUNTIME_API' is not defined";
            return Err(into_exception(msg.to_string(), ctx));
        }
    };

    let (module_name, handler_name): (String, String) =
        get_module_and_handler_name().map_err(|err| into_exception(err, ctx))?;
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
    ctx.globals().set("initTasks", js_init_tasks)?;

    if init_tasks_sze == 1 {
        ctx.eval::<(), _>("await initTasks[0]")?;
    } else if init_tasks_sze > 1 {
        ctx.eval::<(), _>("await Promise.all(initTasks)")?;
    }

    let handler: Value = js_handler_module.get(handler_name.as_str())?;

    if !handler.is_function() {
        panic!(
            "{}",
            format!(
                "\"{}\" is not a function in \"{}\"",
                handler_name, module_name
            )
        );
    }

    let base_url = format!("http://{}/{}", aws_lambda_runtime_api, RUNTIME_PATH);
    let handler = handler.as_function().unwrap();
    start_process_events(handler, base_url.as_str(), ctx).await
}

// handler: (event: any, context: Context) => Promise<any>
async fn next_invocation<'js>(
    client: &HyperClient,
    uri: &str,
    ctx: &Ctx<'js>,
) -> rquickjs::Result<NextInvocationResponse<'js>> {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .body(Empty::new())
        .map_err(|err| into_exception(err.to_string(), ctx))?;

    let res = client
        .request(req)
        .await
        .map_err(|err| into_exception(err.to_string(), ctx))?;

    if res.status() != StatusCode::OK {
        todo!()
    }

    match res.headers().get(TRACE_ID) {
        Some(trace_id_value) => {
            let trace_id_value = String::from_utf8_lossy(trace_id_value.as_bytes());
            env::set_var(_X_AMZN_TRACE_ID, trace_id_value.as_ref());
        }
        None => {
            env::remove_var(_X_AMZN_TRACE_ID);
        }
    };

    let headers = res.headers();

    let deadline_ms = get_value(headers, &DEADLINE_MS)
        .parse::<i64>()
        .map_err(|err| into_exception(err.to_string(), ctx))?;

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
        aws_request_id: get_value(headers, &REQUEST_ID),
        invoked_function_arn: get_value(headers, &INVOKED_FUNCTION_ARN),
        log_group_name: get_env(AWS_LAMBDA_LOG_GROUP_NAME).unwrap(),
        log_stream_name: get_env(AWS_LAMBDA_LOG_STREAM_NAME).unwrap(),
        function_name: get_env(AWS_LAMBDA_FUNCTION_NAME).unwrap(),
        function_version: get_env(AWS_LAMBDA_FUNCTION_VERSION).unwrap(),
        memory_limit_in_mb: get_env(AWS_LAMBDA_FUNCTION_MEMORY_SIZE)
            .unwrap()
            .parse::<usize>()
            .map_err(|err| into_exception(err.to_string(), ctx))?,
        callback_waits_for_empty_event_loop: true,
        get_remaining_time_in_millis,
        client_context,
        cognito_identity_json,
    };
    let bytes = res
        .collect()
        .await
        .map_err(|err| into_exception(err.to_string(), ctx))?
        .to_bytes();
    let event: Value<'js> = json_parse(ctx, bytes.into())?;

    Ok(NextInvocationResponse { event, context })
}

async fn invoke_repsonse<'js>(
    _reult: Value<'js>,
    _lambda_context: LambdaContext<'js>,
) -> Result<(), rquickjs::Error> {
    todo!()
}

async fn start_process_events<'js>(
    handler: &Function<'js>,
    base_url: &str,
    ctx: &Ctx<'js>,
) -> rquickjs::Result<()> {
    let client = get_hyper_client();
    let uri = format!("{base_url}/invocation/next");
    let mut request_id: Option<String> = None;
    loop {
        let NextInvocationResponse { event, context } =
            next_invocation(&client, uri.as_str(), ctx).await?;
        if request_id.is_none() {
            request_id = Some(context.aws_request_id.clone())
        };
        let js_context = convert_into_js_value(ctx.clone(), context.clone())?;
        let result = handler.call::<_, Value>((event, js_context.as_value().clone()))?;
        invoke_repsonse(result, context).await?;
    }
}
fn get_env(env: &str) -> Option<String> {
    env::var(env).ok()
}

fn get_handler_env() -> Option<String> {
    match get_env(LAMBDA_HANDLER) {
        Some(lambda_handler) => Some(lambda_handler),
        None => get_env(_HANDLER),
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
        Some(handler) => {
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
        None => Err("env value LAMBDA_HANDLER is not set".to_string()),
    }
}

fn get_task_root() -> String {
    match get_env(LAMBDA_TASK_ROOT) {
        Some(lambda_task_root) => lambda_task_root,
        None => env::current_dir()
            .unwrap()
            .into_os_string()
            .into_string()
            .unwrap(),
    }
}

fn get_hyper_client() -> HyperClient {
    let pool_idle_timeout = get_pool_idle_timeout();
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(TLS_CONFIG.clone())
        .https_or_http()
        .enable_http1()
        .build();

    let client = Client::builder(TokioExecutor::new())
        .pool_idle_timeout(Duration::from_secs(pool_idle_timeout))
        .pool_timer(TokioTimer::new())
        .build(https);
    client
}

fn get_pool_idle_timeout() -> u64 {
    let pool_idle_timeout: u64 = env::var(environment::ENV_LLRT_NET_POOL_IDLE_TIMEOUT)
        .map(|timeout| {
            timeout
                .parse()
                .unwrap_or(DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT_SECONDS)
        })
        .unwrap_or(DEFAULT_CONNECTION_POOL_IDLE_TIMEOUT_SECONDS);
    if pool_idle_timeout > 300 {
        warn!(
            r#""{}" is exceeds 300s (5min), risking errors due to possible server connection closures."#,
            environment::ENV_LLRT_NET_POOL_IDLE_TIMEOUT
        )
    }
    pool_idle_timeout
}

fn get_value(headers: &HeaderMap, header: &HeaderName) -> String {
    headers.get(header).unwrap().to_str().unwrap().to_string()
}

fn into_exception(message: String, ctx: &Ctx<'_>) -> rquickjs::Error {
    rquickjs::Exception::throw_message(ctx, message.as_str())
}
