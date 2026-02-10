// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc, sync::RwLock};

use llrt_abort::AbortSignal;
use llrt_http::Agent;
use llrt_json::parse::json_parse;
use llrt_stream_web::{
    readable_stream_default_controller_close_stream,
    readable_stream_default_controller_enqueue_value, CancelAlgorithm, PullAlgorithm,
    ReadableStream, ReadableStreamControllerClass, ReadableStreamDefaultControllerClass,
};
use llrt_url::{url_class::URL, url_search_params::URLSearchParams};
use llrt_utils::{
    bytes::ObjectBytes, object::ObjectExt, primordials::Primordial, result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, ArrayBuffer, Class, Ctx, Exception, FromJs,
    IntoJs, Null, Object, Result, TypedArray, Value,
};

use super::{
    headers::{Headers, HeadersGuard, HEADERS_KEY_CONTENT_TYPE},
    strip_bom, Blob, FormData, MIME_TYPE_FORM_DATA, MIME_TYPE_FORM_URLENCODED,
    MIME_TYPE_OCTET_STREAM, MIME_TYPE_TEXT,
};

#[derive(Clone, Default, PartialEq)]
pub enum RequestMode {
    #[default]
    Cors,
    NoCors,
    SameOrigin,
    Navigate,
}

impl TryFrom<String> for RequestMode {
    type Error = String;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "cors" => RequestMode::Cors,
            "no-cors" => RequestMode::NoCors,
            "same-origin" => RequestMode::SameOrigin,
            "navigate" => RequestMode::Navigate,
            _ => return Err(["Invalid requrest mode: ", s.as_str()].concat()),
        })
    }
}

impl RequestMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Cors => "cors",
            Self::NoCors => "no-cors",
            Self::SameOrigin => "same-origin",
            Self::Navigate => "navigate",
        }
    }
}

#[allow(dead_code)]
#[derive(rquickjs::JsLifetime)]
enum BodyVariant<'js> {
    Provided(Option<Value<'js>>),
    Empty,
}

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct Request<'js> {
    url: String,
    method: String,
    headers: Option<Class<'js, Headers>>,
    body: RwLock<BodyVariant<'js>>,
    body_stream: RefCell<Option<Value<'js>>>,
    signal: Option<Class<'js, AbortSignal<'js>>>,
    mode: RequestMode,
    keepalive: bool,
    agent: Option<Class<'js, Agent>>,
}

impl<'js> Trace<'js> for Request<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        if let Some(headers) = &self.headers {
            headers.trace(tracer);
        }
        let body = self.body.read().unwrap();
        if let BodyVariant::Provided(Some(body)) = &*body {
            body.trace(tracer);
        }
        if let Some(stream) = self.body_stream.borrow().as_ref() {
            stream.trace(tracer);
        }
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Request<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, options: Opt<Value<'js>>) -> Result<Self> {
        let mut request = Self {
            url: "".into(),
            method: "GET".into(),
            headers: None,
            body: RwLock::new(BodyVariant::Empty),
            body_stream: RefCell::new(None),
            signal: None,
            mode: RequestMode::Cors,
            keepalive: false,
            agent: None,
        };

        if input.is_string() {
            request.url = input.get()?;
        } else if let Ok(url) = URL::from_js(&ctx, input.clone()) {
            request.url = url.to_string();
        } else if input.is_object() {
            let obj = unsafe { input.as_object().unwrap_unchecked() };
            // Check if input is a Request - if so, transfer body (mark original as used)
            if let Some(input_request) = Class::<Request>::from_object(obj) {
                let input_req = input_request.borrow_mut();
                request.url = input_req.url.clone();
                request.method = input_req.method.clone();
                request.mode = input_req.mode.clone();
                request.keepalive = input_req.keepalive;
                request.signal = input_req.signal.clone();
                request.agent = input_req.agent.clone();
                if let Some(headers) = &input_req.headers {
                    request.headers = Some(Class::<Headers>::instance(
                        ctx.clone(),
                        headers.borrow().clone(),
                    )?);
                }
                // Transfer body - take from original, marking it as used
                let mut body_guard = input_req.body.write().unwrap();
                if let BodyVariant::Provided(ref mut provided) = &mut *body_guard {
                    if let Some(body_value) = provided.take() {
                        request.body = RwLock::new(BodyVariant::Provided(Some(body_value)));
                    }
                }
                drop(body_guard);
                drop(input_req);
            } else {
                assign_request(&mut request, ctx.clone(), obj)?;
            }
        }
        if let Some(options) = options.0 {
            if let Some(obj) = options.as_object() {
                assign_request(&mut request, ctx.clone(), obj)?;
            }
        }
        if request.headers.is_none() {
            let headers = Class::instance(ctx, Headers::default())?;
            request.headers = Some(headers);
        }

        Ok(request)
    }

    #[qjs(get)]
    fn url(&self) -> String {
        self.url.clone()
    }

    #[qjs(get)]
    fn method(&self) -> String {
        self.method.clone()
    }

    #[qjs(get)]
    fn headers(&self) -> Option<Class<'js, Headers>> {
        self.headers.clone()
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(Request)
    }

    #[qjs(get)]
    fn body(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        // Return cached stream if available
        if let Some(stream) = self.body_stream.borrow().as_ref() {
            return Ok(stream.clone());
        }

        let body_guard = self.body.read().unwrap();
        match &*body_guard {
            BodyVariant::Empty => Ok(Value::new_null(ctx)),
            BodyVariant::Provided(None) => Ok(Value::new_null(ctx)),
            BodyVariant::Provided(Some(value)) => {
                // If already a ReadableStream, return it directly
                if let Some(stream) = value
                    .as_object()
                    .and_then(Class::<ReadableStream>::from_object)
                {
                    return Ok(stream.into_value());
                }

                // Create a ReadableStream that yields the body data once
                let body_value = value.clone();
                drop(body_guard);

                let stream = create_body_stream(&ctx, body_value)?;
                *self.body_stream.borrow_mut() = Some(stream.clone());
                Ok(stream)
            },
        }
    }

    #[qjs(get)]
    fn keepalive(&self) -> bool {
        self.keepalive
    }

    #[qjs(get)]
    fn signal(&self) -> Option<Class<'js, AbortSignal<'js>>> {
        self.signal.clone()
    }

    #[qjs(get)]
    fn body_used(&self) -> bool {
        // Check if body stream is disturbed (has been read from)
        if let Some(stream_value) = self.body_stream.borrow().as_ref() {
            if let Some(stream) = stream_value
                .as_object()
                .and_then(Class::<ReadableStream>::from_object)
            {
                if let Ok(disturbed) = stream.get::<_, bool>("disturbed") {
                    if disturbed {
                        return true;
                    }
                }
            }
        }

        let body = self.body.read().unwrap();
        match &*body {
            BodyVariant::Provided(value) => value.is_none(),
            BodyVariant::Empty => false,
        }
    }

    #[qjs(get)]
    fn mode(&self) -> &str {
        self.mode.as_str()
    }

    #[qjs(get)]
    fn cache(&self) -> &'static str {
        "no-store"
    }

    #[qjs(get)]
    fn agent(&self) -> Option<Class<'js, Agent>> {
        self.agent.clone()
    }

    pub async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(String::from_utf8_lossy(&strip_bom(bytes)).to_string());
        }
        Ok("".into())
    }

    pub async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return json_parse(&ctx, strip_bom(bytes));
        }
        Err(Exception::throw_syntax(&ctx, "JSON input is empty"))
    }

    async fn array_buffer(&mut self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return ArrayBuffer::new(ctx, bytes);
        }
        ArrayBuffer::new(ctx, Vec::<u8>::new())
    }

    async fn bytes(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return TypedArray::new(ctx, bytes).map(|m| m.into_value());
        }
        TypedArray::new(ctx, Vec::<u8>::new()).map(|m| m.into_value())
    }

    async fn blob(&mut self, ctx: Ctx<'js>) -> Result<Blob> {
        let mime_type = self.get_header_value(&ctx, HEADERS_KEY_CONTENT_TYPE)?;

        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(Blob::from_bytes(bytes, mime_type));
        }
        Ok(Blob::from_bytes(Vec::<u8>::new(), mime_type))
    }

    async fn form_data(&self, ctx: Ctx<'js>) -> Result<FormData> {
        let mime_type = self
            .get_header_value(&ctx, HEADERS_KEY_CONTENT_TYPE)?
            .unwrap_or(MIME_TYPE_OCTET_STREAM.into());

        if let Some(bytes) = self.take_bytes(&ctx).await? {
            let form_data = FormData::from_multipart_bytes(&ctx, &mime_type, bytes)?;
            return Ok(form_data);
        }
        Ok(FormData::default())
    }

    fn clone(&mut self, ctx: Ctx<'js>) -> Result<Self> {
        let headers = if let Some(headers) = &self.headers {
            Some(Class::<Headers>::instance(
                ctx.clone(),
                headers.borrow().clone(),
            )?)
        } else {
            None
        };

        let body_guard = self.body.read().unwrap();
        let cloned_body = match &*body_guard {
            BodyVariant::Provided(provided) => {
                // Cannot clone if body has been consumed (bodyUsed=true)
                if provided.is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Cannot clone request after body has been used",
                    ));
                }
                // For ReadableStream bodies, use tee() to create two branches
                if let Some(ref value) = provided {
                    if let Some(stream) = value
                        .as_object()
                        .and_then(Class::<ReadableStream>::from_object)
                    {
                        let disturbed: bool = stream.get("disturbed").unwrap_or(false);
                        if disturbed {
                            return Err(Exception::throw_type(
                                &ctx,
                                "Cannot clone request with disturbed body",
                            ));
                        }
                        let locked: bool = stream.get("locked").unwrap_or(false);
                        if locked {
                            return Err(Exception::throw_type(
                                &ctx,
                                "Cannot clone request with locked body",
                            ));
                        }

                        // Use tee() to split the stream
                        let tee_fn: rquickjs::Function = stream.get("tee")?;
                        let branches: rquickjs::Array =
                            tee_fn.call((rquickjs::function::This(stream.clone()),))?;
                        let branch1: Value = branches.get(0)?;
                        let branch2: Value = branches.get(1)?;

                        // Update original body to use branch1
                        drop(body_guard);
                        let mut body_write = self.body.write().unwrap();
                        *body_write = BodyVariant::Provided(Some(branch1));
                        // Clear cached body stream since we changed the body
                        *self.body_stream.borrow_mut() = None;
                        drop(body_write);

                        // Return clone with branch2
                        return Ok(Self {
                            url: self.url.clone(),
                            method: self.method.clone(),
                            headers,
                            body: RwLock::new(BodyVariant::Provided(Some(branch2))),
                            body_stream: RefCell::new(None),
                            signal: self.signal.clone(),
                            mode: self.mode.clone(),
                            keepalive: self.keepalive,
                            agent: self.agent.clone(),
                        });
                    }
                }
                BodyVariant::Provided(provided.clone())
            },
            BodyVariant::Empty => BodyVariant::Empty,
        };
        drop(body_guard);

        Ok(Self {
            url: self.url.clone(),
            method: self.method.clone(),
            headers,
            body: RwLock::new(cloned_body),
            body_stream: RefCell::new(None),
            signal: self.signal.clone(),
            mode: self.mode.clone(),
            keepalive: self.keepalive,
            agent: self.agent.clone(),
        })
    }
}

impl<'js> Request<'js> {
    #[allow(clippy::await_holding_lock)]
    #[allow(clippy::readonly_write_lock)]
    async fn take_bytes(&self, ctx: &Ctx<'js>) -> Result<Option<Vec<u8>>> {
        // Check if body stream is locked (user got a reader from req.body)
        if let Some(stream_val) = self.body_stream.borrow().as_ref() {
            if let Some(_stream) = stream_val
                .as_object()
                .and_then(Class::<ReadableStream>::from_object)
            {
                let locked: bool = stream_val
                    .as_object()
                    .unwrap()
                    .get("locked")
                    .unwrap_or(false);
                if locked {
                    return Err(Exception::throw_type(
                        ctx,
                        "Cannot read body: stream is locked",
                    ));
                }
            }
        }

        let mut body_guard = self.body.write().unwrap();
        let body = &mut *body_guard;
        let bytes = match body {
            BodyVariant::Provided(provided) => {
                let provided = provided
                    .take()
                    .ok_or(Exception::throw_message(ctx, "Already read"))?;
                drop(body_guard);

                // Check if it's a ReadableStream
                if let Some(stream) = provided
                    .as_object()
                    .and_then(Class::<ReadableStream>::from_object)
                {
                    return collect_readable_stream(ctx, &stream).await.map(Some);
                }

                if let Some(blob) = provided.as_object().and_then(Class::<Blob>::from_object) {
                    blob.borrow().get_bytes()
                } else {
                    let bytes = ObjectBytes::from(ctx, &provided)?;
                    bytes.as_bytes(ctx)?.to_vec()
                }
            },
            BodyVariant::Empty => return Ok(None),
        };

        Ok(Some(bytes))
    }

    fn get_headers(&self, ctx: &Ctx<'js>) -> Result<Option<Headers>> {
        self.headers()
            .map(|headers| Headers::from_js(ctx, headers.into_value()))
            .transpose()
            .or_throw(ctx)
    }

    fn get_header_value(&self, ctx: &Ctx<'js>, key: &str) -> Result<Option<String>> {
        Ok(self.get_headers(ctx)?.and_then(|headers| {
            headers
                .iter()
                .find_map(|(k, v)| (k == key).then(|| v.to_string()))
        }))
    }
}

/// Collects all data from a ReadableStream into a Vec<u8>
async fn collect_readable_stream<'js>(
    _ctx: &Ctx<'js>,
    stream: &Class<'js, ReadableStream<'js>>,
) -> Result<Vec<u8>> {
    crate::collect_readable_stream(stream).await
}

fn assign_request<'js>(request: &mut Request<'js>, ctx: Ctx<'js>, obj: &Object<'js>) -> Result<()> {
    if let Some(url) = obj.get_optional("url")? {
        request.url = url;
    }
    if let Some(method) = obj.get_optional::<_, String>("method")? {
        let method = method.to_ascii_uppercase();
        if let "CONNECT" | "TRACE" | "TRACK" = method.as_str() {
            return Err(Exception::throw_type(&ctx, "This method is not allowed."));
        }
        request.method = method;
    }
    if let Some(mode) = obj.get_optional::<_, String>("mode")? {
        request.mode = mode.try_into().or_throw(&ctx)?;
    }
    if let Some(keepalive) = obj.get_optional::<_, Value>("keepalive")? {
        request.keepalive = if let Some(b) = keepalive.as_bool() {
            b
        } else if let Some(n) = keepalive.as_number() {
            n != 0.0
        } else {
            false
        }
    }

    if let Some(signal) = obj.get_optional::<_, Value>("signal")? {
        if !signal.is_undefined() && !signal.is_null() {
            let signal = AbortSignal::from_js(&ctx, signal).map_err(|_| {
                Exception::throw_type(
                    &ctx,
                    "Failed to construct 'Request': 'signal' property is not an AbortSignal",
                )
            })?;
            request.signal = Some(Class::instance(ctx.clone(), signal)?);
        }
    }

    let mut content_type: Option<String> = None;

    if let Some(body) = obj.get_optional::<_, Value>("body")? {
        if !body.is_undefined() && !body.is_null() {
            if let "GET" | "HEAD" = request.method.as_str() {
                return Err(Exception::throw_type(
                    &ctx,
                    "Failed to construct 'Request': Request with GET/HEAD method cannot have body.",
                ));
            }

            let body = if body.is_string() {
                content_type = Some(MIME_TYPE_TEXT.into());
                BodyVariant::Provided(Some(body))
            } else if let Some(body_obj) = body.as_object() {
                // Check if it's a ReadableStream
                if let Some(stream) = Class::<ReadableStream>::from_object(body_obj) {
                    let stream_ref = stream.borrow();
                    // Check if stream is disturbed
                    if stream_ref.disturbed {
                        return Err(Exception::throw_type(
                            &ctx,
                            "Cannot construct Request with a disturbed ReadableStream",
                        ));
                    }
                    // Check if stream is locked (reader.is_some())
                    let locked: bool = body_obj.get("locked").unwrap_or(false);
                    drop(stream_ref);
                    if locked {
                        return Err(Exception::throw_type(
                            &ctx,
                            "Cannot construct Request with a locked ReadableStream",
                        ));
                    }
                    BodyVariant::Provided(Some(body))
                } else if let Some(blob) = Class::<Blob>::from_object(body_obj) {
                    let blob = blob.borrow();
                    if !blob.mime_type().is_empty() {
                        content_type = Some(blob.mime_type());
                    }
                    BodyVariant::Provided(Some(body))
                } else if let Some(fd) = Class::<FormData>::from_object(body_obj) {
                    let fd = fd.borrow();
                    let (multipart_body, boundary) = fd.to_multipart_bytes(&ctx)?;
                    content_type = Some([MIME_TYPE_FORM_DATA, &boundary].concat());
                    BodyVariant::Provided(Some(multipart_body.into_js(&ctx)?))
                } else if body_obj.instance_of::<URLSearchParams>() {
                    content_type = Some(MIME_TYPE_FORM_URLENCODED.into());
                    BodyVariant::Provided(Some(body))
                } else {
                    BodyVariant::Provided(Some(body))
                }
            } else {
                BodyVariant::Provided(Some(body))
            };
            request.body = RwLock::new(body);
        }
    }

    let headers = {
        let guard = if request.mode == RequestMode::NoCors {
            HeadersGuard::RequestNoCors
        } else {
            HeadersGuard::Request
        };
        let mut headers = Headers::from_value(
            &ctx,
            obj.get_optional("headers")?
                .unwrap_or_else(|| Null.into_js(&ctx).unwrap()),
            guard,
        )?;
        if !headers.has(ctx.clone(), HEADERS_KEY_CONTENT_TYPE.into())? {
            if let Some(value) = content_type {
                headers.set(
                    ctx.clone(),
                    HEADERS_KEY_CONTENT_TYPE.into(),
                    value.into_js(&ctx)?,
                )?;
            }
        }
        Class::instance(ctx, headers)?
    };
    request.headers = Some(headers);

    if let Some(agent_opt) = obj.get_optional("agent")? {
        request.agent = Some(agent_opt);
    }

    Ok(())
}

/// Creates a ReadableStream from a body value (string, Blob, ArrayBuffer, etc.)
fn create_body_stream<'js>(ctx: &Ctx<'js>, body_value: Value<'js>) -> Result<Value<'js>> {
    use llrt_stream_web::utils::promise::PromisePrimordials;

    let body_data: Rc<RefCell<Option<Value<'js>>>> = Rc::new(RefCell::new(Some(body_value)));

    let pull = PullAlgorithm::from_fn(
        move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
            let body_data = body_data.clone();

            let ctrl_class: ReadableStreamDefaultControllerClass = match controller {
                ReadableStreamControllerClass::ReadableStreamDefaultController(c) => c,
                _ => return Err(Exception::throw_type(&ctx, "Expected default controller")),
            };

            // Take body data (only yields once)
            let data = body_data.borrow_mut().take();

            if let Some(value) = data {
                // Convert to bytes synchronously
                let bytes =
                    if let Some(blob) = value.as_object().and_then(Class::<Blob>::from_object) {
                        blob.borrow().get_bytes()
                    } else {
                        ObjectBytes::from(&ctx, &value)?.as_bytes(&ctx)?.to_vec()
                    };

                let array = TypedArray::<u8>::new(ctx.clone(), bytes)?;
                readable_stream_default_controller_enqueue_value(
                    ctx.clone(),
                    ctrl_class.clone(),
                    array.into_value(),
                )?;
                readable_stream_default_controller_close_stream(ctx.clone(), ctrl_class)?;
            } else {
                readable_stream_default_controller_close_stream(ctx.clone(), ctrl_class)?;
            }

            // Return resolved promise
            let primordials = PromisePrimordials::get(&ctx)?;
            Ok(primordials.promise_resolved_with_undefined.clone())
        },
    );

    let stream = ReadableStream::from_pull_algorithm(
        ctx.clone(),
        pull,
        CancelAlgorithm::ReturnPromiseUndefined,
    )?;

    Ok(stream.into_value())
}
