// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::json::parse::json_parse;
use crate::json::stringify::{self, json_stringify};
use crate::modules::console;
use crate::modules::net::HTTP_CLIENT;
use crate::utils::class::get_class_name;
use crate::utils::result::ResultExt;
use crate::vm::Vm;
use bytes::Bytes;
use chrono::Utc;
use http_body_util::{BodyExt, Full};
use hyper::{
    header::{HeaderMap, CONTENT_TYPE},
    http::header::HeaderName,
    Request, StatusCode,
};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use once_cell::sync::Lazy;

use rquickjs::function::{Rest, This};
use rquickjs::Exception;
use rquickjs::{
    atom::PredefinedAtom, prelude::Func, promise::Promise, Array, CaughtError, Ctx, Function,
    IntoJs, Object, Result, Value,
};

use tracing::info;
use zstd::zstd_safe::WriteBuf;

use std::sync::RwLock;
use std::{env, result::Result as StdResult, time::Instant};

const ENV_AWS_LAMBDA_FUNCTION_NAME: &str = "AWS_LAMBDA_FUNCTION_NAME";
const ENV_AWS_LAMBDA_FUNCTION_VERSION: &str = "AWS_LAMBDA_FUNCTION_VERSION";
const ENV_AWS_LAMBDA_FUNCTION_MEMORY_SIZE: &str = "AWS_LAMBDA_FUNCTION_MEMORY_SIZE";
const ENV_AWS_LAMBDA_LOG_GROUP_NAME: &str = "AWS_LAMBDA_LOG_GROUP_NAME";
const ENV_AWS_LAMBDA_LOG_STREAM_NAME: &str = "AWS_LAMBDA_LOG_STREAM_NAME";
const ENV_LAMBDA_TASK_ROOT: &str = "LAMBDA_TASK_ROOT";
const ENV_UNDERSCORE_HANDLER: &str = "_HANDLER";
const ENV_LAMBDA_HANDLER: &str = "LAMBDA_HANDLER";
const AWS_LAMBDA_RUNTIME_API: &str = "AWS_LAMBDA_RUNTIME_API";
const ENV_UNDERSCORE_EXIT_ITERATIONS: &str = "_EXIT_ITERATIONS";
const ENV_RUNTIME_PATH: &str = "2018-06-01/runtime";
const ENV_X_AMZN_TRACE_ID: &str = "_X_AMZN_TRACE_ID";

static HEADER_TRACE_ID: HeaderName = HeaderName::from_static("lambda-runtime-trace-id");
static HEADER_DEADLINE_MS: HeaderName = HeaderName::from_static("lambda-runtime-deadline-ms");
static HEADER_REQUEST_ID: HeaderName = HeaderName::from_static("lambda-runtime-aws-request-id");
static HEADER_ERROR_TYPE: HeaderName =
    HeaderName::from_static("lambda-runtime-function-error-type");
static HEADER_INVOKED_FUNCTION_ARN: HeaderName =
    HeaderName::from_static("lambda-runtime-invoked-function-arn");
static HEADER_CLIENT_CONTEXT: HeaderName = HeaderName::from_static("lambda-runtime-client-context");
static HEADER_COGNITO_IDENTITY: HeaderName =
    HeaderName::from_static("lambda-runtime-cognito-identity");

pub static LAMBDA_REQUEST_ID: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

type HyperClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

#[derive(Clone)]
struct LambdaContext<'js, 'a> {
    pub aws_request_id: String,
    pub invoked_function_arn: String,
    pub callback_waits_for_empty_event_loop: bool,
    pub get_remaining_time_in_millis: Function<'js>,
    pub client_context: Value<'js>,
    pub cognito_identity_json: Value<'js>,
    pub lambda_environment: &'a LambdaEnvironment,
}

impl<'js, 'a> IntoJs<'js> for LambdaContext<'js, 'a> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let obj = Object::new(ctx.clone())?;
        obj.set("awsRequestId", self.aws_request_id)?;
        obj.set("invokedFunctionArn", self.invoked_function_arn)?;
        obj.set("logGroupName", &self.lambda_environment.log_group_name)?;
        obj.set("logStreamName", &self.lambda_environment.log_stream_name)?;
        obj.set("functionName", &self.lambda_environment.function_name)?;
        obj.set("functionVersion", &self.lambda_environment.function_version)?;
        obj.set(
            "memoryLimitInMB",
            self.lambda_environment.memory_limit_in_mb,
        )?;
        obj.set(
            "callbackWaitsForEmptyEventLoop",
            self.callback_waits_for_empty_event_loop,
        )?;
        obj.set(
            "getRemainingTimeInMillis",
            self.get_remaining_time_in_millis,
        )?;
        obj.set("clientContext", self.client_context)?;
        obj.set("cognitoIdentityJson", self.cognito_identity_json)?;
        Ok(obj.into_value())
    }
}

#[derive(Clone)]
struct LambdaEnvironment {
    pub log_group_name: String,
    pub log_stream_name: String,
    pub function_name: String,
    pub function_version: String,
    pub memory_limit_in_mb: usize,
}

impl LambdaEnvironment {
    fn new() -> Self {
        Self {
            log_group_name: env::var(ENV_AWS_LAMBDA_LOG_GROUP_NAME).unwrap_or_default(),
            log_stream_name: env::var(ENV_AWS_LAMBDA_LOG_STREAM_NAME).unwrap_or_default(),
            function_name: env::var(ENV_AWS_LAMBDA_FUNCTION_NAME).unwrap_or_default(),
            function_version: env::var(ENV_AWS_LAMBDA_FUNCTION_VERSION).unwrap_or_default(),
            memory_limit_in_mb: env::var(ENV_AWS_LAMBDA_FUNCTION_MEMORY_SIZE)
                .unwrap_or("128".into())
                .parse()
                .unwrap_or_default(),
        }
    }
}

struct NextInvocationResponse<'js, 'a> {
    event: Value<'js>,
    context: LambdaContext<'js, 'a>,
}

struct RuntimeConfig {
    runtime_api: String,
    handler: String,
    iterations: usize,
}

impl RuntimeConfig {
    fn default(ctx: &Ctx) -> Result<Self> {
        Ok(Self {
            runtime_api: env::var(AWS_LAMBDA_RUNTIME_API).map_err(|_| {
                Exception::throw_message(
                    ctx,
                    concat!(
                        "Environment variable ",
                        stringify!(AWS_LAMBDA_RUNTIME_API),
                        " is not defined.",
                    ),
                )
            })?,
            handler: env::var(ENV_LAMBDA_HANDLER)
                .or_else(|_| env::var(ENV_UNDERSCORE_HANDLER))
                .map_err(|_| {
                    Exception::throw_message(
                        ctx,
                        concat!(
                            "Environment variable ",
                            stringify!(ENV_LAMBDA_HANDLER),
                            " or ",
                            stringify!(ENV_UNDERSCORE_HANDLER),
                            " is not defined.",
                        ),
                    )
                })?,
            iterations: env::var(ENV_UNDERSCORE_EXIT_ITERATIONS)
                .ok()
                .and_then(|i| i.parse().ok())
                .unwrap_or_default(),
        })
    }
}

pub async fn start(ctx: &Ctx<'_>) -> Result<()> {
    start_with_cfg(ctx, RuntimeConfig::default(ctx)?).await
}

async fn start_with_cfg(ctx: &Ctx<'_>, config: RuntimeConfig) -> Result<()> {
    let (module_name, handler_name) = get_module_and_handler_name(ctx, &config.handler)?;
    let task_root = get_task_root();

    //allows CJS handlers
    let require_function: Function = ctx.globals().get("require")?;
    let require_specifier: String = [task_root.as_str(), module_name].join("/");
    let js_handler_module: Object = require_function.call((require_specifier,))?;
    let js_init = js_handler_module.get::<_, Value>("init")?;
    let js_bootstrap: Object = ctx.globals().get("__bootstrap")?;
    let js_init_tasks: Array = js_bootstrap.get("initTasks")?;

    if js_init.is_function() {
        let idx = js_init_tasks.len();
        let js_call: Object = js_init.as_function().unwrap().call(())?;
        js_init_tasks.set(idx, js_call)?;
    }

    let init_tasks_size = js_init_tasks.len();
    #[allow(clippy::comparison_chain)]
    if init_tasks_size == 1 {
        let init_promise = js_init_tasks.get::<Promise>(0)?;
        init_promise.into_future::<()>().await?;
    } else if init_tasks_size > 1 {
        let promise_ctor: Object = ctx.globals().get(PredefinedAtom::Promise)?;
        let init_promise: Promise = promise_ctor
            .get::<_, Function>("all")?
            .call((This(promise_ctor), js_init_tasks.clone()))?;
        () = init_promise.into_future().await?;
    }

    let handler: Value = js_handler_module.get(handler_name)?;

    if !handler.is_function() {
        return Err(Exception::throw_message(
            ctx,
            &[
                "\"",
                handler_name,
                "\" is not a function in \"",
                module_name,
                "\"",
            ]
            .concat(),
        ));
    }

    let client = HTTP_CLIENT.as_ref().or_throw(ctx)?.clone();

    let base_url = ["http://", &config.runtime_api, "/", ENV_RUNTIME_PATH].concat();
    let handler = handler.as_function().unwrap();
    if let Err(err) = start_process_events(ctx, &client, handler, base_url.as_str(), &config)
        .await
        .map_err(|e| CaughtError::from_error(ctx, e))
    {
        post_error(ctx, &client, &base_url, "/init/error", &err, None).await?;
        Vm::print_error_and_exit(ctx, err);
    }
    Ok(())
}

async fn next_invocation<'js, 'a>(
    ctx: &Ctx<'js>,
    client: &'a HyperClient,
    uri: &str,
    lambda_environment: &'a LambdaEnvironment,
) -> Result<NextInvocationResponse<'js, 'a>> {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .body(Full::default())
        .or_throw(ctx)?;

    let res = client.request(req).await.or_throw(ctx)?;

    if res.status() != StatusCode::OK {
        let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
        let res_str = String::from_utf8_lossy(res_bytes.as_slice());
        return Err(Exception::throw_message(
            ctx,
            &["Unexpected /invocation/next response: ", &res_str].concat(),
        ));
    }

    let headers = res.headers();

    if let Some(trace_id_value) = headers.get(&HEADER_TRACE_ID) {
        let trace_id_value = String::from_utf8_lossy(trace_id_value.as_bytes());
        env::set_var(ENV_X_AMZN_TRACE_ID, trace_id_value.as_ref());
    } else {
        env::remove_var(ENV_X_AMZN_TRACE_ID);
    };

    let deadline_ms = get_header_value(headers, &HEADER_DEADLINE_MS)
        .unwrap_or("0".into())
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

    let client_context = if let Some(json) = headers.get(&HEADER_CLIENT_CONTEXT) {
        json_parse(ctx, json.as_bytes())
    } else {
        rquickjs::Undefined.into_js(ctx)
    }?;
    let cognito_identity_json = if let Some(json) = headers.get(&HEADER_COGNITO_IDENTITY) {
        json_parse(ctx, json.as_bytes())
    } else {
        rquickjs::Undefined.into_js(ctx)
    }?;
    let context = LambdaContext {
        aws_request_id: get_header_value(headers, &HEADER_REQUEST_ID).or_throw(ctx)?,
        invoked_function_arn: get_header_value(headers, &HEADER_INVOKED_FUNCTION_ARN)
            .unwrap_or("n/a".into()),
        callback_waits_for_empty_event_loop: true,
        get_remaining_time_in_millis,
        client_context,
        cognito_identity_json,
        lambda_environment,
    };
    let bytes = res.collect().await.or_throw(ctx)?.to_bytes();
    let event: Value<'js> = json_parse(ctx, bytes)?;

    Ok(NextInvocationResponse { event, context })
}

async fn invoke_response<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    base_url: &str,
    request_id: &str,
    result: Value<'js>,
) -> Result<()> {
    let result_json = stringify::json_stringify(ctx, result)?;
    let req = Request::builder()
        .method("POST")
        .uri([base_url, "/invocation/", request_id, "/response"].concat())
        .header(CONTENT_TYPE, "application/json")
        .body(Full::from(bytes::Bytes::from(
            result_json.unwrap_or_default(),
        )))
        .or_throw(ctx)?;

    let res = client.request(req).await.or_throw(ctx)?;
    match res.status() {
        StatusCode::ACCEPTED => Ok(()),
        _ => {
            let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
            let res_str = String::from_utf8_lossy(res_bytes.as_slice());
            Err(Exception::throw_message(
                ctx,
                &["Unexpected /invocation/response response: ", &res_str].concat(),
            ))
        },
    }
}

// handler: (event: any, context: Context) => Promise<any>
async fn start_process_events<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    handler: &Function<'js>,
    base_url: &str,
    config: &RuntimeConfig,
) -> rquickjs::Result<()> {
    let mut iterations = 0;
    let next_invocation_url = [base_url, "/invocation/next"].concat();

    let mut request_id = String::with_capacity(36); //length of uuid

    let lambda_environment = LambdaEnvironment::new();

    let promise_ctor: Value = ctx.globals().get(PredefinedAtom::Promise)?;

    loop {
        let now = Instant::now();

        if let Err(err) = process_event(
            ctx,
            client,
            handler,
            base_url,
            &next_invocation_url,
            &mut request_id,
            &lambda_environment,
            &promise_ctor,
        )
        .await
        .map_err(|e| CaughtError::from_error(ctx, e))
        {
            if request_id.is_empty() {
                Vm::print_error_and_exit(ctx, err);
            }

            let error_path = ["/invocation/", &request_id, "/error"].concat();
            if let Err(err) =
                post_error(ctx, client, base_url, &error_path, &err, Some(&request_id))
                    .await
                    .map_err(|e| CaughtError::from_error(ctx, e))
            {
                Vm::print_error_and_exit(ctx, err);
            }
        }
        if config.iterations > 0 {
            if iterations >= config.iterations - 1 {
                info!("Done in {:?}", now.elapsed().as_millis());
                break;
            }
            iterations += 1;
        }
        request_id.clear();
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_event<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    handler: &Function<'js>,
    base_url: &str,
    next_invocation_url: &str,
    request_id: &mut String,
    lambda_environment: &LambdaEnvironment,
    promise_constructor: &Value<'js>,
) -> Result<()> {
    let NextInvocationResponse { event, context } =
        next_invocation(ctx, client, next_invocation_url, lambda_environment).await?;
    request_id.clear();
    request_id.push_str(&context.aws_request_id);
    LAMBDA_REQUEST_ID
        .write()
        .unwrap()
        .replace(context.aws_request_id.to_owned());

    let js_context = context.into_js(ctx)?;
    let handler_result =
        handler.call::<_, Value>((event.clone(), js_context.as_value().clone()))?;

    let result = match handler_result.as_object() {
        Some(obj) if obj.is_instance_of(promise_constructor) => {
            handler_result
                .get::<Promise>()?
                .into_future::<Value>()
                .await?
        },
        _ => handler_result,
    };
    invoke_response(ctx, client, base_url, request_id, result).await?;
    Ok(())
}

async fn post_error<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    base_url: &str,
    path: &str,
    error: &CaughtError<'js>,
    request_id: Option<&String>,
) -> Result<()> {
    let mut error_stack = String::new();
    let mut error_type = String::from("Error");
    let error_msg = match error {
        CaughtError::Error(err) => format!("Error: {:?}", &err),
        CaughtError::Exception(ex) => {
            let error_name = get_class_name(ex)
                .unwrap_or(None)
                .unwrap_or(String::from("Error"));

            let mut str = String::with_capacity(100);
            str.push_str(&error_name);
            str.push_str(": ");
            str.push_str(&ex.message().unwrap_or_default());

            error_type = error_name;

            if let Some(mut stack) = ex.stack() {
                console::replace_newline_with_carriage_return(&mut stack);
                error_stack = stack
            }
            str
        },
        CaughtError::Value(value) => {
            let log_msg = console::format_values(ctx, Rest(vec![value.clone()]), false)
                .unwrap_or(String::from("{unknown value}"));
            ["Error: ", &log_msg].concat()
        },
    };

    let error_object = Object::new(ctx.clone())?;
    error_object.set("errorType", error_type.clone())?;
    error_object.set("errorMessage", error_msg)?;
    error_object.set("stackTrace", error_stack)?;
    error_object.set("requestId", request_id.unwrap_or(&String::from("n/a")))?;
    let error_object = error_object.into_value();

    #[cfg(not(test))]
    {
        console::log_std_err(
            ctx,
            Rest(vec![error_object.clone()]),
            console::LogLevel::Error,
        )?;
    }

    let error_body = json_stringify(ctx, error_object)?.unwrap_or_default();

    let url = [base_url, path].concat();

    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header(CONTENT_TYPE, "application/json")
        .header(&HEADER_ERROR_TYPE, error_type)
        .body(Full::from(bytes::Bytes::from(error_body)))
        .or_throw(ctx)?;
    let res = client.request(req).await.or_throw(ctx)?;
    if res.status() != StatusCode::ACCEPTED {
        let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
        let res_str = String::from_utf8_lossy(res_bytes.as_slice());
        return Err(Exception::throw_message(
            ctx,
            &["Unexpected ", path, " response: ", &res_str].concat(),
        ));
    }
    Ok(())
}

fn get_module_and_handler_name<'a>(ctx: &Ctx, handler: &'a str) -> Result<(&'a str, &'a str)> {
    handler
        .rfind('.')
        .and_then(|pos| {
            let (module_name, handler_name) = handler.split_at(pos);
            if !module_name.is_empty() && handler_name.len() > 1 {
                //removes the dot and make sure the length is greater than 0
                Some((module_name, &handler_name[1..]))
            } else {
                None
            }
        })
        .ok_or_else(|| {
            Exception::throw_message(
                ctx,
                &[
                    "Invalid handler name or LAMBDA_HANDLER env value: \"",
                    handler,
                    "\": Should be in format {{filepath}}.{{method_name}}",
                ]
                .concat(),
            )
        })
}

fn get_task_root() -> String {
    env::var(ENV_LAMBDA_TASK_ROOT).unwrap_or_else(|_| {
        env::current_dir()
            .ok()
            .and_then(|path| path.into_os_string().into_string().ok())
            .unwrap_or_else(|| "/".to_string())
    })
}

fn get_header_value(headers: &HeaderMap, header: &HeaderName) -> StdResult<String, String> {
    headers
        .get(header)
        .map(|h| String::from_utf8_lossy(h.as_bytes()).to_string())
        .ok_or_else(|| ["Missing or invalid header: ", header.as_str()].concat())
}

#[cfg(test)]
mod tests {

    use hyper::header::CONTENT_TYPE;
    use rquickjs::{async_with, CatchResultExt};
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    use crate::{
        modules::llrt::uuid::uuidv4,
        runtime_client::{
            self, RuntimeConfig, ENV_RUNTIME_PATH, HEADER_INVOKED_FUNCTION_ARN, HEADER_REQUEST_ID,
        },
        vm::Vm,
    };

    #[tokio::test]
    async fn runtime() {
        let mock_server = MockServer::start().await;

        Mock::given(matchers::method("GET"))
            .and(matchers::path(format!(
                "{}/invocation/next",
                ENV_RUNTIME_PATH
            )))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(&HEADER_REQUEST_ID, uuidv4())
                    .insert_header(&HEADER_INVOKED_FUNCTION_ARN, "n/a")
                    .set_body_string(r#"{"hello": "world"}"#),
            )
            .mount(&mock_server)
            .await;

        Mock::given(matchers::method("POST"))
            .and(matchers::path_regex(
                r#"invocation/[A-z0-9-]{1,}/response$"#,
            ))
            .and(matchers::header(&CONTENT_TYPE, "application/json"))
            .respond_with(ResponseTemplate::new(202))
            .mount(&mock_server)
            .await;

        Mock::given(matchers::method("POST"))
            .and(matchers::path_regex(r#"invocation/[A-z0-9-]{1,}/error$"#))
            .and(matchers::header(&CONTENT_TYPE, "application/json"))
            .respond_with(ResponseTemplate::new(202))
            .mount(&mock_server)
            .await;

        let runtime_api = format!("localhost:{}", mock_server.address().port());

        let vm = Vm::new().await.unwrap();

        async fn run_with_handler(vm: &Vm, handler: &str, runtime_api: &str) {
            let mock_config = RuntimeConfig {
                runtime_api: runtime_api.into(),
                handler: handler.into(),
                iterations: 10,
            };

            async_with!(vm.ctx => |ctx|{
                runtime_client::start_with_cfg(&ctx,mock_config).await.catch(&ctx).unwrap()
            })
            .await;
        }

        run_with_handler(&vm, "../fixtures/handler.handler", &runtime_api).await;
        run_with_handler(&vm, "../fixtures/primitive-handler.handler", &runtime_api).await;
        run_with_handler(&vm, "../fixtures/throwing-handler.handler", &runtime_api).await;

        vm.runtime.idle().await;
    }
}
