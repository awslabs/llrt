// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::{
    cell::RefCell,
    convert::Infallible,
    env,
    pin::Pin,
    rc::Rc,
    result::Result as StdResult,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    task::{Context, Poll},
    time::Instant,
};

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{
    body::{Body, Frame, Incoming},
    header::{HeaderMap, HeaderValue, CONNECTION, CONTENT_TYPE, TRAILER, TRANSFER_ENCODING},
    http::header::HeaderName,
    Request, Response, StatusCode,
};
use jiff::Timestamp;
use once_cell::sync::Lazy;
use rquickjs::{
    atom::PredefinedAtom,
    function::{Opt, Rest, This},
    prelude::Func,
    promise::Promise,
    qjs, CatchResultExt, CaughtError, Coerced, Constructor, Ctx, Exception, Function, IntoJs,
    Module, Object, Result, Symbol, Undefined, Value,
};
use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::libs::{
    context::CtxExtension,
    encoding::bytes_to_b64_string,
    json::{
        parse::json_parse,
        stringify::{self, json_stringify},
    },
    logging::{format_values, replace_newline_with_carriage_return},
    utils::{bytes::ObjectBytes, class::get_class_name, error::ErrorExtensions, result::ResultExt},
};

#[cfg(not(test))]
use crate::modules::console::log_error;
use crate::modules::https::{HyperClient, HTTP_CLIENT};
use llrt_utils::latch::Latch;

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

// Response streaming: https://docs.aws.amazon.com/lambda/latest/dg/runtimes-custom.html#runtimes-custom-response-streaming
// The symbols and the "response" value match the Node.js runtime, so handlers
// wrapped by libraries such as Middy or Hono are recognized.
const HANDLER_STREAMING: &str = "aws.lambda.runtime.handler.streaming";
const HANDLER_HIGHWATERMARK: &str = "aws.lambda.runtime.handler.streaming.highWaterMark";
const STREAM_RESPONSE: &str = "response";
const DEFAULT_STREAM_CONTENT_TYPE: &str = "application/octet-stream";
static HEADER_RESPONSE_MODE: HeaderName =
    HeaderName::from_static("lambda-runtime-function-response-mode");
static HEADER_ERROR_BODY: HeaderName =
    HeaderName::from_static("lambda-runtime-function-error-body");
static TRAILER_ERROR_FIELDS: HeaderValue = HeaderValue::from_static(
    "Lambda-Runtime-Function-Error-Type, Lambda-Runtime-Function-Error-Body",
);

pub struct SdkClientInitState {
    rt: *mut qjs::JSRuntime,
    latch: Arc<Latch>,
    endpoints: Vec<Box<str>>, //we're likely to have a small number of clients
}
impl SdkClientInitState {
    fn new(rt: *mut qjs::JSRuntime) -> Self {
        Self {
            rt,
            latch: Latch::default().into(),
            endpoints: Vec::new(),
        }
    }
}

unsafe impl Sync for SdkClientInitState {}
unsafe impl Send for SdkClientInitState {}

fn get_sdk_client_init_state_mut<'a>(
    guard: &'a mut RwLockWriteGuard<Vec<SdkClientInitState>>,
    rt: *mut qjs::JSRuntime,
) -> &'a mut SdkClientInitState {
    let state = guard.iter_mut().find(|state| state.rt == rt);

    //save a branch
    unsafe { state.unwrap_unchecked() }
}

fn get_sdk_client_init_state<'a>(
    guard: &'a RwLockReadGuard<Vec<SdkClientInitState>>,
    rt: *mut qjs::JSRuntime,
) -> &'a SdkClientInitState {
    let state = guard.iter().find(|state| state.rt == rt);

    //save a branch
    unsafe { state.unwrap_unchecked() }
}

pub static LAMBDA_REQUEST_ID: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

static SDK_CONNECTION_INIT_LATCH: Lazy<RwLock<Vec<SdkClientInitState>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

pub fn check_client_inited(rt: *mut qjs::JSRuntime, endpoint: &str) -> bool {
    let mut write = SDK_CONNECTION_INIT_LATCH.write().unwrap();
    let state = get_sdk_client_init_state_mut(&mut write, rt);
    if state.endpoints.iter().any(|s| s.as_ref() == endpoint) {
        return true;
    }
    state.endpoints.push(endpoint.into());
    state.latch.increment();
    false
}

pub fn mark_client_inited(rt: *mut qjs::JSRuntime) {
    let mut write = SDK_CONNECTION_INIT_LATCH.write().unwrap();
    let state = get_sdk_client_init_state_mut(&mut write, rt);
    state.latch.decrement();
}

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

impl<'js> IntoJs<'js> for LambdaContext<'js, '_> {
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

    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
    {
        let mut state_ref = SDK_CONNECTION_INIT_LATCH.write().unwrap();
        state_ref.push(SdkClientInitState::new(rt));
    }

    let specifier: String = [task_root.as_str(), module_name].join("/");

    // Handler modules call `awslambda.streamifyResponse` while they load.
    init_awslambda_global(ctx)?;

    let import_promise = Module::import(ctx, specifier.as_bytes())?;

    let latch = {
        let state_ref = SDK_CONNECTION_INIT_LATCH.read().unwrap();
        get_sdk_client_init_state(&state_ref, rt).latch.clone()
    };
    latch.wait().await;

    let js_handler_module = import_promise.into_future::<Object>().await?;

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
    if let Err(err) = async {
        let streaming = StreamingHandler::from_handler(ctx, handler).await?;
        start_process_events(
            ctx,
            &client,
            handler,
            streaming.as_ref(),
            base_url.as_str(),
            &config,
        )
        .await
    }
    .await
    .catch(ctx)
    {
        post_error(ctx, &client, &base_url, "/init/error", &err, None).await?;
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
        .body(BoxBody::new(Full::default()))
        .or_throw(ctx)?;

    let res = client.request(req).await.or_throw(ctx)?;

    if res.status() != StatusCode::OK {
        let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
        let res_str = String::from_utf8_lossy(res_bytes.as_ref());
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
        let now = Timestamp::now();
        deadline_ms - now.as_millisecond()
    });
    let get_remaining_time_in_millis = get_remaining_time_in_millis
        .into_js(ctx)?
        .into_function()
        .unwrap();

    let client_context = if let Some(json) = headers.get(&HEADER_CLIENT_CONTEXT) {
        json_parse(ctx, json.as_bytes())
    } else {
        Undefined.into_js(ctx)
    }?;
    let cognito_identity_json = if let Some(json) = headers.get(&HEADER_COGNITO_IDENTITY) {
        json_parse(ctx, json.as_bytes())
    } else {
        Undefined.into_js(ctx)
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
        .body(BoxBody::new(Full::from(bytes::Bytes::from(
            result_json.unwrap_or_default(),
        ))))
        .or_throw(ctx)?;

    let res = client.request(req).await.or_throw(ctx)?;
    ensure_response_accepted(ctx, res).await
}

async fn ensure_response_accepted(ctx: &Ctx<'_>, res: Response<Incoming>) -> Result<()> {
    match res.status() {
        StatusCode::ACCEPTED => Ok(()),
        _ => {
            let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
            let res_str = String::from_utf8_lossy(res_bytes.as_ref());
            Err(Exception::throw_message(
                ctx,
                &["Unexpected /invocation/response response: ", &res_str].concat(),
            ))
        },
    }
}

// handler: (event: any, context: Context) => Promise<any>
// streaming handler: (event: any, responseStream: Writable, context: Context) => Promise<void>
async fn start_process_events<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    handler: &Function<'js>,
    streaming: Option<&StreamingHandler<'js>>,
    base_url: &str,
    config: &RuntimeConfig,
) -> Result<()> {
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
            streaming,
            base_url,
            &next_invocation_url,
            &mut request_id,
            &lambda_environment,
            &promise_ctor,
        )
        .await
        {
            if request_id.is_empty() {
                return Err(err);
            }

            let err = CaughtError::from_error(ctx, err);

            let error_path = ["/invocation/", &request_id, "/error"].concat();
            post_error(
                ctx,
                client,
                base_url,
                &error_path,
                &err,
                Some(request_id.as_str()),
            )
            .await?;
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
    streaming: Option<&StreamingHandler<'js>>,
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

    if let Some(streaming) = streaming {
        return process_streaming_event(
            ctx,
            client,
            handler,
            streaming,
            base_url,
            request_id,
            event,
            js_context,
            promise_constructor,
        )
        .await;
    }

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
    request_id: Option<&str>,
) -> Result<()> {
    let (error_type, error_object) = format_error(ctx, error, request_id)?;

    log_error_object(ctx, &error_object)?;

    let error_body = json_stringify(ctx, error_object)?.unwrap_or_default();

    let url = [base_url, path].concat();

    let req = Request::builder()
        .method("POST")
        .uri(url)
        .header(CONTENT_TYPE, "application/json")
        .header(&HEADER_ERROR_TYPE, error_type)
        .body(BoxBody::new(Full::from(bytes::Bytes::from(error_body))))
        .or_throw(ctx)?;
    let res = client.request(req).await.or_throw(ctx)?;
    if res.status() != StatusCode::ACCEPTED {
        let res_bytes = res.collect().await.or_throw(ctx)?.to_bytes();
        let res_str = String::from_utf8_lossy(res_bytes.as_ref());
        return Err(Exception::throw_message(
            ctx,
            &["Unexpected ", path, " response: ", &res_str].concat(),
        ));
    }
    Ok(())
}

fn log_error_object<'js>(_ctx: &Ctx<'js>, _error_object: &Value<'js>) -> Result<()> {
    #[cfg(not(test))]
    {
        log_error(_ctx.clone(), Rest(vec![_error_object.clone()]))?;
    }
    Ok(())
}

/// Builds the error type and the error object the Runtime API expects.
fn format_error<'js>(
    ctx: &Ctx<'js>,
    error: &CaughtError<'js>,
    request_id: Option<&str>,
) -> Result<(String, Value<'js>)> {
    let mut error_stack = None;
    let mut error_type = None;
    let error_msg = match error {
        CaughtError::Error(err) => format!("Error: {:?}", err),
        CaughtError::Exception(ex) => {
            let error_name = get_class_name(ex)
                .unwrap_or(None)
                .unwrap_or(String::from("Error"));

            let message = ex.message().unwrap_or_default();

            error_type = Some(error_name);

            if let Some(mut stack) = ex.stack() {
                replace_newline_with_carriage_return(&mut stack);
                error_stack = Some(stack);
            }
            message
        },
        CaughtError::Value(value) => {
            let log_msg = format_values(ctx, Rest(vec![value.clone()]), false, true)
                .unwrap_or(String::from("{unknown value}"));
            ["Error: ", &log_msg].concat()
        },
    };

    let error_type = error_type.unwrap_or_else(|| "Error".into());
    let error_stack = error_stack.unwrap_or_default();

    let error_object = Object::new(ctx.clone())?;
    error_object.set("errorType", &error_type)?;
    error_object.set("errorMessage", error_msg)?;
    error_object.set("stackTrace", error_stack)?;
    error_object.set("requestId", request_id.unwrap_or("n/a"))?;

    Ok((error_type, error_object.into_value()))
}

/// Defines `globalThis.awslambda`, as the Node.js runtime does.
fn init_awslambda_global(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();
    let awslambda = match globals.get::<_, Option<Object>>("awslambda")? {
        Some(awslambda) => awslambda,
        None => {
            let awslambda = Object::new(ctx.clone())?;
            globals.set("awslambda", awslambda.clone())?;
            awslambda
        },
    };
    awslambda.set("streamifyResponse", Func::from(streamify_response))?;
    Ok(())
}

// awslambda.streamifyResponse(handler, options?: { highWaterMark?: number })
fn streamify_response<'js>(
    ctx: Ctx<'js>,
    handler: Value<'js>,
    options: Opt<Value<'js>>,
) -> Result<Value<'js>> {
    if let Some(handler_obj) = handler.as_object() {
        handler_obj.set(
            Symbol::new_global(ctx.clone(), HANDLER_STREAMING)?,
            STREAM_RESPONSE,
        )?;
        let high_water_mark = match options.0.as_ref().and_then(Value::as_object) {
            Some(options) => options.get::<_, Value>("highWaterMark")?.as_number(),
            None => None,
        };
        if let Some(high_water_mark) = high_water_mark {
            handler_obj.set(
                Symbol::new_global(ctx, HANDLER_HIGHWATERMARK)?,
                high_water_mark.trunc(),
            )?;
        }
    }
    Ok(handler)
}

/// A handler wrapped by `awslambda.streamifyResponse`.
struct StreamingHandler<'js> {
    writable: Constructor<'js>,
    high_water_mark: Option<f64>,
}

impl<'js> StreamingHandler<'js> {
    async fn from_handler(ctx: &Ctx<'js>, handler: &Function<'js>) -> Result<Option<Self>> {
        let mode: Value = handler.get(Symbol::new_global(ctx.clone(), HANDLER_STREAMING)?)?;
        if !mode.get::<Coerced<bool>>()?.0 {
            return Ok(None);
        }
        if mode.as_string().and_then(|mode| mode.to_string().ok()) != Some(STREAM_RESPONSE.into()) {
            return Err(Exception::throw_message(
                ctx,
                "Only response streaming is supported.",
            ));
        }
        let high_water_mark = handler
            .get::<_, Value>(Symbol::new_global(ctx.clone(), HANDLER_HIGHWATERMARK)?)?
            .as_number()
            .filter(|high_water_mark| high_water_mark.is_finite() && *high_water_mark > 0.0);

        let stream_module = Module::import(ctx, "stream")?
            .into_future::<Object>()
            .await?;

        Ok(Some(Self {
            writable: stream_module.get("Writable")?,
            high_water_mark,
        }))
    }
}

/// A body frame, and for data, a sender that fires once the request body
/// takes it. JS calls queue frames synchronously, so they stay in call order.
type BodyItem = (Frame<Bytes>, Option<oneshot::Sender<()>>);

/// State shared by the response stream's JS callbacks. The request to the
/// Runtime API starts on the first write, so `setContentType` works until then.
struct ResponseStreamState {
    content_type: String,
    start_tx: Option<oneshot::Sender<String>>,
    body_tx: Option<mpsc::UnboundedSender<BodyItem>>,
    // Set when the handler fails; `final` sends them after the data.
    error_trailers: Option<HeaderMap>,
    failed: bool,
}

impl ResponseStreamState {
    fn start(&mut self) {
        if let Some(start_tx) = self.start_tx.take() {
            let _ = start_tx.send(std::mem::take(&mut self.content_type));
        }
    }

    /// Queues a chunk. The receiver resolves once the request body takes it,
    /// and there is none for an empty chunk. Err means the request is over,
    /// and the stream is marked failed because `respond` reports why.
    fn write(&mut self, chunk: Bytes) -> StdResult<Option<oneshot::Receiver<()>>, ()> {
        self.start();
        if chunk.is_empty() {
            return Ok(None);
        }
        let (ack_tx, ack_rx) = oneshot::channel();
        match &self.body_tx {
            Some(body_tx) if body_tx.send((Frame::data(chunk), Some(ack_tx))).is_ok() => {
                Ok(Some(ack_rx))
            },
            _ => {
                self.failed = true;
                Err(())
            },
        }
    }

    /// Ends the body, sending the trailers first if there are any.
    fn close(&mut self, trailers: Option<HeaderMap>) {
        self.start();
        if let (Some(body_tx), Some(trailers)) = (self.body_tx.take(), trailers) {
            let _ = body_tx.send((Frame::trailers(trailers), None));
        }
    }

    /// Marks an open stream failed. False if it already failed or has ended.
    fn try_fail(&mut self) -> bool {
        if self.failed || self.body_tx.is_none() {
            return false;
        }
        self.failed = true;
        true
    }
}

/// The streamed request body. Trailers end it.
struct ResponseStreamBody {
    rx: mpsc::UnboundedReceiver<BodyItem>,
    done: bool,
}

impl Body for ResponseStreamBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<StdResult<Frame<Self::Data>, Self::Error>>> {
        if self.done {
            return Poll::Ready(None);
        }
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some((frame, ack_tx))) => {
                if let Some(ack_tx) = ack_tx {
                    let _ = ack_tx.send(());
                }
                self.done = frame.is_trailers();
                Poll::Ready(Some(Ok(frame)))
            },
            Poll::Ready(None) => {
                self.done = true;
                Poll::Ready(None)
            },
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.done
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_streaming_event<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    handler: &Function<'js>,
    streaming: &StreamingHandler<'js>,
    base_url: &str,
    request_id: &str,
    event: Value<'js>,
    js_context: Value<'js>,
    promise_constructor: &Value<'js>,
) -> Result<()> {
    let (start_tx, start_rx) = oneshot::channel();
    let (body_tx, body_rx) = mpsc::unbounded_channel();
    let state = Rc::new(RefCell::new(ResponseStreamState {
        content_type: DEFAULT_STREAM_CONTENT_TYPE.into(),
        start_tx: Some(start_tx),
        body_tx: Some(body_tx),
        error_trailers: None,
        failed: false,
    }));
    let response_stream = create_response_stream(ctx, streaming, &state, request_id)?;

    // Errors from here on never reach the main loop: the Runtime API refuses
    // /error once a response has started, and a refused post stops the runtime.
    let invoke = async {
        let result = call_streaming_handler(
            ctx,
            handler,
            &state,
            &response_stream,
            request_id,
            event,
            js_context,
            promise_constructor,
        )
        .await;
        if let Err(err) = result {
            abort_response_stream(ctx, &state, &response_stream, err, request_id);
        }
    };

    let respond = async {
        let Ok(content_type) = start_rx.await else {
            return;
        };
        let body = ResponseStreamBody {
            rx: body_rx,
            done: false,
        };
        let result =
            post_streamed_response(ctx, client, base_url, request_id, content_type, body).await;
        if let Err(err) = result {
            abort_response_stream(ctx, &state, &response_stream, err, request_id);
        }
    };

    // As in Node.js, the invocation ends once the handler's promise has
    // settled and the response stream has ended.
    tokio::join!(invoke, respond);
    Ok(())
}

/// Calls a streaming handler and waits for its promise. A handler failure is
/// reported in the stream; Err is a failure of the runtime itself.
#[allow(clippy::too_many_arguments)]
async fn call_streaming_handler<'js>(
    ctx: &Ctx<'js>,
    handler: &Function<'js>,
    state: &RefCell<ResponseStreamState>,
    response_stream: &Object<'js>,
    request_id: &str,
    event: Value<'js>,
    js_context: Value<'js>,
    promise_constructor: &Value<'js>,
) -> Result<()> {
    let result = handler
        .call::<_, Value>((event, response_stream.clone(), js_context))
        .and_then(|result| match result.as_object() {
            Some(obj) if obj.is_instance_of(promise_constructor) => Ok(result),
            _ => Err(Exception::throw_message(
                ctx,
                "Streaming does not support non-async handlers.",
            )),
        });
    let result = match result {
        Ok(promise) => promise.get::<Promise>()?.into_future::<Value>().await,
        Err(err) => Err(err),
    };
    if let Err(err) = result {
        let err = CaughtError::from_error(ctx, err);
        fail_response_stream(ctx, state, response_stream, &err, request_id)?;
    }
    Ok(())
}

/// Posts the streamed response. The Runtime API answers once the body ends,
/// or earlier if it refuses the response.
async fn post_streamed_response(
    ctx: &Ctx<'_>,
    client: &HyperClient,
    base_url: &str,
    request_id: &str,
    content_type: String,
    body: ResponseStreamBody,
) -> Result<()> {
    let req = Request::builder()
        .method("POST")
        .uri([base_url, "/invocation/", request_id, "/response"].concat())
        .header(&HEADER_RESPONSE_MODE, "streaming")
        .header(TRANSFER_ENCODING, "chunked")
        .header(TRAILER, TRAILER_ERROR_FIELDS.clone())
        .header(CONTENT_TYPE, content_type)
        // The custom runtime docs say to close the connection once a streamed
        // response is written, so it doesn't go back to the pool for /next.
        .header(CONNECTION, "close")
        .body(BoxBody::new(body))
        .or_throw(ctx)?;
    let res = client.request(req).await.or_throw(ctx)?;
    ensure_response_accepted(ctx, res).await
}

/// Builds the `stream.Writable` passed to a streaming handler.
fn create_response_stream<'js>(
    ctx: &Ctx<'js>,
    streaming: &StreamingHandler<'js>,
    state: &Rc<RefCell<ResponseStreamState>>,
    request_id: &str,
) -> Result<Object<'js>> {
    let options = Object::new(ctx.clone())?;
    if let Some(high_water_mark) = streaming.high_water_mark {
        options.set("highWaterMark", high_water_mark)?;
    }

    let write_state = state.clone();
    options.set(
        "write",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, chunk: Value<'js>, _encoding: Value<'js>, cb: Function<'js>| {
                let chunk = ObjectBytes::from(&ctx, &chunk)?;
                let chunk = Bytes::copy_from_slice(chunk.as_bytes(&ctx)?);
                let result = write_state.borrow_mut().write(chunk);
                let ack_rx = match result {
                    Ok(Some(ack_rx)) => ack_rx,
                    Ok(None) => return cb.call::<_, ()>(()),
                    Err(()) => return cb.call::<_, ()>((closed_stream_error(&ctx)?,)),
                };
                let state = write_state.clone();
                ctx.clone().spawn_exit_simple(async move {
                    if ack_rx.await.is_ok() {
                        return cb.call::<_, ()>(());
                    }
                    // The request dropped the chunk, so it is over.
                    state.borrow_mut().failed = true;
                    cb.call::<_, ()>((closed_stream_error(&ctx)?,))
                });
                Ok(())
            },
        )?,
    )?;

    let final_state = state.clone();
    options.set(
        "final",
        Function::new(ctx.clone(), move |cb: Function<'js>| {
            let mut state = final_state.borrow_mut();
            let trailers = state.error_trailers.take();
            state.close(trailers);
            drop(state);
            cb.call::<_, ()>(())
        })?,
    )?;

    // Destroying the stream before it ends, as `pipeline()` does when the
    // source fails, sends the error in the trailers. Unwritten data is dropped.
    let destroy_state = state.clone();
    let request_id = request_id.to_owned();
    options.set(
        "destroy",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, err: Value<'js>, cb: Function<'js>| {
                if destroy_state.borrow_mut().try_fail() {
                    let error = if err.is_null() || err.is_undefined() {
                        CaughtError::Exception(Exception::from_message(
                            ctx.clone(),
                            "Response stream was destroyed before it ended.",
                        )?)
                    } else {
                        match err
                            .as_object()
                            .and_then(|o| Exception::from_object(o.clone()))
                        {
                            Some(ex) => CaughtError::Exception(ex),
                            None => CaughtError::Value(err.clone()),
                        }
                    };
                    let trailers = report_stream_error(&ctx, &error, &request_id)?;
                    destroy_state.borrow_mut().close(Some(trailers));
                }
                cb.call::<_, ()>((err,))
            },
        )?,
    )?;

    let stream: Object = streaming.writable.construct((options,))?;

    // As in Node.js, `write` sends chunks other than strings and Uint8Arrays
    // (Buffers included) as JSON. `end(chunk)` doesn't, in Node.js either.
    // The closure captures no JS values: the GC can't see into it, so they
    // would keep the stream's cycle alive.
    stream.set(
        "write",
        Function::new(
            ctx.clone(),
            |ctx: Ctx<'js>, this: This<Object<'js>>, chunk: Value<'js>, rest: Rest<Value<'js>>| {
                let uint8_array: Value = ctx.globals().get(PredefinedAtom::Uint8Array)?;
                let is_bytes = chunk.is_string()
                    || chunk
                        .as_object()
                        .is_some_and(|chunk| chunk.is_instance_of(&uint8_array));
                let chunk = if is_bytes {
                    chunk
                } else {
                    json_stringify(&ctx, chunk)?.into_js(&ctx)?
                };
                // Writable.prototype.write
                let write: Function = this.get_prototype().or_throw(&ctx)?.get("write")?;
                write.call::<_, Value>((This(this.0), chunk, rest))
            },
        )?,
    )?;

    let content_type_state = state.clone();
    stream.set(
        "setContentType",
        Function::new(ctx.clone(), move |ctx: Ctx<'js>, content_type: String| {
            let mut state = content_type_state.borrow_mut();
            if state.start_tx.is_none() {
                return Err(Exception::throw_message(
                    &ctx,
                    "Cannot set content-type, too late.",
                ));
            }
            HeaderValue::from_str(&content_type).or_throw_type(&ctx, "Invalid content-type")?;
            state.content_type = content_type;
            Ok(())
        })?,
    )?;

    Ok(stream)
}

/// Reports a handler failure in the stream's error trailers. The first
/// failure wins; after the stream has ended the error is only logged.
fn fail_response_stream<'js>(
    ctx: &Ctx<'js>,
    state: &RefCell<ResponseStreamState>,
    response_stream: &Object<'js>,
    error: &CaughtError<'js>,
    request_id: &str,
) -> Result<()> {
    if state.borrow().failed {
        return Ok(());
    }
    let open = state.borrow_mut().try_fail();
    let trailers = report_stream_error(ctx, error, request_id)?;
    if open {
        // Like Node.js's `addTrailers()` and `end()`: what the handler already
        // wrote goes out first, then `final` sends the trailers.
        state.borrow_mut().error_trailers = Some(trailers);
        let end: Function = response_stream.get("end")?;
        end.call::<_, ()>((This(response_stream.clone()),))?;
    }
    Ok(())
}

/// Ends an invocation that failed in the runtime itself, such as when the
/// Runtime API refuses the response. The error is always logged, goes in the
/// trailers if the body is still open, and destroys the stream so the handler
/// stops writing. A failure here has nowhere left to be reported.
fn abort_response_stream<'js>(
    ctx: &Ctx<'js>,
    state: &RefCell<ResponseStreamState>,
    response_stream: &Object<'js>,
    error: rquickjs::Error,
    request_id: &str,
) {
    let error = CaughtError::from_error(ctx, error);
    let trailers = report_stream_error(ctx, &error, request_id).catch(ctx).ok();
    {
        let mut state = state.borrow_mut();
        // The trailers of a handler failure whose `end` didn't finish go first.
        let trailers = match state.error_trailers.take() {
            Some(pending) => Some(pending),
            None => trailers.filter(|_| state.try_fail()),
        };
        state.failed = true;
        state.close(trailers);
    }
    let _ = error
        .into_value(ctx)
        .and_then(|error| {
            let destroy: Function = response_stream.get("destroy")?;
            destroy.call::<_, ()>((This(response_stream.clone()), error))
        })
        .catch(ctx);
}

fn closed_stream_error<'js>(ctx: &Ctx<'js>) -> Result<Exception<'js>> {
    Exception::from_message(ctx.clone(), "The Runtime API closed the response stream.")
}

/// Logs the error and returns the trailers that report it to Lambda.
fn report_stream_error<'js>(
    ctx: &Ctx<'js>,
    error: &CaughtError<'js>,
    request_id: &str,
) -> Result<HeaderMap> {
    let (error_type, error_object) = format_error(ctx, error, Some(request_id))?;
    log_error_object(ctx, &error_object)?;
    let error_body = json_stringify(ctx, error_object)?.unwrap_or_default();

    let mut trailers = HeaderMap::with_capacity(2);
    trailers.insert(
        HEADER_ERROR_TYPE.clone(),
        HeaderValue::from_str(&error_type).unwrap_or(HeaderValue::from_static("Error")),
    );
    trailers.insert(
        HEADER_ERROR_BODY.clone(),
        HeaderValue::from_str(&bytes_to_b64_string(error_body.as_bytes())).or_throw(ctx)?,
    );
    Ok(trailers)
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
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc, Arc,
        },
        thread,
        time::{Duration, Instant},
    };

    use hyper::header::CONTENT_TYPE;
    use rquickjs::CatchResultExt;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    use crate::libs::encoding::bytes_from_b64;
    use crate::runtime_client::{
        self, RuntimeConfig, ENV_RUNTIME_PATH, HEADER_INVOKED_FUNCTION_ARN, HEADER_REQUEST_ID,
    };
    use crate::vm::Vm;

    #[tokio::test]
    async fn runtime() {
        let mock_server = MockServer::start().await;

        fn uuid_v4() -> String {
            let mut bytes = [0u8; 8];
            for b in bytes.iter_mut() {
                *b = rand::random();
            }

            format!(
                "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
            )
        }

        Mock::given(matchers::method("GET"))
            .and(matchers::path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(matchers::method("GET"))
            .and(matchers::path(format!(
                "{}/invocation/next",
                ENV_RUNTIME_PATH
            )))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(&HEADER_REQUEST_ID, uuid_v4())
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
            println!("Testing {}", handler);
            let mock_config = RuntimeConfig {
                runtime_api: runtime_api.into(),
                handler: handler.into(),
                iterations: 10,
            };

            vm.ctx
                .async_with(async |ctx| {
                    ctx.globals()
                        .set("__MOCK_ENDPOINT", ["http://", runtime_api].concat())
                        .unwrap();
                    runtime_client::start_with_cfg(&ctx, mock_config)
                        .await
                        .catch(&ctx)
                        .unwrap()
                })
                .await;
        }

        run_with_handler(&vm, "../fixtures/handler.handler", &runtime_api).await;
        run_with_handler(&vm, "../fixtures/primitive-handler.handler", &runtime_api).await;
        run_with_handler(&vm, "../fixtures/throwing-handler.handler", &runtime_api).await;
        run_with_handler(&vm, "../fixtures/sdk-handler.handler", &runtime_api).await;
        run_with_handler(&vm, "../fixtures/tla-webcall-handler.handler", &runtime_api).await;
        run_with_handler(&vm, "../fixtures/cjs-handler.handler", &runtime_api).await;

        vm.runtime.idle().await;
    }

    /// A `/response` or `/error` request as it arrived on the wire.
    struct RecordedRequest {
        path: String,
        headers: Vec<(String, String)>,
        chunks: Vec<(Instant, Vec<u8>)>,
        trailers: Vec<(String, String)>,
    }

    impl RecordedRequest {
        fn header(&self, name: &str) -> Option<&str> {
            find_field(&self.headers, name)
        }

        fn trailer(&self, name: &str) -> Option<&str> {
            find_field(&self.trailers, name)
        }

        fn body(&self) -> String {
            let body: Vec<u8> = self.chunks.iter().flat_map(|(_, c)| c.clone()).collect();
            String::from_utf8(body).unwrap()
        }

        fn error_body(&self) -> String {
            let body = self.trailer("lambda-runtime-function-error-body").unwrap();
            String::from_utf8(bytes_from_b64(body.as_bytes()).unwrap()).unwrap()
        }
    }

    fn find_field<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
        fields
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// How the mock Runtime API answers a `/response`.
    #[derive(Clone, Copy)]
    enum Answer {
        /// 202 once the body has ended.
        Accept,
        /// Waits before reading the body, then 202.
        AcceptAfter(Duration),
        /// 413 as soon as the headers arrive, as when a response is over the
        /// size limit. The body is still read to the end.
        TooLarge,
    }

    const ACCEPTED: &[u8] = b"HTTP/1.1 202 Accepted\r\ncontent-length: 0\r\n\r\n";
    const TOO_LARGE: &[u8] =
        b"HTTP/1.1 413 Payload Too Large\r\ncontent-length: 21\r\n\r\nresponse is too large";
    // Lambda refuses /error once a response has started.
    const FORBIDDEN: &[u8] = b"HTTP/1.1 403 Forbidden\r\ncontent-length: 0\r\n\r\n";

    /// A minimal Runtime API on a raw socket, so tests can see chunk
    /// boundaries and trailers, which an HTTP server library would hide.
    fn start_runtime_api(
        event: &'static str,
        answer: Answer,
    ) -> (String, mpsc::Receiver<RecordedRequest>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        let invocations = Arc::new(AtomicUsize::new(0));
        thread::spawn(move || {
            for stream in listener.incoming() {
                let tx = tx.clone();
                let invocations = invocations.clone();
                thread::spawn(move || serve(stream.unwrap(), event, answer, &invocations, tx));
            }
        });
        (format!("127.0.0.1:{port}"), rx)
    }

    fn serve(
        stream: TcpStream,
        event: &str,
        answer: Answer,
        invocations: &AtomicUsize,
        tx: mpsc::Sender<RecordedRequest>,
    ) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            let mut request_line = line.split_whitespace();
            let method = request_line.next().unwrap().to_owned();
            let path = request_line.next().unwrap().to_owned();
            let headers = read_fields(&mut reader);

            if method == "GET" {
                let id = invocations.fetch_add(1, Ordering::SeqCst);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nlambda-runtime-aws-request-id: request-{id}\r\ncontent-length: {}\r\n\r\n{event}",
                    event.len()
                );
                writer.write_all(response.as_bytes()).unwrap();
                continue;
            }

            let is_response = path.ends_with("/response");
            let status = match (is_response, answer) {
                (false, _) => Some(FORBIDDEN),
                (true, Answer::Accept) => Some(ACCEPTED),
                (true, Answer::AcceptAfter(delay)) => {
                    thread::sleep(delay);
                    Some(ACCEPTED)
                },
                (true, Answer::TooLarge) => {
                    writer.write_all(TOO_LARGE).unwrap();
                    None
                },
            };

            let mut chunks = Vec::new();
            let mut trailers = Vec::new();
            if find_field(&headers, "transfer-encoding") == Some("chunked") {
                loop {
                    line.clear();
                    // The client may close the connection mid-body once it
                    // has the answer to a refused response.
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    let size = usize::from_str_radix(line.trim(), 16).unwrap();
                    if size == 0 {
                        trailers = read_fields(&mut reader);
                        break;
                    }
                    let mut chunk = vec![0; size + 2];
                    if reader.read_exact(&mut chunk).is_err() {
                        break;
                    }
                    chunk.truncate(size);
                    chunks.push((Instant::now(), chunk));
                }
            } else if let Some(len) = find_field(&headers, "content-length") {
                let mut body = vec![0; len.parse().unwrap()];
                reader.read_exact(&mut body).unwrap();
                chunks.push((Instant::now(), body));
            }

            if let Some(status) = status {
                let _ = writer.write_all(status);
            }
            tx.send(RecordedRequest {
                path,
                headers,
                chunks,
                trailers,
            })
            .unwrap();
        }
    }

    fn read_fields(reader: &mut impl BufRead) -> Vec<(String, String)> {
        let mut fields = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            let Some((name, value)) = line.trim_end().split_once(':') else {
                return fields;
            };
            fields.push((name.to_owned(), value.trim().to_owned()));
        }
    }

    /// Runs one invocation of a streaming fixture handler and returns what the
    /// runtime sent back.
    async fn invoke_streaming_handler(handler: &str) -> RecordedRequest {
        let mut requests = invoke_streaming_handler_with(handler, Answer::Accept, 1).await;
        assert_eq!(requests.len(), 1);
        requests.remove(0)
    }

    /// Runs `iterations` invocations of a streaming fixture handler and
    /// returns every request the runtime posted.
    async fn invoke_streaming_handler_with(
        handler: &str,
        answer: Answer,
        iterations: usize,
    ) -> Vec<RecordedRequest> {
        let (runtime_api, requests) = start_runtime_api(r#"{"name":"world"}"#, answer);
        let vm = Vm::new().await.unwrap();
        let config = RuntimeConfig {
            runtime_api,
            handler: ["../fixtures/streaming-handler.", handler].concat(),
            iterations,
        };
        vm.ctx
            .async_with(async |ctx| {
                runtime_client::start_with_cfg(&ctx, config)
                    .await
                    .catch(&ctx)
                    .unwrap()
            })
            .await;
        vm.runtime.idle().await;

        // When the Runtime API answers before the body ends, the runtime moves
        // on while the connection is still sending it. Waiting without blocking
        // lets that finish; the extra 200 ms catches any unexpected request.
        let mut deadline = Instant::now() + Duration::from_secs(5);
        let mut recorded = Vec::new();
        while recorded.len() < iterations || Instant::now() < deadline {
            match requests.try_recv() {
                Ok(req) => {
                    recorded.push(req);
                    if recorded.len() == iterations {
                        deadline = Instant::now() + Duration::from_millis(200);
                    }
                },
                Err(_) if Instant::now() < deadline => {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                },
                Err(err) => panic!("expected {iterations} requests: {err}"),
            }
        }
        recorded
    }

    #[tokio::test]
    async fn streaming_response() {
        let req = invoke_streaming_handler("handler").await;

        assert_eq!(
            req.path,
            format!("/{ENV_RUNTIME_PATH}/invocation/request-0/response")
        );
        assert_eq!(
            req.header("lambda-runtime-function-response-mode"),
            Some("streaming")
        );
        assert_eq!(req.header("transfer-encoding"), Some("chunked"));
        assert_eq!(req.header("connection"), Some("close"));
        assert_eq!(
            req.header("trailer"),
            Some("Lambda-Runtime-Function-Error-Type, Lambda-Runtime-Function-Error-Body")
        );
        assert_eq!(req.header("content-type"), Some("text/plain"));
        assert_eq!(req.body(), "Hello, world!");
        assert!(req.trailers.is_empty());

        // The first chunk arrived before the handler's 300 ms wait ended
        let chunks: Vec<&[u8]> = req.chunks.iter().map(|(_, c)| c.as_slice()).collect();
        assert_eq!(chunks, [&b"Hello, "[..], b"world", b"!"]);
        let first = req.chunks.first().unwrap().0;
        let last = req.chunks.last().unwrap().0;
        assert!(last.duration_since(first) >= Duration::from_millis(150));
    }

    #[tokio::test]
    async fn streaming_response_through_pipeline() {
        let req = invoke_streaming_handler("pipelineHandler").await;

        assert_eq!(req.header("content-type"), Some("application/octet-stream"));
        assert_eq!(req.body(), "highWaterMark=4;abc");
        assert!(req.trailers.is_empty());
    }

    #[tokio::test]
    async fn streaming_response_ends_after_handler_returns() {
        let req = invoke_streaming_handler("endsAfterReturn").await;

        assert_eq!(req.body(), "ended later");
        assert!(req.trailers.is_empty());
    }

    #[tokio::test]
    async fn streaming_response_content_type_after_write() {
        let req = invoke_streaming_handler("lateContentType").await;

        assert_eq!(req.header("content-type"), Some("application/octet-stream"));
        assert_eq!(req.body(), "Cannot set content-type, too late.");
    }

    #[tokio::test]
    async fn streaming_response_error_after_write() {
        let req = invoke_streaming_handler("throwsAfterWrite").await;

        assert!(req.path.ends_with("/response"));
        assert_eq!(req.body(), "partial");
        assert_eq!(
            req.trailer("lambda-runtime-function-error-type"),
            Some("Error")
        );
        let error_body = req.error_body();
        assert!(
            error_body.contains(r#""errorMessage":"kaboom""#),
            "{error_body}"
        );
        assert!(
            error_body.contains(r#""requestId":"request-0""#),
            "{error_body}"
        );
    }

    #[tokio::test]
    async fn streaming_response_error_before_write() {
        let req = invoke_streaming_handler("throwsBeforeWrite").await;

        assert!(req.path.ends_with("/response"));
        assert_eq!(req.body(), "");
        assert!(req.error_body().contains(r#""errorMessage":"kaboom""#));
    }

    #[tokio::test]
    async fn streaming_response_requires_async_handler() {
        let req = invoke_streaming_handler("notAsync").await;

        assert_eq!(req.body(), "partial");
        assert!(req
            .error_body()
            .contains("Streaming does not support non-async handlers."));
    }

    #[tokio::test]
    async fn streaming_response_destroyed_by_pipeline() {
        let req = invoke_streaming_handler("pipelineSourceFails").await;

        assert!(req.path.ends_with("/response"));
        assert!(req
            .error_body()
            .contains(r#""errorMessage":"source failed""#));
    }

    #[tokio::test]
    async fn streaming_response_runtime_failure() {
        // Reporting the handler's error fails, and the runtime still sends it
        // and goes on to the next invocation.
        let requests = invoke_streaming_handler_with("endThrows", Answer::Accept, 2).await;

        assert_eq!(requests.len(), 2);
        for req in &requests {
            assert!(req.path.ends_with("/response"));
            assert_eq!(req.body(), "partial");
            let error_body = req.error_body();
            assert!(
                error_body.contains(r#""errorMessage":"kaboom""#),
                "{error_body}"
            );
        }
    }

    #[tokio::test]
    async fn streaming_response_writes_json() {
        let req = invoke_streaming_handler("writesJson").await;

        assert_eq!(req.body(), r#"{"a":1}null[1,2]bytes!"#);
    }

    #[tokio::test]
    async fn streaming_response_waits_for_slow_reader() {
        // The Runtime API reads nothing for 1.5 s, so once the socket and
        // hyper's buffers are full, 'drain' has to wait for it.
        let delay = Duration::from_millis(1500);
        let mut requests =
            invoke_streaming_handler_with("waitsForDrain", Answer::AcceptAfter(delay), 1).await;
        assert_eq!(requests.len(), 1);
        let body = requests.remove(0).body();

        let (data, summary) = body.split_once(';').unwrap();
        let summary: Vec<usize> = summary
            .split(';')
            .map(|field| field.split_once('=').unwrap().1.parse().unwrap())
            .collect();
        let [writes, falses, longest_wait_ms] = summary[..] else {
            panic!("unexpected summary: {summary:?}");
        };
        assert!(falses > 0, "write() never returned false");
        assert!(
            longest_wait_ms >= 500,
            "no 'drain' waited for the reader: {writes} writes, longest wait {longest_wait_ms} ms"
        );
        assert_eq!(data.len(), writes * 1024 * 1024);
    }

    #[tokio::test]
    async fn streaming_response_refused_by_runtime_api() {
        let mut requests =
            invoke_streaming_handler_with("writesUntilDestroyed", Answer::TooLarge, 2).await;

        // Both invocations posted only their response: none fell back to
        // /error, which Lambda refuses once a response has started, and the
        // runtime went on to the next invocation.
        requests.sort_by(|a, b| a.path.cmp(&b.path));
        let paths: Vec<&str> = requests.iter().map(|req| req.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                format!("/{ENV_RUNTIME_PATH}/invocation/request-0/response"),
                format!("/{ENV_RUNTIME_PATH}/invocation/request-1/response"),
            ]
        );
        for req in &requests {
            // The runtime destroyed the stream, so the handler stopped writing
            assert!(!req.body().ends_with("never destroyed"));
            let error_body = req.error_body();
            assert!(
                error_body
                    .contains("Unexpected /invocation/response response: response is too large"),
                "{error_body}"
            );
        }
    }
}
