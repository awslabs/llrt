// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
    sync::RwLock,
    time::Instant,
};

use crate::decompress::StreamingDecoder;
use either::Either;
use http_body_util::BodyExt;
use hyper::{body::Incoming, header::HeaderName};
use llrt_abort::AbortSignal;
use llrt_json::{parse::json_parse, stringify::json_stringify};
use llrt_stream_web::ReadableStream;
use llrt_url::{url_class::URL, url_search_params::URLSearchParams};
use llrt_utils::{bytes::ObjectBytes, mc_oneshot};
use once_cell::sync::Lazy;
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    function::Opt,
    ArrayBuffer, Class, Coerced, Ctx, Exception, IntoJs, JsLifetime, Object, Result, TypedArray,
    Value,
};

use llrt_stream_web::{
    readable_stream_default_controller_close_stream,
    readable_stream_default_controller_enqueue_value,
    readable_stream_default_controller_error_stream, utils::promise::upon_promise_fulfilment,
    CancelAlgorithm, PullAlgorithm, ReadableStreamControllerClass,
    ReadableStreamDefaultControllerClass,
};

use super::{
    headers::{Headers, HeadersGuard, HEADERS_KEY_CONTENT_TYPE},
    strip_bom, Blob, FormData, MIME_TYPE_FORM_DATA, MIME_TYPE_FORM_URLENCODED, MIME_TYPE_JSON,
    MIME_TYPE_OCTET_STREAM, MIME_TYPE_TEXT,
};

static STATUS_TEXTS: Lazy<HashMap<u16, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(100, "Continue");
    map.insert(101, "Switching Protocols");
    map.insert(102, "Processing");
    map.insert(103, "Early Hints");
    map.insert(200, "OK");
    map.insert(201, "Created");
    map.insert(202, "Accepted");
    map.insert(203, "Non-Authoritative Information");
    map.insert(204, "No Content");
    map.insert(205, "Reset Content");
    map.insert(206, "Partial Content");
    map.insert(207, "Multi-Status");
    map.insert(208, "Already Reported");
    map.insert(226, "IM Used");
    map.insert(300, "Multiple Choices");
    map.insert(301, "Moved Permanently");
    map.insert(302, "Found");
    map.insert(303, "See Other");
    map.insert(304, "Not Modified");
    map.insert(305, "Use Proxy");
    map.insert(307, "Temporary Redirect");
    map.insert(308, "Permanent Redirect");
    map.insert(400, "Bad Request");
    map.insert(401, "Unauthorized");
    map.insert(402, "Payment Required");
    map.insert(403, "Forbidden");
    map.insert(404, "Not Found");
    map.insert(405, "Method Not Allowed");
    map.insert(406, "Not Acceptable");
    map.insert(407, "Proxy Authentication Required");
    map.insert(408, "Request Timeout");
    map.insert(409, "Conflict");
    map.insert(410, "Gone");
    map.insert(411, "Length Required");
    map.insert(412, "Precondition Failed");
    map.insert(413, "Payload Too Large");
    map.insert(414, "URI Too Long");
    map.insert(415, "Unsupported Media Type");
    map.insert(416, "Range Not Satisfiable");
    map.insert(417, "Expectation Failed");
    map.insert(418, "I'm a teapot");
    map.insert(421, "Misdirected Request");
    map.insert(422, "Unprocessable Content");
    map.insert(423, "Locked");
    map.insert(424, "Failed Dependency");
    map.insert(425, "Too Early");
    map.insert(426, "Upgrade Required");
    map.insert(428, "Precondition Required");
    map.insert(429, "Too Many Requests");
    map.insert(431, "Request Header Fields Too Large");
    map.insert(451, "Unavailable For Legal Reasons");
    map.insert(500, "Internal Server Error");
    map.insert(501, "Not Implemented");
    map.insert(502, "Bad Gateway");
    map.insert(503, "Service Unavailable");
    map.insert(504, "Gateway Timeout");
    map.insert(505, "HTTP Version Not Supported");
    map.insert(506, "Variant Also Negotiates");
    map.insert(507, "Insufficient Storage");
    map.insert(508, "Loop Detected");
    map.insert(510, "Not Extended");
    map.insert(511, "Network Authentication Required");

    map
});

enum BodyVariant<'js> {
    /// Raw incoming HTTP body - consumed directly for text()/json()/etc
    Incoming(Rc<RefCell<Option<Incoming>>>, Option<String>), // body + content_encoding
    /// User-provided body value
    Provided(Option<Value<'js>>),
    /// Empty body
    Empty,
}

#[rquickjs::class]
pub struct Response<'js> {
    body: RwLock<BodyVariant<'js>>,
    /// Cached ReadableStream for the body getter (created lazily)
    body_stream: RefCell<Option<Value<'js>>>,
    method: String,
    url: String,
    start: Instant,
    status: u16,
    status_text: Option<String>,
    redirected: bool,
    headers: Class<'js, Headers>,
    abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
}

impl<'js> Trace<'js> for Response<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.headers.trace(tracer);
        if let Ok(body) = self.body.read() {
            if let BodyVariant::Provided(Some(body)) = &*body {
                body.trace(tracer);
            }
        }
        if let Some(stream) = self.body_stream.borrow().as_ref() {
            stream.trace(tracer);
        }
    }
}

unsafe impl<'js> JsLifetime<'js> for Response<'js> {
    type Changed<'to> = Response<'to>;
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Response<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, body: Opt<Value<'js>>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut url = "".into();
        let mut status: u16 = 200;
        let mut headers = None;
        let mut status_text = None;
        let mut abort_receiver = None;

        if let Some(opt) = options.0 {
            if let Some(url_opt) = opt.get("url")? {
                url = url_opt;
            }
            if let Some(status_opt) = opt.get::<_, Option<u16>>("status")? {
                status = status_opt;
            }
            if let Some(headers_opt) = opt.get("headers")? {
                headers = Some(Headers::from_value(
                    &ctx,
                    headers_opt,
                    HeadersGuard::Response,
                )?);
            }
            if let Some(status_text_opt) = opt.get("statusText")? {
                status_text = Some(status_text_opt);
            }

            if let Some(signal) = opt.get::<_, Option<Class<AbortSignal>>>("signal")? {
                abort_receiver = Some(signal.borrow().sender.subscribe())
            }
        }

        // Validate status range (200-599 per WHATWG Fetch spec)
        if !(200..=599).contains(&status) {
            return Err(Exception::throw_range(
                &ctx,
                &format!("Invalid status code: {}", status),
            ));
        }

        let has_body = body
            .0
            .as_ref()
            .is_some_and(|b| !b.is_null() && !b.is_undefined());

        // Null body status check (204, 304 per WHATWG Fetch spec)
        if has_body && (status == 204 || status == 304) {
            return Err(Exception::throw_type(
                &ctx,
                "Response with null body status cannot have body",
            ));
        }

        let mut content_type: Option<String> = None;

        let body = body
            .0
            .and_then(|body| {
                if body.is_null() || body.is_undefined() {
                    None
                } else if body.is_string() {
                    content_type = Some(MIME_TYPE_TEXT.into());
                    Some(Ok(BodyVariant::Provided(Some(body))))
                } else if let Some(obj) = body.as_object() {
                    // Check if it's a ReadableStream
                    if let Some(stream) = Class::<ReadableStream>::from_object(obj) {
                        let stream_ref = stream.borrow();
                        if stream_ref.disturbed {
                            return Some(Err(Exception::throw_type(
                                &ctx,
                                "Cannot construct Response with a disturbed ReadableStream",
                            )));
                        }
                        if stream_ref.is_readable_stream_locked() {
                            return Some(Err(Exception::throw_type(
                                &ctx,
                                "Cannot construct Response with a locked ReadableStream",
                            )));
                        }
                        drop(stream_ref);
                        Some(Ok(BodyVariant::Provided(Some(body))))
                    } else if let Some(blob) = Class::<Blob>::from_object(obj) {
                        let blob = blob.borrow();
                        if !blob.mime_type().is_empty() {
                            content_type = Some(blob.mime_type());
                        }
                        Some(Ok(BodyVariant::Provided(Some(body))))
                    } else if let Some(fd) = Class::<FormData>::from_object(obj) {
                        let fd = fd.borrow();
                        let (multipart_body, boundary) = fd.to_multipart_bytes(&ctx).ok()?;
                        content_type = Some([MIME_TYPE_FORM_DATA, &boundary].concat());
                        Some(Ok(BodyVariant::Provided(Some(
                            multipart_body.into_js(&ctx).ok()?,
                        ))))
                    } else if obj.instance_of::<URLSearchParams>() {
                        content_type = Some(MIME_TYPE_FORM_URLENCODED.into());
                        Some(Ok(BodyVariant::Provided(Some(body))))
                    } else {
                        Some(Ok(BodyVariant::Provided(Some(body))))
                    }
                } else {
                    Some(Ok(BodyVariant::Provided(Some(body))))
                }
            })
            .transpose()?
            .unwrap_or_else(|| BodyVariant::Empty);

        let mut headers = headers.unwrap_or_default();
        if !headers.has(ctx.clone(), HEADERS_KEY_CONTENT_TYPE.into())? {
            if let Some(value) = content_type {
                headers.set(
                    ctx.clone(),
                    HEADERS_KEY_CONTENT_TYPE.into(),
                    value.into_js(&ctx)?,
                )?;
            }
        }
        let headers = Class::instance(ctx.clone(), headers)?;

        Ok(Self {
            body: RwLock::new(body),
            body_stream: RefCell::new(None),
            method: "GET".into(),
            url,
            start: Instant::now(),
            status,
            status_text,
            redirected: false,
            headers,
            abort_receiver,
        })
    }

    #[qjs(get)]
    pub fn status(&self) -> u64 {
        self.status.into()
    }

    #[qjs(get)]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    #[qjs(get)]
    pub fn ok(&self) -> bool {
        self.status > 199 && self.status < 300
    }

    #[qjs(get)]
    pub fn redirected(&self) -> bool {
        self.redirected
    }

    #[qjs(get)]
    pub fn body(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        // Return cached stream if available
        if let Some(stream) = self.body_stream.borrow().as_ref() {
            return Ok(stream.clone());
        }

        let body_guard = self.body.read().unwrap();
        match &*body_guard {
            BodyVariant::Incoming(incoming, content_encoding) => {
                let incoming = incoming.clone();
                let content_encoding = content_encoding.clone();
                drop(body_guard);
                let stream = create_body_stream(&ctx, incoming, content_encoding)?;
                *self.body_stream.borrow_mut() = Some(stream.clone());
                Ok(stream)
            },
            // Per WHATWG Fetch spec, body should be null for null body responses
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

                let stream = crate::create_body_value_stream(&ctx, body_value)?;
                *self.body_stream.borrow_mut() = Some(stream.clone());
                Ok(stream)
            },
        }
    }

    #[qjs(get)]
    fn headers(&self) -> Class<'js, Headers> {
        self.headers.clone()
    }

    #[qjs(get, rename = "type")]
    fn response_type(&self) -> &'js str {
        match &self.status {
            0 => "error",
            _ => "basic",
        }
    }

    #[qjs(get, rename = PredefinedAtom::SymbolToStringTag)]
    pub fn to_string_tag(&self) -> &'static str {
        stringify!(Response)
    }

    #[qjs(get)]
    fn status_text(&self) -> String {
        if let Some(text) = &self.status_text {
            return text.to_string();
        }
        STATUS_TEXTS.get(&self.status).unwrap_or(&"").to_string()
    }

    #[qjs(get)]
    fn body_used(&self) -> bool {
        // Check if body stream is disturbed (has been read from)
        if let Some(stream_value) = self.body_stream.borrow().as_ref() {
            if let Some(stream) = stream_value
                .as_object()
                .and_then(Class::<ReadableStream>::from_object)
            {
                // Check the 'disturbed' property - stream has been read from
                if let Ok(disturbed) = stream.get::<_, bool>("disturbed") {
                    if disturbed {
                        return true;
                    }
                }
            }
        }

        if let Ok(body) = self.body.read() {
            return match &*body {
                BodyVariant::Incoming(incoming, _) => incoming.borrow().is_none(),
                BodyVariant::Provided(value) => value.is_none(),
                BodyVariant::Empty => false,
            };
        }
        false
    }

    pub(crate) async fn text(&self, ctx: Ctx<'js>) -> Result<String> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(String::from_utf8_lossy(&strip_bom(bytes)).to_string());
        }
        Ok("".into())
    }

    pub(crate) async fn json(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return json_parse(&ctx, strip_bom(bytes));
        }
        Err(Exception::throw_syntax(&ctx, "JSON input is empty"))
    }

    async fn array_buffer(&self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return ArrayBuffer::new(ctx, bytes);
        }
        ArrayBuffer::new(ctx, Vec::<u8>::new())
    }

    async fn bytes(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return TypedArray::new(ctx, bytes).map(|m| m.into_value());
        }
        TypedArray::new(ctx, Vec::<u8>::new()).map(|m| m.into_value())
    }

    async fn blob(&self, ctx: Ctx<'js>) -> Result<Blob> {
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

    pub(crate) fn clone(&self, ctx: Ctx<'js>) -> Result<Self> {
        let body = self.body.read().unwrap();
        let cloned_body = match &*body {
            BodyVariant::Incoming(incoming, _) => {
                // Cannot clone if body has been consumed
                if incoming.borrow().is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Cannot clone response after body has been used",
                    ));
                }
                return Err(Exception::throw_type(
                    &ctx,
                    "Cannot clone response with unconsumed streaming body. Read the body first or use Response.clone() on responses with non-streaming bodies.",
                ));
            },
            BodyVariant::Provided(provided) => {
                // Cannot clone if body has been consumed (bodyUsed=true)
                if provided.is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Cannot clone response after body has been used",
                    ));
                }
                // For ReadableStream bodies, use tee() to create two branches
                if let Some(ref value) = provided {
                    if let Some(stream) = value
                        .as_object()
                        .and_then(Class::<ReadableStream>::from_object)
                    {
                        let (branch1, branch2) =
                            crate::tee_stream_for_clone(&ctx, &stream, "response")?;

                        // Update original body to use branch1
                        drop(body);
                        let mut body_write = self.body.write().unwrap();
                        *body_write = BodyVariant::Provided(Some(branch1.into_value()));
                        *self.body_stream.borrow_mut() = None;
                        drop(body_write);

                        return Ok(Self {
                            body: RwLock::new(BodyVariant::Provided(Some(branch2.into_value()))),
                            body_stream: RefCell::new(None),
                            method: self.method.clone(),
                            url: self.url.clone(),
                            start: self.start,
                            status: self.status,
                            status_text: self.status_text.clone(),
                            redirected: self.redirected,
                            headers: Class::<Headers>::instance(
                                ctx,
                                self.headers.borrow().clone(),
                            )?,
                            abort_receiver: self.abort_receiver.clone(),
                        });
                    }
                }
                BodyVariant::Provided(provided.clone())
            },
            BodyVariant::Empty => BodyVariant::Empty,
        };
        drop(body);

        Ok(Self {
            body: RwLock::new(cloned_body),
            body_stream: RefCell::new(None),
            method: self.method.clone(),
            url: self.url.clone(),
            start: self.start,
            status: self.status,
            status_text: self.status_text.clone(),
            redirected: self.redirected,
            headers: Class::<Headers>::instance(ctx, self.headers.borrow().clone())?,
            abort_receiver: self.abort_receiver.clone(),
        })
    }

    #[qjs(static)]
    fn error(ctx: Ctx<'js>) -> Result<Self> {
        Ok(Self {
            body: RwLock::new(BodyVariant::Empty),
            body_stream: RefCell::new(None),
            method: "".into(),
            url: "".into(),
            start: Instant::now(),
            status: 0,
            status_text: None,
            redirected: false,
            headers: Class::instance(ctx.clone(), Headers::default())?,
            abort_receiver: None,
        })
    }

    #[qjs(static, rename = "json")]
    fn json_static(ctx: Ctx<'js>, body: Value<'js>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut status = 200;
        let mut status_text = None;

        if let Some(ref opt) = options.0 {
            if let Some(status_opt) = opt.get("status")? {
                status = status_opt;
            }
            if let Some(status_text_opt) = opt.get("statusText")? {
                status_text = Some(status_text_opt);
            }
        }

        let mut headers = if let Some(ref opt) = options.0 {
            let head = if let Some(headers_opt) = opt.get("headers")? {
                headers_opt
            } else {
                Value::new_null(ctx.clone())
            };
            Headers::from_value(&ctx, head, HeadersGuard::Response)?
        } else {
            Headers::from_value(&ctx, Value::new_null(ctx.clone()), HeadersGuard::Response)?
        };

        if !headers.has(ctx.clone(), "content-type".into())? {
            headers.append(
                ctx.clone(),
                "content-type".into(),
                MIME_TYPE_JSON.into_js(&ctx)?,
            )?;
        }

        let headers = Class::instance(ctx.clone(), headers)?;

        let body = if let Ok(Some(v)) = json_stringify(&ctx, body) {
            BodyVariant::Provided(Some(v.into_js(&ctx)?))
        } else {
            return Err(Exception::throw_type(&ctx, "Failed to convert JSON string"));
        };

        Ok(Self {
            body: RwLock::new(body),
            body_stream: RefCell::new(None),
            method: "".into(),
            url: "".into(),
            start: Instant::now(),
            status,
            status_text,
            redirected: false,
            headers,
            abort_receiver: None,
        })
    }

    #[qjs(static)]
    fn redirect(
        ctx: Ctx<'js>,
        url: Either<URL<'js>, Coerced<String>>,
        status: Opt<u16>,
    ) -> Result<Self> {
        let status = status.0.unwrap_or(302_u16);

        // Validate redirect status (301, 302, 303, 307, 308 per WHATWG Fetch spec)
        if !matches!(status, 301 | 302 | 303 | 307 | 308) {
            return Err(Exception::throw_range(
                &ctx,
                &format!("Invalid redirect status: {}", status),
            ));
        }

        let url = match url {
            Either::Left(url) => url.to_string(),
            Either::Right(url) => url.0,
        };

        let mut header = BTreeMap::new();
        header.insert("location".to_string(), Coerced(url));
        let headers = Headers::from_map(&ctx, header, HeadersGuard::Response);
        let headers = Class::instance(ctx.clone(), headers)?;

        Ok(Self {
            body: RwLock::new(BodyVariant::Empty),
            body_stream: RefCell::new(None),
            method: "".into(),
            url: "".into(),
            start: Instant::now(),
            status,
            status_text: None,
            redirected: false,
            headers,
            abort_receiver: None,
        })
    }
}

#[allow(clippy::too_many_arguments)]
impl<'js> Response<'js> {
    pub fn from_incoming(
        ctx: Ctx<'js>,
        response: hyper::Response<Incoming>,
        method: String,
        url: String,
        start: Instant,
        redirected: bool,
        abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
        guard: HeadersGuard,
    ) -> Result<Self> {
        let response_headers = response.headers();

        let mut content_encoding = None;
        if let Some(content_encoding_header) =
            response_headers.get(HeaderName::from_static("content-encoding"))
        {
            if let Ok(content_encoding_header) = content_encoding_header.to_str() {
                content_encoding = Some(content_encoding_header.to_owned())
            }
        }

        let headers = Headers::from_http_headers(response.headers(), guard)?;
        let headers = Class::instance(ctx.clone(), headers)?;

        let status = response.status();

        Ok(Self {
            body: RwLock::new(BodyVariant::Incoming(
                Rc::new(RefCell::new(Some(response.into_body()))),
                content_encoding.clone(),
            )),
            body_stream: RefCell::new(None),
            method,
            url,
            start,
            status: status.as_u16(),
            status_text: None,
            redirected,
            headers,
            abort_receiver,
        })
    }

    #[allow(clippy::await_holding_lock)]
    #[allow(clippy::readonly_write_lock)]
    async fn take_bytes(&self, ctx: &Ctx<'js>) -> Result<Option<Vec<u8>>> {
        // Check if body stream is locked
        if let Some(stream_value) = self.body_stream.borrow().as_ref() {
            if let Some(stream) = stream_value
                .as_object()
                .and_then(Class::<ReadableStream>::from_object)
            {
                if stream.borrow().is_readable_stream_locked() {
                    return Err(Exception::throw_type(
                        ctx,
                        "Cannot read body: stream is locked",
                    ));
                }
            }
        }

        // Single lock acquisition to determine variant and extract Incoming if applicable
        let incoming_data = {
            let body_guard = self.body.read().unwrap();
            match &*body_guard {
                BodyVariant::Incoming(incoming, enc) => Some((incoming.clone(), enc.clone())),
                BodyVariant::Provided(None) => {
                    return Err(Exception::throw_message(ctx, "Already read"))
                },
                BodyVariant::Empty => return Ok(None),
                BodyVariant::Provided(Some(_)) => None,
            }
        };

        if let Some((incoming, content_encoding)) = incoming_data {
            let body = incoming
                .borrow_mut()
                .take()
                .ok_or(Exception::throw_message(ctx, "Already read"))?;

            // Read all frames
            let mut bytes = Vec::new();
            let mut body = body;
            while let Some(frame) = body.frame().await {
                match frame {
                    Ok(frame) => {
                        if let Some(data) = frame.data_ref() {
                            bytes.extend_from_slice(data);
                        }
                    },
                    Err(e) => return Err(Exception::throw_message(ctx, &e.to_string())),
                }
            }

            // Decompress if needed
            if let Some(encoding) = &content_encoding {
                if let Ok(mut decoder) = StreamingDecoder::new(encoding) {
                    let decompressed = decoder
                        .decompress_chunk(&bytes)
                        .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
                    let remaining = decoder
                        .finish()
                        .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
                    let mut result = decompressed;
                    result.extend(remaining);
                    return Ok(Some(result));
                }
            }

            return Ok(Some(bytes));
        }

        // Handle Provided case
        let mut body_guard = self.body.write().unwrap();
        if let BodyVariant::Provided(provided) = &mut *body_guard {
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

            let bytes =
                if let Some(blob) = provided.as_object().and_then(Class::<Blob>::from_object) {
                    blob.borrow().get_bytes()
                } else {
                    let obj_bytes = ObjectBytes::from(ctx, &provided)?;
                    obj_bytes.as_bytes(ctx)?.to_vec()
                };
            return Ok(Some(bytes));
        }

        Ok(None)
    }

    fn get_headers(&self, ctx: &Ctx<'js>) -> Result<Headers> {
        Headers::from_value(ctx, self.headers().as_value().clone(), HeadersGuard::None)
    }

    fn get_header_value(&self, ctx: &Ctx<'js>, key: &str) -> Result<Option<String>> {
        Ok(self
            .get_headers(ctx)?
            .iter()
            .find_map(|(k, v)| (k == key).then(|| v.to_string())))
    }
}

/// Collects all data from a ReadableStream into a Vec<u8>
async fn collect_readable_stream<'js>(
    _ctx: &Ctx<'js>,
    stream: &Class<'js, ReadableStream<'js>>,
) -> Result<Vec<u8>> {
    crate::collect_readable_stream(stream).await
}

fn create_body_stream<'js>(
    ctx: &Ctx<'js>,
    incoming: Rc<RefCell<Option<Incoming>>>,
    content_encoding: Option<String>,
) -> Result<Value<'js>> {
    // State: body + decoder
    struct BodyState {
        body: Option<Incoming>,
        decoder: Option<StreamingDecoder>,
        error: Option<String>,
    }

    let decoder = content_encoding
        .as_ref()
        .and_then(|enc| StreamingDecoder::new(enc).ok());

    // Take the body out of the shared Rc<RefCell> once, own it in the pull state
    let body = incoming.borrow_mut().take();

    let state = Rc::new(RefCell::new(BodyState {
        body,
        decoder,
        error: None,
    }));

    let pull = PullAlgorithm::from_fn(
        move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
            let state = state.clone();
            let state_for_cb = state.clone();

            // Get the default controller class
            let ctrl_class: ReadableStreamDefaultControllerClass = match controller {
                ReadableStreamControllerClass::ReadableStreamDefaultController(c) => c,
                _ => {
                    return Err(rquickjs::Exception::throw_type(
                        &ctx,
                        "Expected default controller",
                    ))
                },
            };

            // Create a future that reads one frame
            let future = async move {
                let mut body = {
                    let mut state_ref = state.borrow_mut();
                    match state_ref.body.take() {
                        Some(b) => b,
                        None => return Ok::<_, rquickjs::Error>(None),
                    }
                };

                let frame_result = body.frame().await;

                match frame_result {
                    Some(Ok(frame)) => {
                        // Put body back for next pull
                        state.borrow_mut().body = Some(body);

                        if let Some(data) = frame.data_ref() {
                            let mut state_ref = state.borrow_mut();
                            let bytes = if let Some(dec) = state_ref.decoder.as_mut() {
                                dec.decompress_chunk(data).unwrap_or_else(|_| data.to_vec())
                            } else {
                                data.to_vec()
                            };
                            Ok(Some(bytes))
                        } else {
                            Ok(Some(Vec::new()))
                        }
                    },
                    Some(Err(e)) => {
                        state.borrow_mut().error = Some(e.to_string());
                        Ok(None)
                    },
                    None => {
                        // End of body - flush decoder
                        let remaining = {
                            let mut state_ref = state.borrow_mut();
                            if let Some(dec) = state_ref.decoder.take() {
                                dec.finish().unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        };
                        if remaining.is_empty() {
                            Ok(None)
                        } else {
                            Ok(Some(remaining))
                        }
                    },
                }
            };

            // Convert future to promise
            let promise = rquickjs::Promise::wrap_future(&ctx, future)?;

            let result_promise = upon_promise_fulfilment(
                ctx.clone(),
                promise,
                move |ctx, result: Option<Vec<u8>>| {
                    match result {
                        Some(bytes) if !bytes.is_empty() => {
                            let array = TypedArray::<u8>::new(ctx.clone(), bytes)?;
                            readable_stream_default_controller_enqueue_value(
                                ctx,
                                ctrl_class.clone(),
                                array.into_value(),
                            )?;
                        },
                        Some(_) => {}, // Empty bytes - do nothing
                        None => {
                            // Check if this was an error or a normal close
                            let error_msg = state_for_cb.borrow_mut().error.take();
                            if let Some(msg) = error_msg {
                                let error_val =
                                    rquickjs::String::from_str(ctx.clone(), &msg)?.into_value();
                                readable_stream_default_controller_error_stream(
                                    ctrl_class.clone(),
                                    error_val,
                                )?;
                            } else {
                                readable_stream_default_controller_close_stream(
                                    ctx,
                                    ctrl_class.clone(),
                                )?;
                            }
                        },
                    }
                    Ok::<_, rquickjs::Error>(())
                },
            )?;

            Ok(result_promise)
        },
    );

    let stream = ReadableStream::from_pull_algorithm(
        ctx.clone(),
        pull,
        CancelAlgorithm::ReturnPromiseUndefined,
    )?;

    Ok(stream.into_value())
}

#[cfg(test)]
mod tests {
    use llrt_test::{test_async_with_opts, TestOptions};
    use rquickjs::{CatchResultExt, Class, Function, Object, Promise};
    use wiremock::*;

    use super::*;

    #[tokio::test]
    async fn test_response_stream() {
        let mock_server = MockServer::start().await;
        let welcome_message = "Hello, LLRT!";

        Mock::given(matchers::path("some-path/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(welcome_message.to_string()))
            .mount(&mock_server)
            .await;

        test_async_with_opts(
            |ctx| {
                crate::init(&ctx).unwrap();
                Box::pin(async move {
                    let globals = ctx.globals();
                    let run = async {
                        let fetch: Function = globals.get("fetch")?;
                        let options = Object::new(ctx.clone())?;
                        options.set("method", "GET")?;
                        let url = format!("http://{}/some-path/", mock_server.address().clone());

                        let response_promise: Promise = fetch.call((url, options.clone()))?;
                        let response: Class<Response> = response_promise.into_future().await?;
                        let response = response.borrow_mut();

                        let response_text = response.text(ctx.clone()).await?;
                        assert_eq!(response.status(), 200);
                        assert_eq!(response_text, welcome_message);
                        Ok(())
                    };
                    run.await.catch(&ctx).unwrap();
                })
            },
            TestOptions::new().no_pending_jobs(),
        )
        .await;
    }

    #[tokio::test]
    async fn test_response_clone_error() {
        let mock_server = MockServer::start().await;

        Mock::given(matchers::path("some-path/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello".to_string()))
            .mount(&mock_server)
            .await;

        test_async_with_opts(
            |ctx| {
                crate::init(&ctx).unwrap();
                Box::pin(async move {
                    let globals = ctx.globals();
                    let run = async {
                        let fetch: Function = globals.get("fetch")?;
                        let options = Object::new(ctx.clone())?;
                        options.set("method", "GET")?;
                        let url = format!("http://{}/some-path/", mock_server.address().clone());

                        let response_promise: Promise = fetch.call((url, options.clone()))?;
                        let response: Class<Response> = response_promise.into_future().await?;
                        let response = response.borrow_mut();

                        // Cloning a response with unconsumed body should fail
                        let clone_result = response.clone(ctx.clone());
                        assert!(clone_result.is_err());
                        Ok(())
                    };
                    run.await.catch(&ctx).unwrap();
                })
            },
            TestOptions::new().no_pending_jobs(),
        )
        .await;
    }

    #[tokio::test]
    async fn test_response_large_body() {
        let mock_server = MockServer::start().await;
        let large_body = vec![b'x'; 1024 * 1024]; // 1MB

        Mock::given(matchers::path("some-path/"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(large_body.clone()))
            .mount(&mock_server)
            .await;

        test_async_with_opts(
            |ctx| {
                crate::init(&ctx).unwrap();
                Box::pin(async move {
                    let globals = ctx.globals();
                    let run = async {
                        let fetch: Function = globals.get("fetch")?;
                        let options = Object::new(ctx.clone())?;
                        options.set("method", "GET")?;
                        let url = format!("http://{}/some-path/", mock_server.address().clone());

                        let response_promise: Promise = fetch.call((url, options.clone()))?;
                        let response: Class<Response> = response_promise.into_future().await?;
                        let response = response.borrow_mut();

                        let response_text = response.text(ctx.clone()).await?;
                        assert_eq!(response.status(), 200);
                        assert_eq!(response_text.as_bytes(), large_body);
                        Ok(())
                    };
                    run.await.catch(&ctx).unwrap();
                })
            },
            TestOptions::new().no_pending_jobs(),
        )
        .await;
    }
}
