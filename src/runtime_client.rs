// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::json::parse::json_parse;
use crate::json::stringify::{self, json_stringify};
use crate::net::HTTP_CLIENT;
use crate::utils::result::ResultExt;
use crate::vm::{ErrorDetails, Vm};
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

use rquickjs::Exception;
use rquickjs::{
    atom::PredefinedAtom, prelude::Func, promise::Promise, Array, CatchResultExt, CaughtError, Ctx,
    Function, IntoJs, Module, Object, Result, ThrowResultExt, Value,
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
                    &format!(
                        "Environment variable {} is not defined.",
                        AWS_LAMBDA_RUNTIME_API
                    ),
                )
            })?,
            handler: env::var(ENV_LAMBDA_HANDLER)
                .or_else(|_| env::var(ENV_UNDERSCORE_HANDLER))
                .map_err(|_| {
                    Exception::throw_message(
                        ctx,
                        &format!(
                            "Environment {} or {} is not defined.",
                            ENV_UNDERSCORE_HANDLER, ENV_LAMBDA_HANDLER
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

    let js_handler_module: Object = Module::import(ctx, format!("{}/{}", task_root, module_name))?;
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
        let init_promise = js_init_tasks.get::<Promise<()>>(0)?;
        init_promise.await.catch(ctx).throw(ctx)?;
    } else if init_tasks_size > 1 {
        let promise_actor: Object = ctx.globals().get(PredefinedAtom::Promise)?;
        let init_promise: Promise<()> = promise_actor
            .get::<_, Function>("all")?
            .call((js_init_tasks.clone(),))?;
        init_promise.await.catch(ctx).throw(ctx)?;
    }

    let handler: Value = js_handler_module.get(handler_name.as_str())?;

    if !handler.is_function() {
        return Err(Exception::throw_message(
            ctx,
            &format!(
                "\"{}\" is not a function in \"{}\"",
                handler_name, module_name
            ),
        ));
    }

    let client = (*HTTP_CLIENT).clone();

    let base_url = format!("http://{}/{}", config.runtime_api, ENV_RUNTIME_PATH);
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
    client: &HyperClient,
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
            &format!("Unexpected /invocation/next response: {:?}", res_str),
        ));
    }

    if let Some(trace_id_value) = res.headers().get(&HEADER_TRACE_ID) {
        let trace_id_value = String::from_utf8_lossy(trace_id_value.as_bytes());
        env::set_var(ENV_X_AMZN_TRACE_ID, trace_id_value.as_ref());
    } else {
        env::remove_var(ENV_X_AMZN_TRACE_ID);
    };

    let headers = res.headers();

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
        json_parse(ctx, json.as_bytes().into())
    } else {
        rquickjs::Undefined.into_js(ctx)
    }?;
    let cognito_identity_json = if let Some(json) = headers.get(&HEADER_COGNITO_IDENTITY) {
        json_parse(ctx, json.as_bytes().into())
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
    let event: Value<'js> = json_parse(ctx, bytes.into())?;

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
        .uri(format!("{}/invocation/{}/response", base_url, request_id))
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
                &format!("Unexpected /invocation/response response: {}", res_str),
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
    let next_invocation_url = format!("{base_url}/invocation/next");

    let mut request_id = String::with_capacity(36); //length of uuid

    let lambda_environment = LambdaEnvironment::new();

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
        )
        .await
        .map_err(|e| CaughtError::from_error(ctx, e))
        {
            if request_id.is_empty() {
                Vm::print_error_and_exit(ctx, err);
            }

            let error_path = format!("/invocation/{}/error", request_id);
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

async fn process_event<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    handler: &Function<'js>,
    base_url: &str,
    next_invocation_url: &str,
    request_id: &mut String,
    lambda_environment: &LambdaEnvironment,
) -> Result<()> {
    let NextInvocationResponse { event, context } =
        next_invocation(ctx, client, next_invocation_url, lambda_environment).await?;
    *request_id = context.aws_request_id.clone();
    LAMBDA_REQUEST_ID
        .write()
        .unwrap()
        .replace(context.aws_request_id.clone());

    let js_context = context.into_js(ctx)?;
    let promise =
        handler.call::<_, Promise<Value>>((event.clone(), js_context.as_value().clone()))?;
    let result: Value = promise.await?;
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
    let ErrorDetails { msg, r#type, stack } = Vm::error_details(ctx, error);

    let error_object = Object::new(ctx.clone())?;
    error_object.set("errorType", r#type.clone())?;
    error_object.set("errorMessage", msg)?;
    error_object.set("stackTrace", stack)?;
    error_object.set("requestId", request_id.unwrap_or(&String::from("n/a")))?;
    let error_object = error_object.into_value();

    #[cfg(not(test))]
    {
        use crate::console;
        use rquickjs::function::Rest;
        console::log_std_err(
            ctx,
            Rest(vec![error_object.clone()]),
            console::LogLevel::Error,
        )?;
    }

    let error_body = json_stringify(ctx, error_object)?.unwrap_or_default();

    let url = format!("{base_url}{path}");

    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header(CONTENT_TYPE, "application/json")
        .header(&HEADER_ERROR_TYPE, r#type)
        .body(Full::from(bytes::Bytes::from(error_body)))
        .or_throw(ctx)?;
    let res = client.request(req).await.or_throw(ctx)?;
    if res.status() != StatusCode::ACCEPTED {
        let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
        let res_str = String::from_utf8_lossy(res_bytes.as_slice());
        return Err(Exception::throw_message(
            ctx,
            &format!("Unexpected {} response: {}", path, res_str),
        ));
    }
    Ok(())
}

fn get_module_and_handler_name(ctx: &Ctx, handler: &str) -> Result<(String, String)> {
    let parts: Vec<_> = handler.split('.').filter(|&s| !s.is_empty()).collect();

    match parts.as_slice() {
        [module_name, handler_name] => Ok((module_name.to_string(), handler_name.to_string())),
        _ => Err(Exception::throw_message(ctx,  &format!("Invalid handler name or LAMBDA_HANDLER env value: \"{}\": Should be in format {{filename}}.{{method_name}}", handler)))
    }
}

fn get_task_root() -> String {
    env::var(ENV_LAMBDA_TASK_ROOT).unwrap_or_else(|_| {
        env::current_dir()
            .unwrap_or("/".into())
            .into_os_string()
            .into_string()
            .unwrap()
    })
}

fn get_header_value(headers: &HeaderMap, header: &HeaderName) -> StdResult<String, String> {
    headers
        .get(header)
        .map(|h| String::from_utf8_lossy(h.as_bytes()).to_string())
        .ok_or_else(|| format!("Missing header: {}", header))
}

#[cfg(test)]
mod tests {

    use hyper::header::CONTENT_TYPE;
    use rquickjs::{async_with, CatchResultExt};
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    use crate::{
        runtime_client::{
            self, RuntimeConfig, ENV_RUNTIME_PATH, HEADER_INVOKED_FUNCTION_ARN, HEADER_REQUEST_ID,
        },
        uuid::uuidv4,
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

        let mock_config = RuntimeConfig {
            runtime_api: format!("localhost:{}", mock_server.address().port()),
            handler: "fixtures/handler.handler".into(),
            iterations: 10,
        };

        let vm = Vm::new().await.unwrap();

        async_with!(vm.ctx => |ctx|{
            runtime_client::start_with_cfg(&ctx,mock_config).await.catch(&ctx).unwrap()
        })
        .await;

        let throwing_mock_config = RuntimeConfig {
            runtime_api: format!("localhost:{}", mock_server.address().port()),
            handler: "fixtures/throwing-handler.handler".into(),
            iterations: 10,
        };

        async_with!(vm.ctx => |ctx|{
            runtime_client::start_with_cfg(&ctx,throwing_mock_config).await.catch(&ctx).unwrap()
        })
        .await;

        vm.runtime.idle().await;
    }
}
