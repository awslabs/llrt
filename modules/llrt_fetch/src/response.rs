// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use super::{
    headers::{Headers, HeadersGuard},
    Blob, FormData, MIME_TYPE_FORM_DATA, MIME_TYPE_FORM_URLENCODED, MIME_TYPE_JSON_STATIC,
    MIME_TYPE_OCTET_STREAM, MIME_TYPE_TEXT,
};
use crate::body_helpers::{self, strip_bom};
use crate::{body_helpers::collect_readable_stream, utils::BodyDrain};
use either::Either;
use http_body::Body as _;
use http_body_util::BodyExt;
use hyper::{
    body::Incoming,
    header::{CONTENT_ENCODING, CONTENT_TYPE, LOCATION},
};
use llrt_abort::AbortSignal;
use llrt_compression::streaming::StreamingDecoder;
use llrt_context::CtxExtension;
use llrt_json::{parse::json_parse, stringify::json_stringify};
use llrt_stream_web::{
    readable_stream_default_controller_close_stream,
    readable_stream_default_controller_enqueue_value,
    readable_stream_default_controller_error_stream, utils::promise::ResolveablePromise,
    CancelAlgorithm, NativePullResult, PullAlgorithm, ReadableStream,
    ReadableStreamControllerClass, ReadableStreamDefaultControllerClass,
};
use llrt_url::{url_class::URL, url_search_params::URLSearchParams};
use llrt_utils::{bytes::ObjectBytes, mc_oneshot};
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    function::Opt,
    prelude::This,
    ArrayBuffer, Class, Coerced, Ctx, Exception, FromJs, IntoJs, JsLifetime, Object, Promise,
    Result, TypedArray, Value,
};
use std::{
    pin::Pin,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    task::{Context, Poll, Waker},
    time::Instant,
};

/// A validated HTTP status code (200-599 per WHATWG Fetch spec).
#[derive(Clone, Copy)]
struct StatusCode(u16);

impl StatusCode {
    fn is_null_body(self) -> bool {
        matches!(self.0, 101 | 103 | 204 | 205 | 304)
    }
}

impl<'js> FromJs<'js> for StatusCode {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let code = u16::from_js(ctx, value)?;
        if !(200..=599).contains(&code) {
            return Err(Exception::throw_range(
                ctx,
                &format!("Invalid status code: {}", code),
            ));
        }
        Ok(Self(code))
    }
}

impl From<StatusCode> for u16 {
    fn from(s: StatusCode) -> u16 {
        s.0
    }
}

/// Validate reason-phrase per RFC 9110: HTAB / SP / VCHAR / obs-text (no CR/LF).
/// Mark a Response's body as consumed (unless it has a null body, in which
/// case `bodyUsed` must stay false per spec). If there is a cached body
/// stream, lock it by acquiring a reader so subsequent `getReader()` calls
/// throw synchronously.
fn mark_consumed<'js>(response: &Class<'js, Response<'js>>) {
    let resp = response.borrow();
    let is_null_body = matches!(&*resp.body.read().unwrap(), BodyVariant::Empty);
    if !is_null_body {
        resp.body_consumed.store(true, Ordering::Release);
    }
}

fn is_valid_reason_phrase(s: &str) -> bool {
    s.chars().all(|c| {
        let cp = c as u32;
        cp == 0x09 || (0x20..=0x7E).contains(&cp) || (0x80..=0xFF).contains(&cp)
    })
}

/// Cheap sanity check for a path-relative URL (`/foo`) used by
/// `Response.redirect`. Real resolution needs a base URL, which we don't
/// always have — but WPT expects `Response.redirect("/")` to succeed with
/// the path going straight into the `Location` header.
fn looks_like_relative_url(s: &str) -> bool {
    s.starts_with('/') && !s.contains(['\r', '\n', '\t', ' '])
}

enum BodyVariant<'js> {
    /// Raw incoming HTTP body - consumed directly for text()/json()/etc
    Incoming(Arc<RwLock<Option<Incoming>>>, Option<String>), // body + content_encoding
    /// User-provided body value
    Provided(Option<Value<'js>>),
    /// Empty body
    Empty,
}

#[rquickjs::class]
pub struct Response<'js> {
    body: RwLock<BodyVariant<'js>>,
    /// Cached ReadableStream for the body getter (created lazily)
    body_stream: RwLock<Option<Value<'js>>>,
    body_consumed: AtomicBool,
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
        if let Some(stream) = self.body_stream.read().unwrap().as_ref() {
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
        let mut status_is_null_body = false;

        if let Some(opt) = options.0 {
            if let Some(url_opt) = opt.get("url")? {
                url = url_opt;
            }
            if let Some(status_opt) = opt.get::<_, Option<StatusCode>>("status")? {
                status_is_null_body = status_opt.is_null_body();
                status = status_opt.0;
            }
            if let Some(headers_opt) = opt.get("headers")? {
                headers = Some(Headers::from_value(
                    &ctx,
                    headers_opt,
                    HeadersGuard::Response,
                )?);
            }
            if let Some(status_text_opt) = opt.get::<_, Option<String>>("statusText")? {
                if !is_valid_reason_phrase(&status_text_opt) {
                    return Err(Exception::throw_type(&ctx, "Invalid statusText"));
                }
                status_text = Some(status_text_opt);
            }

            if let Some(signal) = opt.get::<_, Option<Class<AbortSignal>>>("signal")? {
                abort_receiver = Some(signal.borrow().sender.subscribe())
            }
        }

        let has_body = body
            .0
            .as_ref()
            .is_some_and(|b| !b.is_null() && !b.is_undefined());

        // Null body status check (204, 304 per WHATWG Fetch spec)
        if has_body && status_is_null_body {
            return Err(Exception::throw_type(
                &ctx,
                "Response with null body status cannot have body",
            ));
        }

        let mut content_type: Option<String> = None;
        let response_body_stream: RwLock<Option<Value<'js>>> = RwLock::new(None);

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
                        if let Err(e) = body_helpers::validate_stream_usable(
                            &ctx,
                            &stream,
                            "construct Response",
                        ) {
                            return Some(Err(e));
                        }
                        *response_body_stream.write().unwrap() = Some(body.clone());
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

        let mut headers = headers.unwrap_or_else(|| {
            let mut h = Headers::default();
            h.guard = HeadersGuard::Response;
            h
        });
        if !headers.contains_lower(CONTENT_TYPE.as_str()) {
            if let Some(value) = content_type {
                headers.set(
                    ctx.clone(),
                    CONTENT_TYPE.as_str().into(),
                    value.into_js(&ctx)?,
                )?;
            }
        }
        let headers = Class::instance(ctx.clone(), headers)?;

        Ok(Self {
            body: RwLock::new(body),
            body_stream: response_body_stream,
            body_consumed: AtomicBool::new(false),
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
    pub fn url(&self) -> &str {
        &self.url
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
        // If a consume method was called, expose a disturbed stream so
        // subsequent `.body.getReader()` and similar throw synchronously.
        if self.body_consumed.load(Ordering::Acquire) {
            if let Some(stream_val) = self.body_stream.read().unwrap().as_ref() {
                // If the cached stream is already locked or disturbed we can
                // return it directly; otherwise replace it with a disturbed
                // stream.
                if let Some(stream) = stream_val
                    .as_object()
                    .and_then(Class::<ReadableStream>::from_object)
                {
                    let s = stream.borrow();
                    if s.disturbed || s.is_readable_stream_locked() {
                        return Ok(stream_val.clone());
                    }
                }
            }
            let stream = body_helpers::create_disturbed_stream(&ctx)?;
            *self.body_stream.write().unwrap() = Some(stream.clone());
            return Ok(stream);
        }

        // Return cached stream if available
        if let Some(stream) = self.body_stream.read().unwrap().as_ref() {
            return Ok(stream.clone());
        }

        let body_guard = self.body.read().unwrap();
        match &*body_guard {
            BodyVariant::Incoming(incoming, content_encoding) => {
                // If body has already been consumed, create a disturbed stream
                if incoming.read().unwrap().is_none() {
                    drop(body_guard);
                    let stream = body_helpers::create_disturbed_stream(&ctx)?;
                    *self.body_stream.write().unwrap() = Some(stream.clone());
                    return Ok(stream);
                }
                let incoming = incoming.clone();
                let content_encoding = content_encoding.clone();
                drop(body_guard);
                let stream = create_body_stream(&ctx, incoming, content_encoding)?;
                *self.body_stream.write().unwrap() = Some(stream.clone());
                Ok(stream)
            },
            // Per WHATWG Fetch spec, body should be null for null body responses
            BodyVariant::Empty => Ok(Value::new_null(ctx)),
            BodyVariant::Provided(None) => {
                drop(body_guard);
                let stream = body_helpers::create_disturbed_stream(&ctx)?;
                *self.body_stream.write().unwrap() = Some(stream.clone());
                Ok(stream)
            },
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

                let stream = body_helpers::create_body_value_stream(&ctx, body_value)?;
                *self.body_stream.write().unwrap() = Some(stream.clone());
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
        if self.status == 0 {
            "error"
        } else if self.url.is_empty() {
            "default"
        } else {
            "basic"
        }
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(Response)
    }

    #[qjs(get)]
    fn status_text(&self) -> &str {
        self.status_text.as_deref().unwrap_or("")
    }

    #[qjs(get)]
    fn body_used(&self) -> bool {
        if self.body_consumed.load(Ordering::Acquire) {
            return true;
        }
        if body_helpers::is_body_stream_disturbed(&self.body_stream) {
            return true;
        }

        if let Ok(body) = self.body.read() {
            return match &*body {
                BodyVariant::Incoming(incoming, _) => incoming.read().unwrap().is_none(),
                BodyVariant::Provided(value) => value.is_none(),
                BodyVariant::Empty => false,
            };
        }
        false
    }

    pub(crate) fn text(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        mark_consumed(&this.0);
        let class = this.0.clone();
        let ctx_clone = ctx.clone();
        Promise::wrap_future(&ctx, async move {
            let bytes_opt = class.borrow().take_bytes(&ctx_clone).await?;
            match bytes_opt {
                Some(bytes) => {
                    let bytes = strip_bom(bytes);
                    Result::<String>::Ok(match String::from_utf8(bytes.into()) {
                        Ok(s) => s,
                        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
                    })
                },
                None => Ok(String::new()),
            }
        })
    }

    pub(crate) fn json(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        mark_consumed(&this.0);
        let class = this.0.clone();
        let ctx_clone = ctx.clone();
        Promise::wrap_future(&ctx, async move {
            let bytes_opt = class.borrow().take_bytes(&ctx_clone).await?;
            match bytes_opt {
                Some(bytes) => json_parse(&ctx_clone, strip_bom(bytes)),
                None => Err(Exception::throw_syntax(&ctx_clone, "JSON input is empty")),
            }
        })
    }

    fn array_buffer(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        mark_consumed(&this.0);
        let class = this.0.clone();
        let ctx_clone = ctx.clone();
        Promise::wrap_future(&ctx, async move {
            let bytes = class
                .borrow()
                .take_bytes(&ctx_clone)
                .await?
                .unwrap_or_default();
            ArrayBuffer::new(ctx_clone, bytes)
        })
    }

    fn bytes(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        mark_consumed(&this.0);
        let class = this.0.clone();
        let ctx_clone = ctx.clone();
        Promise::wrap_future(&ctx, async move {
            let bytes = class
                .borrow()
                .take_bytes(&ctx_clone)
                .await?
                .unwrap_or_default();
            TypedArray::new(ctx_clone, bytes).map(|m| m.into_value())
        })
    }

    fn blob(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        mark_consumed(&this.0);
        let mime_type = this
            .0
            .borrow()
            .get_header_value(&ctx, CONTENT_TYPE.as_str())?;
        let class = this.0.clone();
        let ctx_clone = ctx.clone();
        Promise::wrap_future(&ctx, async move {
            let bytes = class
                .borrow()
                .take_bytes(&ctx_clone)
                .await?
                .unwrap_or_default();
            Blob::from_bytes(&ctx_clone, bytes, mime_type)
        })
    }

    fn form_data(this: This<Class<'js, Self>>, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        mark_consumed(&this.0);
        let mime_type = this
            .0
            .borrow()
            .get_header_value(&ctx, CONTENT_TYPE.as_str())?
            .unwrap_or(MIME_TYPE_OCTET_STREAM.into());
        let class = this.0.clone();
        let ctx_clone = ctx.clone();
        Promise::wrap_future(&ctx, async move {
            let is_multipart = mime_type.starts_with("multipart/form-data");
            let is_urlencoded =
                mime_type.starts_with(MIME_TYPE_FORM_URLENCODED.split(';').next().unwrap_or(""));

            // Consume the body first so stream errors propagate before the
            // content-type check (matching spec order).
            let bytes = class.borrow().take_bytes(&ctx_clone).await?;

            if !is_multipart && !is_urlencoded {
                return Err(Exception::throw_type(
                    &ctx_clone,
                    "formData: invalid Content-Type",
                ));
            }

            if let Some(bytes) = bytes {
                let form_data = FormData::from_multipart_bytes(&ctx_clone, &mime_type, bytes)?;
                return Ok(form_data);
            }
            if is_multipart {
                return Err(Exception::throw_type(&ctx_clone, "formData: body is empty"));
            }
            Ok(FormData::default())
        })
    }

    pub(crate) fn clone(&self, ctx: Ctx<'js>) -> Result<Self> {
        let body = self.body.read().unwrap();
        let cloned_body = match &*body {
            BodyVariant::Incoming(incoming, content_encoding) => {
                // Cannot clone if body has been consumed
                if incoming.read().unwrap().is_none() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Cannot clone response after body has been used",
                    ));
                }

                // Convert to stream first, then tee
                let stream_val =
                    create_body_stream(&ctx, incoming.clone(), content_encoding.clone())?;
                let stream = Class::<ReadableStream>::from_value(&stream_val)?;
                let (branch1, branch2) =
                    llrt_stream_web::tee_readable_stream(ctx.clone(), stream.clone())?;

                // Update self to use branch1
                drop(body);
                let mut body_write = self.body.write().unwrap();
                *body_write = BodyVariant::Provided(Some(branch1.into_value()));
                *self.body_stream.write().unwrap() = None;

                return Ok(Self {
                    body: RwLock::new(BodyVariant::Provided(Some(branch2.into_value()))),
                    body_stream: RwLock::new(None),
                    body_consumed: AtomicBool::new(false),
                    method: self.method.clone(),
                    url: self.url.clone(),
                    start: self.start,
                    status: self.status,
                    status_text: self.status_text.clone(),
                    redirected: self.redirected,
                    headers: Class::<Headers>::instance(ctx, self.headers.borrow().clone())?,
                    abort_receiver: self.abort_receiver.clone(),
                });
            },
            BodyVariant::Provided(provided) => {
                let Some(provided) = provided else {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Cannot clone response after body has been used",
                    ));
                };
                // For ReadableStream bodies, use tee() to create two branches
                if let Some(stream) = provided
                    .as_object()
                    .and_then(Class::<ReadableStream>::from_object)
                {
                    let (branch1, branch2) =
                        llrt_stream_web::tee_readable_stream(ctx.clone(), stream.clone())?;

                    // Update original body to use branch1
                    drop(body);
                    let mut body_write = self.body.write().unwrap();
                    *body_write = BodyVariant::Provided(Some(branch1.into_value()));
                    *self.body_stream.write().unwrap() = None;

                    return Ok(Self {
                        body: RwLock::new(BodyVariant::Provided(Some(branch2.into_value()))),
                        body_stream: RwLock::new(None),
                        body_consumed: AtomicBool::new(false),
                        method: self.method.clone(),
                        url: self.url.clone(),
                        start: self.start,
                        status: self.status,
                        status_text: self.status_text.clone(),
                        redirected: self.redirected,
                        headers: Class::<Headers>::instance(ctx, self.headers.borrow().clone())?,
                        abort_receiver: self.abort_receiver.clone(),
                    });
                }
                BodyVariant::Provided(Some(provided.clone()))
            },
            BodyVariant::Empty => BodyVariant::Empty,
        };
        drop(body);

        Ok(Self {
            body: RwLock::new(cloned_body),
            body_stream: RwLock::new(None),
            body_consumed: AtomicBool::new(false),
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
        let mut headers = Headers::default();
        headers.guard = HeadersGuard::Immutable;
        Ok(Self {
            body: RwLock::new(BodyVariant::Empty),
            body_stream: RwLock::new(None),
            body_consumed: AtomicBool::new(false),
            method: "".into(),
            url: "".into(),
            start: Instant::now(),
            status: 0,
            status_text: None,
            redirected: false,
            headers: Class::instance(ctx.clone(), headers)?,
            abort_receiver: None,
        })
    }

    #[qjs(static, rename = "json")]
    fn json_static(ctx: Ctx<'js>, body: Value<'js>, options: Opt<Value<'js>>) -> Result<Self> {
        let options: Option<Object<'js>> =
            options
                .0
                .and_then(|v| if v.is_object() { v.into_object() } else { None });
        let mut status = 200;
        let mut status_text = None;

        if let Some(ref opt) = options {
            if let Some(status_opt) = opt.get::<_, Option<StatusCode>>("status")? {
                if status_opt.is_null_body() {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Response.json cannot have a null body status",
                    ));
                }
                status = status_opt.0;
            }
            if let Some(status_text_opt) = opt.get::<_, Option<String>>("statusText")? {
                if !is_valid_reason_phrase(&status_text_opt) {
                    return Err(Exception::throw_type(&ctx, "Invalid statusText"));
                }
                status_text = Some(status_text_opt);
            }
        }

        let mut headers = if let Some(ref opt) = options {
            let head = if let Some(headers_opt) = opt.get("headers")? {
                headers_opt
            } else {
                Value::new_null(ctx.clone())
            };
            Headers::from_value(&ctx, head, HeadersGuard::Response)?
        } else {
            Headers::from_value(&ctx, Value::new_null(ctx.clone()), HeadersGuard::Response)?
        };

        if !headers.contains_lower(CONTENT_TYPE.as_str()) {
            headers.append(
                ctx.clone(),
                CONTENT_TYPE.as_str().into(),
                MIME_TYPE_JSON_STATIC.into_js(&ctx)?,
            )?;
        }

        let headers = Class::instance(ctx.clone(), headers)?;

        let body = match json_stringify(&ctx, body)? {
            Some(v) => {
                // Store as bytes so serialisation preserves lone surrogates as
                // `\uXXXX` escapes produced by json_stringify.
                let bytes = v.into_bytes();
                BodyVariant::Provided(Some(TypedArray::new(ctx.clone(), bytes)?.into_value()))
            },
            None => return Err(Exception::throw_type(&ctx, "Failed to convert JSON string")),
        };

        Ok(Self {
            body: RwLock::new(body),
            body_stream: RwLock::new(None),
            body_consumed: AtomicBool::new(false),
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
        // Validate URL per WHATWG Fetch spec. We accept relative URLs too
        // (a bare path like `"/"` is legal per spec when parsed against the
        // current settings object's API base URL); only reject values that
        // clearly aren't URL-shaped.
        if !URL::is_valid(&url) && !looks_like_relative_url(&url) {
            return Err(Exception::throw_type(&ctx, "Invalid redirect URL"));
        }

        let mut headers = Headers::default();
        headers.guard = HeadersGuard::Response;
        headers.append(ctx.clone(), LOCATION.as_str().into(), url.into_js(&ctx)?)?;
        let headers = Class::instance(ctx.clone(), headers)?;

        Ok(Self {
            body: RwLock::new(BodyVariant::Empty),
            body_stream: RwLock::new(None),
            body_consumed: AtomicBool::new(false),
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
        if let Some(content_encoding_header) = response_headers.get(&CONTENT_ENCODING) {
            if let Ok(content_encoding_header) = content_encoding_header.to_str() {
                content_encoding = Some(content_encoding_header.to_owned())
            }
        }

        let headers = Headers::from_http_headers(response.headers(), guard)?;
        let headers = Class::instance(ctx.clone(), headers)?;

        let status = response.status();
        let is_null_body = matches!(status.as_u16(), 101 | 103 | 204 | 205 | 304)
            || method.eq_ignore_ascii_case("HEAD");

        let body = if is_null_body {
            BodyVariant::Empty
        } else {
            BodyVariant::Incoming(
                Arc::new(RwLock::new(Some(response.into_body()))),
                content_encoding,
            )
        };

        Ok(Self {
            body: RwLock::new(body),
            body_stream: RwLock::new(None),
            body_consumed: AtomicBool::new(false),
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

    /// Build a Response from pre-fetched + integrity-verified bytes. Used by
    /// the fetch path when `integrity:` metadata is set — the body has been
    /// collected and hashed already, so we embed the bytes as a Provided
    /// body (avoiding the Incoming streaming path that would re-decompress).
    #[allow(clippy::too_many_arguments)]
    pub fn from_verified_bytes(
        ctx: Ctx<'js>,
        parts: hyper::http::response::Parts,
        bytes: Vec<u8>,
        method: String,
        url: String,
        start: Instant,
        redirected: bool,
        abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
        guard: HeadersGuard,
    ) -> Result<Self> {
        let headers = Headers::from_http_headers(&parts.headers, guard)?;
        let headers = Class::instance(ctx.clone(), headers)?;

        let status = parts.status;
        let is_null_body = matches!(status.as_u16(), 101 | 103 | 204 | 205 | 304)
            || method.eq_ignore_ascii_case("HEAD");

        let body = if is_null_body {
            BodyVariant::Empty
        } else {
            let ab = rquickjs::ArrayBuffer::new_copy(ctx.clone(), &bytes)?;
            BodyVariant::Provided(Some(ab.into_value()))
        };

        Ok(Self {
            body: RwLock::new(body),
            body_stream: RwLock::new(None),
            body_consumed: AtomicBool::new(false),
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
        // Fast path: if no stream was ever created, skip all stream checks
        // and take the incoming body directly without cloning Arc/String.
        let body_stream_exists = self.body_stream.read().unwrap().is_some();

        if !body_stream_exists {
            let mut body_guard = self.body.write().unwrap();
            match &mut *body_guard {
                BodyVariant::Incoming(incoming, enc) => {
                    let body = incoming
                        .write()
                        .unwrap()
                        .take()
                        .ok_or(Exception::throw_type(ctx, "Body is already read"))?;
                    let encoding = enc.take();
                    drop(body_guard);

                    let has_decoder = encoding
                        .as_deref()
                        .is_some_and(|e| !matches!(e, "" | "identity"));

                    if !has_decoder {
                        let collected = body
                            .collect()
                            .await
                            .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
                        return Ok(Some(collected.to_bytes().into()));
                    }

                    let collected = body
                        .collect()
                        .await
                        .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
                    let raw = collected.to_bytes();
                    if let Some(mut dec) = encoding
                        .as_deref()
                        .and_then(|enc| StreamingDecoder::new(enc).ok())
                    {
                        let mut decompressed = dec
                            .decompress_chunk(&raw)
                            .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
                        let remaining = dec
                            .finish()
                            .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
                        if !remaining.is_empty() {
                            decompressed.extend_from_slice(&remaining);
                        }
                        return Ok(Some(decompressed));
                    }
                    return Ok(Some(raw.into()));
                },
                BodyVariant::Empty => return Ok(None),
                BodyVariant::Provided(None) => {
                    return Err(Exception::throw_type(ctx, "Body is already read"))
                },
                BodyVariant::Provided(Some(_)) => {
                    // Fall through to take_provided
                    drop(body_guard);
                    return take_provided(ctx, &self.body).await;
                },
            }
        }

        // Slow path: stream was accessed, need to check if it's locked/disturbed
        if let Some(stream_value) = self.body_stream.read().unwrap().as_ref() {
            if let Some(stream) = stream_value
                .as_object()
                .and_then(Class::<ReadableStream>::from_object)
            {
                body_helpers::validate_stream_usable(ctx, &stream, "read body")?;
            }
        }

        let incoming_data = {
            let body_guard = self.body.read().unwrap();
            match &*body_guard {
                BodyVariant::Incoming(incoming, enc) => Some((incoming.clone(), enc.clone())),
                BodyVariant::Provided(None) => {
                    return Err(Exception::throw_type(ctx, "Body is already read"))
                },
                BodyVariant::Empty => return Ok(None),
                BodyVariant::Provided(Some(_)) => None,
            }
        };

        if let Some((incoming, content_encoding)) = incoming_data {
            return take_incoming(ctx, &incoming, content_encoding.as_deref()).await;
        }

        take_provided(ctx, &self.body).await
    }

    fn get_header_value(&self, _ctx: &Ctx<'js>, key: &str) -> Result<Option<String>> {
        Ok(self
            .headers
            .borrow()
            .iter()
            .find_map(|(k, v)| (k == key).then(|| v.to_string())))
    }
}

/// Consume an incoming HTTP body, decompressing if needed.
async fn take_incoming<'js>(
    ctx: &Ctx<'js>,
    incoming: &Arc<RwLock<Option<Incoming>>>,
    content_encoding: Option<&str>,
) -> Result<Option<Vec<u8>>> {
    let body = incoming
        .write()
        .unwrap()
        .take()
        .ok_or(Exception::throw_type(ctx, "Body is already read"))?;

    let has_decoder = content_encoding.is_some_and(|e| !matches!(e, "" | "identity"));

    if !has_decoder {
        // Fast path: collect entire body at once — http-body-util handles
        // internal buffering efficiently and avoids per-frame async overhead.
        let collected = body
            .collect()
            .await
            .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
        return Ok(Some(collected.to_bytes().into()));
    }

    // Decompression path: collect raw bytes then decompress in one shot
    let collected = body
        .collect()
        .await
        .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
    let raw = collected.to_bytes();
    if let Some(mut dec) = content_encoding.and_then(|enc| StreamingDecoder::new(enc).ok()) {
        let mut decompressed = dec
            .decompress_chunk(&raw)
            .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
        let remaining = dec
            .finish()
            .map_err(|e| Exception::throw_message(ctx, &e.to_string()))?;
        if !remaining.is_empty() {
            decompressed.extend_from_slice(&remaining);
        }
        return Ok(Some(decompressed));
    }

    Ok(Some(raw.into()))
}

/// Consume a user-provided body value.
#[allow(clippy::readonly_write_lock)]
async fn take_provided<'js>(
    ctx: &Ctx<'js>,
    body: &RwLock<BodyVariant<'js>>,
) -> Result<Option<Vec<u8>>> {
    let provided = {
        let mut body_guard = body.write().unwrap();
        if let BodyVariant::Provided(provided) = &mut *body_guard {
            provided.take()
        } else {
            return Ok(None);
        }
    };

    let provided = provided.ok_or(Exception::throw_type(ctx, "Body is already read"))?;

    if let Some(stream) = provided
        .as_object()
        .and_then(Class::<ReadableStream>::from_object)
    {
        return collect_readable_stream(&stream).await.map(Some);
    }

    let bytes = if let Some(blob) = provided.as_object().and_then(Class::<Blob>::from_object) {
        blob.borrow().get_bytes()
    } else {
        let obj_bytes = ObjectBytes::from(ctx, &provided)?;
        obj_bytes.as_bytes(ctx)?.to_vec()
    };
    Ok(Some(bytes))
}

/// Read one or more frames from the body, coalescing buffered frames into a single Vec.
/// Returns Ok(Some(bytes)) for data, Ok(None) for EOF.
async fn read_body_chunk(
    body: &mut Incoming,
    decoder: &RwLock<Option<StreamingDecoder>>,
    has_decoder: bool,
) -> std::result::Result<Option<Vec<u8>>, String> {
    // 1. Get the first frame (the only async part)
    let first_frame = match body.frame().await {
        Some(Ok(frame)) => frame,
        Some(Err(e)) => return Err(e.to_string()),
        None => {
            let remaining = decoder.write().unwrap().take().and_then(|dec| {
                let r = dec.finish().unwrap_or_default();
                (!r.is_empty()).then_some(r)
            });
            return Ok(remaining);
        },
    };

    // 2. Extract initial data
    let Ok(first_data) = first_frame.into_data() else {
        return Ok(None);
    };

    // 3. Accumulate into buffer, draining all synchronously-ready frames
    let mut result_buffer = Vec::with_capacity(first_data.len());
    let mut dec_guard = has_decoder.then(|| decoder.write().unwrap());
    let mut error: Option<String> = None;

    // Inline helper: decompress or copy data into result_buffer
    macro_rules! process {
        ($data:expr) => {
            if error.is_none() {
                if let Some(Some(dec)) = dec_guard.as_mut().map(|g| g.as_mut()) {
                    match dec.decompress_chunk(&$data) {
                        Ok(decompressed) => result_buffer.extend_from_slice(&decompressed),
                        Err(e) => error = Some(e.to_string()),
                    }
                } else {
                    result_buffer.extend_from_slice(&$data);
                }
            }
        };
    }

    process!(first_data);
    body.drain_ready(|data| process!(data));

    drop(dec_guard);

    if let Some(e) = error {
        return Err(e);
    }

    Ok(if result_buffer.is_empty() {
        None
    } else {
        Some(result_buffer)
    })
}

fn create_body_stream<'js>(
    ctx: &Ctx<'js>,
    incoming: Arc<RwLock<Option<Incoming>>>,
    content_encoding: Option<String>,
) -> Result<Value<'js>> {
    let has_decoder = content_encoding
        .as_ref()
        .is_some_and(|enc| !matches!(enc.as_str(), "" | "identity"));
    let decoder = content_encoding
        .as_ref()
        .and_then(|enc| StreamingDecoder::new(enc).ok());
    let decoder_state: Arc<RwLock<Option<StreamingDecoder>>> = Arc::new(RwLock::new(decoder));

    // Clones for native_pull (pull closure moves the originals)
    let incoming_for_native = incoming.clone();
    let decoder_for_native = decoder_state.clone();

    let pull = PullAlgorithm::from_fn(
        move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
            let incoming = incoming.clone();
            let decoder_state = decoder_state.clone();

            let ctrl_class: ReadableStreamDefaultControllerClass = match controller {
                ReadableStreamControllerClass::ReadableStreamDefaultController(c) => c,
                _ => {
                    return Err(rquickjs::Exception::throw_type(
                        &ctx,
                        "Expected default controller",
                    ))
                },
            };

            // Create a promise that resolves when the async read completes.
            // The stream's pull mechanism awaits this promise before pulling again.
            let resolveable = ResolveablePromise::new(&ctx)?;
            let promise = resolveable.promise.clone();

            let ctx2 = ctx.clone();
            ctx.spawn_exit_simple(async move {
                // Take the body out of the shared state. If already taken
                // (e.g. by native_pull fast-path), resolve immediately.
                let mut body = match incoming.write().unwrap().take() {
                    Some(b) => b,
                    None => {
                        resolveable.resolve_undefined()?;
                        return Ok(());
                    },
                };

                match read_body_chunk(&mut body, &decoder_state, has_decoder).await {
                    Ok(Some(bytes)) => {
                        // Put the body back for the next pull, then enqueue the chunk
                        *incoming.write().unwrap() = Some(body);
                        let array = TypedArray::<u8>::new(ctx2.clone(), bytes)?;
                        readable_stream_default_controller_enqueue_value(
                            ctx2,
                            ctrl_class,
                            array.into_value(),
                        )?;
                    },
                    Ok(None) => {
                        // EOF — close the stream
                        readable_stream_default_controller_close_stream(ctx2, ctrl_class)?;
                    },
                    Err(msg) => {
                        // Propagate hyper/decompression errors to the JS stream
                        let v = rquickjs::String::from_str(ctx2.clone(), &msg)?.into_value();
                        readable_stream_default_controller_error_stream(ctrl_class, v)?;
                    },
                }
                resolveable.resolve_undefined()?;
                Ok(())
            });

            Ok(promise)
        },
    );

    let stream = ReadableStream::from_pull_algorithm(
        ctx.clone(),
        pull,
        CancelAlgorithm::ReturnPromiseUndefined,
    )?;

    // Set native_pull on controller for reader.read() fast-path
    {
        let incoming2 = incoming_for_native;
        let state2 = decoder_for_native;
        let np: Rc<llrt_stream_web::NativePullFn> = Rc::new(move |ctx: &rquickjs::Ctx<'js>| {
            let mut guard = incoming2.write().unwrap();
            let Some(body) = guard.as_mut() else {
                return Ok(llrt_stream_web::NativePullResult::Eof);
            };

            let waker = Waker::noop();
            let mut poll_cx = Context::from_waker(waker);

            loop {
                match Pin::new(&mut *body).poll_frame(&mut poll_cx) {
                    Poll::Ready(Some(Ok(frame))) => {
                        let Ok(first_data) = frame.into_data() else {
                            continue;
                        };

                        // Drain any additional synchronously-ready frames
                        let mut total_len = first_data.len();
                        let mut extras = Vec::new();
                        body.drain_ready(|data| {
                            total_len += data.len();
                            extras.push(data);
                        });

                        drop(guard);

                        let mut chunk = if extras.is_empty() {
                            None
                        } else {
                            let mut buf = Vec::with_capacity(total_len);
                            buf.extend_from_slice(&first_data);
                            for e in &extras {
                                buf.extend_from_slice(e);
                            }
                            Some(buf)
                        };

                        if has_decoder {
                            if let Some(d) = state2.write().unwrap().as_mut() {
                                let slice = chunk.as_deref().unwrap_or(&first_data);
                                match d.decompress_chunk(slice) {
                                    Ok(decompressed) => chunk = Some(decompressed),
                                    Err(e) => {
                                        return Err(Exception::throw_message(ctx, &e.to_string()))
                                    },
                                }
                            }
                        }

                        let val = match chunk {
                            Some(buf) => TypedArray::<u8>::new(ctx.clone(), buf)?.into_value(),
                            None => {
                                TypedArray::<u8>::new_copy(ctx.clone(), &first_data)?.into_value()
                            },
                        };

                        return Ok(NativePullResult::Ready(val));
                    },
                    Poll::Ready(Some(Err(e))) => {
                        *guard = None;
                        return Err(Exception::throw_message(ctx, &e.to_string()));
                    },
                    Poll::Ready(None) => {
                        *guard = None; // Release the body to free the HTTP connection
                        drop(guard);
                        if has_decoder {
                            if let Some(dec) = state2.write().unwrap().take() {
                                match dec.finish() {
                                    Ok(remaining) if !remaining.is_empty() => {
                                        let val = TypedArray::<u8>::new(ctx.clone(), remaining)?
                                            .into_value();
                                        return Ok(NativePullResult::Ready(val));
                                    },
                                    Err(e) => {
                                        return Err(Exception::throw_message(ctx, &e.to_string()))
                                    },
                                    _ => {},
                                }
                            }
                        }
                        return Ok(NativePullResult::Eof);
                    },
                    Poll::Pending => {
                        drop(guard);

                        let incoming = incoming2.clone();
                        let state = state2.clone();
                        let ctx = ctx.clone();

                        let fut = async move {
                            let Some(mut body) = incoming.write().unwrap().take() else {
                                return Ok(None);
                            };

                            match read_body_chunk(&mut body, &state, has_decoder).await {
                                Ok(Some(chunk)) => {
                                    *incoming.write().unwrap() = Some(body);
                                    Ok(Some(TypedArray::<u8>::new(ctx, chunk)?.into_value()))
                                },
                                Ok(None) => Ok(None),
                                Err(msg) => Err(Exception::throw_message(&ctx, &msg)),
                            }
                        };

                        return Ok(NativePullResult::Pending(Box::pin(fut)));
                    },
                }
            }
        });
        let s = stream.borrow();
        if let ReadableStreamControllerClass::ReadableStreamDefaultController(ref ctrl) =
            s.controller
        {
            ctrl.borrow_mut().native_pull = Some(llrt_stream_web::NativePull(np));
        }
    }

    Ok(stream.into_value())
}

#[cfg(test)]
mod tests {
    use llrt_test::{test_async_with_opts, TestOptions};
    use rquickjs::{prelude::This, CatchResultExt, Class, Function, Object, Promise};
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

                        let response_text: String =
                            Response::text(This(response.clone()), ctx.clone())?
                                .into_future()
                                .await?;
                        assert_eq!(response.borrow().status(), 200);
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
                llrt_stream_web::init(&ctx).unwrap();
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

                        // Cloning a response with unconsumed Incoming body should work via tee
                        let cloned = response.borrow().clone(ctx.clone())?;
                        let cloned = Class::instance(ctx.clone(), cloned)?;
                        let text1: String = Response::text(This(response.clone()), ctx.clone())?
                            .into_future()
                            .await?;
                        let text2: String = Response::text(This(cloned), ctx.clone())?
                            .into_future()
                            .await?;
                        assert_eq!(text1, "Hello");
                        assert_eq!(text2, "Hello");

                        // Cloning after body is consumed should fail
                        let clone_result = response.borrow().clone(ctx.clone());
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
    async fn test_response_compressed_body_stream() {
        use std::io::Read;

        let mock_server = MockServer::start().await;
        let message = "Hello, compressed stream!";

        let mut gzip_data: Vec<u8> = Vec::new();
        llrt_compression::gz::encoder(
            message.as_bytes(),
            llrt_compression::gz::Compression::default(),
        )
        .read_to_end(&mut gzip_data)
        .unwrap();

        Mock::given(matchers::path("compressed/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-encoding", "gzip")
                    .set_body_raw(gzip_data, "text/plain"),
            )
            .mount(&mock_server)
            .await;

        test_async_with_opts(
            |ctx| {
                llrt_stream_web::init(&ctx).unwrap();
                crate::init(&ctx).unwrap();
                Box::pin(async move {
                    let globals = ctx.globals();
                    let run = async {
                        let fetch: Function = globals.get("fetch")?;
                        let options = Object::new(ctx.clone())?;
                        options.set("method", "GET")?;
                        let url = format!("http://{}/compressed/", mock_server.address().clone());

                        let response_promise: Promise = fetch.call((url, options))?;
                        let response: Class<Response> = response_promise.into_future().await?;
                        let response = response.borrow();

                        // Read via body ReadableStream (not text())
                        let body_stream = response.body(ctx.clone())?;
                        let body_obj = body_stream.as_object().unwrap();
                        let get_reader: Function = body_obj.get("getReader")?;
                        let reader: Object =
                            get_reader.call((rquickjs::function::This(body_obj.clone()),))?;
                        let read_fn: Function = reader.get("read")?;

                        let mut result = Vec::new();
                        loop {
                            let promise: Promise =
                                read_fn.call((rquickjs::function::This(reader.clone()),))?;
                            let chunk: Object = promise.into_future().await?;
                            let done: bool = chunk.get("done").unwrap_or(true);
                            if done {
                                break;
                            }
                            let value: rquickjs::Value = chunk.get("value")?;
                            if let Ok(ta) = rquickjs::TypedArray::<u8>::from_value(value) {
                                if let Some(bytes) = ta.as_bytes() {
                                    result.extend_from_slice(bytes);
                                }
                            }
                        }

                        assert_eq!(String::from_utf8(result).unwrap(), message);
                        Ok(())
                    };
                    run.await.catch(&ctx).unwrap();
                })
            },
            TestOptions::new(),
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

                        let response_text: String =
                            Response::text(This(response.clone()), ctx.clone())?
                                .into_future()
                                .await?;
                        assert_eq!(response.borrow().status(), 200);
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
