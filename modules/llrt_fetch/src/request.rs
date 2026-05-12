// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use super::{
    headers::{Headers, HeadersGuard},
    Blob, FormData, MIME_TYPE_FORM_DATA, MIME_TYPE_FORM_URLENCODED, MIME_TYPE_OCTET_STREAM,
    MIME_TYPE_TEXT,
};
use crate::body_helpers::strip_bom;
use hyper::header::CONTENT_TYPE;
use llrt_abort::AbortSignal;
use llrt_http::Agent;
use llrt_json::parse::json_parse;
use llrt_stream_web::ReadableStream;
use llrt_url::{url_class::URL, url_search_params::URLSearchParams};
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, ArrayBuffer, Class, Coerced, Ctx, Exception,
    FromJs, IntoJs, Null, Object, Result, TypedArray, Value,
};
use std::sync::RwLock;

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
        Ok(match s.as_str() {
            "cors" => RequestMode::Cors,
            "no-cors" => RequestMode::NoCors,
            "same-origin" => RequestMode::SameOrigin,
            "navigate" => RequestMode::Navigate,
            _ => return Err(["Invalid request mode: ", s.as_str()].concat()),
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

pub(crate) enum BodyTaken<'js> {
    Bytes(Vec<u8>),
    Stream(Class<'js, ReadableStream<'js>>),
}

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct Request<'js> {
    url: String,
    method: String,
    headers: Option<Class<'js, Headers>>,
    body: RwLock<BodyVariant<'js>>,
    body_stream: RwLock<Option<Value<'js>>>,
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
        if let Some(stream) = self.body_stream.read().unwrap().as_ref() {
            stream.trace(tracer);
        }
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Request<'js> {
    #[qjs(constructor)]
    pub fn new(
        ctx: Ctx<'js>,
        this: rquickjs::prelude::This<Value<'js>>,
        input: Value<'js>,
        options: Opt<Value<'js>>,
    ) -> Result<Self> {
        // When called with `new`, rquickjs passes the constructor function as
        // `this`. When called without `new`, `this` is undefined (strict) or
        // the global object (sloppy). WPT's request-error.any.js requires
        // `Request("...")` to throw TypeError.
        if this.0.as_function().is_none() {
            return Err(Exception::throw_type(
                &ctx,
                "Failed to construct 'Request': Please use the 'new' operator",
            ));
        }
        let mut request = Self {
            url: "".into(),
            method: "GET".into(),
            headers: None,
            body: RwLock::new(BodyVariant::Empty),
            body_stream: RwLock::new(None),
            signal: None,
            mode: RequestMode::Cors,
            keepalive: false,
            agent: None,
        };

        // If the input is a Request, we may need to tee its body at the very
        // end so construction failures (e.g. GET with body) don't leave the
        // input disturbed. This holds the input request and whether init.body
        // overrides the body.
        let mut input_request_to_disturb: Option<(Class<'js, Request<'js>>, bool)> = None;

        if input.is_string() {
            let s: String = input.get()?;
            // Validate as a URL; accept relative URLs against a generic base.
            if !s.is_empty() {
                let base = url::Url::parse("http://llrt.local/").expect("static base URL");
                let parsed = base
                    .join(&s)
                    .map_err(|_| Exception::throw_type(&ctx, "Invalid URL"))?;
                if !parsed.username().is_empty() || parsed.password().is_some() {
                    return Err(Exception::throw_type(&ctx, "URL must not have credentials"));
                }
            }
            request.url = s;
        } else if let Ok(url) = URL::from_js(&ctx, input.clone()) {
            request.url = url.to_string();
        } else if input.is_object() {
            let obj = input.as_object().expect("input is an object");
            // Check if input is a Request - if so, transfer body (mark original as used)
            if let Some(input_request) = Class::<Request>::from_object(obj) {
                let input_req = input_request.borrow();
                let init_overrides_body = options
                    .0
                    .as_ref()
                    .and_then(|v| v.as_object())
                    .and_then(|o| o.get::<_, Value>("body").ok())
                    .is_some_and(|v| !v.is_undefined() && !v.is_null());
                if !init_overrides_body && is_unusable_body(&input_req) {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Cannot construct a Request from a disturbed or locked body",
                    ));
                }
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
                drop(input_req);
                input_request_to_disturb = Some((input_request, init_overrides_body));
            } else {
                assign_request(&mut request, ctx.clone(), obj)?;
            }
        }
        if let Some(options) = options.0 {
            if let Some(obj) = options.as_object() {
                assign_request(&mut request, ctx.clone(), obj)?;
            }
        }
        // Validate: GET/HEAD requests cannot have a body (even when the body
        // was inherited from an input Request).
        if matches!(request.method.as_str(), "GET" | "HEAD") {
            let has_body = matches!(
                &*request.body.read().unwrap(),
                BodyVariant::Provided(Some(_))
            ) || input_request_to_disturb.as_ref().is_some_and(
                |(r, override_body)| {
                    !override_body
                        && matches!(
                            &*r.borrow().body.read().unwrap(),
                            BodyVariant::Provided(Some(_))
                        )
                },
            );
            if has_body {
                return Err(Exception::throw_type(
                    &ctx,
                    "Failed to construct 'Request': Request with GET/HEAD method cannot have body.",
                ));
            }
        }
        // Now that all validation has passed, disturb the input's body (if it
        // was a Request). The input's body field is cleared (marking bodyUsed
        // = true) while its cached body_stream is preserved so that the input's
        // `body` getter keeps returning the same object it did before.
        if let Some((input_request, init_overrides_body)) = input_request_to_disturb {
            let input_req = input_request.borrow();
            if let Ok(body_value) = input_req.body(ctx.clone()) {
                if let Some(stream) = body_value
                    .as_object()
                    .and_then(Class::<ReadableStream>::from_object)
                {
                    let is_unusable =
                        stream.borrow().is_readable_stream_locked() || stream.borrow().disturbed;
                    if !is_unusable {
                        let (_b1, branch2) =
                            llrt_stream_web::tee_readable_stream(ctx.clone(), stream)?;
                        if !init_overrides_body {
                            *request.body.write().unwrap() =
                                BodyVariant::Provided(Some(branch2.into_value()));
                        }
                    }
                }
            }
            // Mark input's body as consumed without clearing the cached stream
            // reference, so `bodyRequest.body === originalBody` still holds.
            *input_req.body.write().unwrap() = BodyVariant::Provided(None);
            drop(input_req);
        }
        if request.headers.is_none() {
            let guard = if request.mode == RequestMode::NoCors {
                HeadersGuard::RequestNoCors
            } else {
                HeadersGuard::Request
            };
            let mut default_headers = Headers::default();
            default_headers.guard = guard;
            let headers = Class::instance(ctx, default_headers)?;
            request.headers = Some(headers);
        }

        Ok(request)
    }

    #[qjs(get)]
    fn url(&self) -> &str {
        &self.url
    }

    #[qjs(get)]
    fn method(&self) -> &str {
        &self.method
    }

    #[qjs(get)]
    fn headers(&self) -> Option<Class<'js, Headers>> {
        self.headers.clone()
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(Request)
    }

    #[qjs(get)]
    fn body(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        // Return cached stream if available
        if let Some(stream) = self.body_stream.read().unwrap().as_ref() {
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

                let stream = crate::body_helpers::create_body_value_stream(&ctx, body_value)?;
                *self.body_stream.write().unwrap() = Some(stream.clone());
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
        if crate::body_helpers::is_body_stream_disturbed(&self.body_stream) {
            return true;
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
    fn destination(&self) -> &'static str {
        ""
    }

    #[qjs(get)]
    fn referrer(&self) -> &'static str {
        "about:client"
    }

    #[qjs(get, rename = "referrerPolicy")]
    fn referrer_policy(&self) -> &'static str {
        ""
    }

    #[qjs(get)]
    fn credentials(&self) -> &'static str {
        "same-origin"
    }

    #[qjs(get)]
    fn integrity(&self) -> &'static str {
        ""
    }

    #[qjs(get)]
    fn redirect(&self) -> &'static str {
        "follow"
    }

    #[qjs(get, rename = "isReloadNavigation")]
    fn is_reload_navigation(&self) -> bool {
        false
    }

    #[qjs(get, rename = "isHistoryNavigation")]
    fn is_history_navigation(&self) -> bool {
        false
    }

    #[qjs(get)]
    fn duplex(&self) -> &'static str {
        "half"
    }

    #[qjs(get)]
    fn cache(&self) -> &'static str {
        "default"
    }

    #[qjs(get)]
    fn agent(&self) -> Option<Class<'js, Agent>> {
        self.agent.clone()
    }

    pub fn text(&self, ctx: Ctx<'js>) -> Result<rquickjs::Promise<'js>> {
        let body = match self.take_body_sync(&ctx) {
            Ok(b) => b,
            Err(e) => return reject_with_error(&ctx, e),
        };
        let ctx_clone = ctx.clone();
        rquickjs::Promise::wrap_future(&ctx, async move {
            let bytes_opt = resolve_body_taken(&ctx_clone, body).await?;
            if let Some(bytes) = bytes_opt {
                let bytes = strip_bom(bytes);
                return Result::<String>::Ok(match String::from_utf8(bytes.into()) {
                    Ok(s) => s,
                    Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
                });
            }
            Ok(String::new())
        })
    }

    pub fn json(&self, ctx: Ctx<'js>) -> Result<rquickjs::Promise<'js>> {
        let body = match self.take_body_sync(&ctx) {
            Ok(b) => b,
            Err(e) => return reject_with_error(&ctx, e),
        };
        let ctx_clone = ctx.clone();
        rquickjs::Promise::wrap_future(&ctx, async move {
            let bytes_opt = resolve_body_taken(&ctx_clone, body).await?;
            if let Some(bytes) = bytes_opt {
                return json_parse(&ctx_clone, strip_bom(bytes));
            }
            Err(Exception::throw_syntax(&ctx_clone, "JSON input is empty"))
        })
    }

    fn array_buffer(&self, ctx: Ctx<'js>) -> Result<rquickjs::Promise<'js>> {
        let body = match self.take_body_sync(&ctx) {
            Ok(b) => b,
            Err(e) => return reject_with_error(&ctx, e),
        };
        let ctx_clone = ctx.clone();
        rquickjs::Promise::wrap_future(&ctx, async move {
            let bytes_opt = resolve_body_taken(&ctx_clone, body).await?;
            if let Some(bytes) = bytes_opt {
                return ArrayBuffer::new(ctx_clone, bytes);
            }
            ArrayBuffer::new(ctx_clone, Vec::<u8>::new())
        })
    }

    fn bytes(&self, ctx: Ctx<'js>) -> Result<rquickjs::Promise<'js>> {
        let body = match self.take_body_sync(&ctx) {
            Ok(b) => b,
            Err(e) => return reject_with_error(&ctx, e),
        };
        let ctx_clone = ctx.clone();
        rquickjs::Promise::wrap_future(&ctx, async move {
            let bytes_opt = resolve_body_taken(&ctx_clone, body).await?;
            if let Some(bytes) = bytes_opt {
                return TypedArray::new(ctx_clone, bytes).map(|m| m.into_value());
            }
            TypedArray::new(ctx_clone, Vec::<u8>::new()).map(|m| m.into_value())
        })
    }

    fn blob(&self, ctx: Ctx<'js>) -> Result<rquickjs::Promise<'js>> {
        let mime_type = self.get_header_value(&ctx, CONTENT_TYPE.as_str())?;
        let body = match self.take_body_sync(&ctx) {
            Ok(b) => b,
            Err(e) => return reject_with_error(&ctx, e),
        };
        let ctx_clone = ctx.clone();
        rquickjs::Promise::wrap_future(&ctx, async move {
            let bytes_opt = resolve_body_taken(&ctx_clone, body).await?;
            let bytes = bytes_opt.unwrap_or_default();
            Blob::from_bytes(&ctx_clone, bytes, mime_type)
        })
    }

    async fn form_data(&self, ctx: Ctx<'js>) -> Result<FormData<'js>> {
        let mime_type = self
            .get_header_value(&ctx, CONTENT_TYPE.as_str())?
            .unwrap_or(MIME_TYPE_OCTET_STREAM.into());

        let is_multipart = mime_type.starts_with("multipart/form-data");
        let is_urlencoded =
            mime_type.starts_with(MIME_TYPE_FORM_URLENCODED.split(';').next().unwrap_or(""));
        if !is_multipart && !is_urlencoded {
            return Err(Exception::throw_type(
                &ctx,
                "formData: invalid Content-Type",
            ));
        }

        if let Some(bytes) = resolve_body_taken(&ctx, self.take_body_sync(&ctx)?).await? {
            let form_data = FormData::from_multipart_bytes(&ctx, &mime_type, bytes)?;
            return Ok(form_data);
        }
        // Empty body: reject only for multipart (parser requires boundary).
        if is_multipart {
            return Err(Exception::throw_type(&ctx, "formData: body is empty"));
        }
        Ok(FormData::default())
    }

    fn clone(&self, ctx: Ctx<'js>) -> Result<Self> {
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
                let Some(provided) = provided else {
                    return Err(Exception::throw_type(
                        &ctx,
                        "Cannot clone request after body has been used",
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
                    drop(body_guard);
                    let mut body_write = self.body.write().unwrap();
                    *body_write = BodyVariant::Provided(Some(branch1.into_value()));
                    *self.body_stream.write().unwrap() = None;

                    return Ok(Self {
                        url: self.url.clone(),
                        method: self.method.clone(),
                        headers,
                        body: RwLock::new(BodyVariant::Provided(Some(branch2.into_value()))),
                        body_stream: RwLock::new(None),
                        signal: self.signal.clone(),
                        mode: self.mode.clone(),
                        keepalive: self.keepalive,
                        agent: self.agent.clone(),
                    });
                }
                BodyVariant::Provided(Some(provided.clone()))
            },
            BodyVariant::Empty => BodyVariant::Empty,
        };
        drop(body_guard);

        Ok(Self {
            url: self.url.clone(),
            method: self.method.clone(),
            headers,
            body: RwLock::new(cloned_body),
            body_stream: RwLock::new(None),
            signal: self.signal.clone(),
            mode: self.mode.clone(),
            keepalive: self.keepalive,
            agent: self.agent.clone(),
        })
    }
}

impl<'js> Request<'js> {
    /// Synchronously takes the body (marking bodyUsed) and returns either
    /// ready bytes or a ReadableStream to be consumed asynchronously.
    pub(crate) fn take_body_sync(&self, ctx: &Ctx<'js>) -> Result<Option<BodyTaken<'js>>> {
        if let Some(stream_val) = self.body_stream.read().unwrap().as_ref() {
            if let Some(stream) = stream_val
                .as_object()
                .and_then(Class::<ReadableStream>::from_object)
            {
                crate::body_helpers::validate_stream_usable(ctx, &stream, "read body")?;
            }
        }

        let mut body_guard = self.body.write().unwrap();
        match &mut *body_guard {
            BodyVariant::Provided(provided) => {
                let provided = provided
                    .take()
                    .ok_or(Exception::throw_type(ctx, "Body is already read"))?;
                drop(body_guard);

                if let Some(stream) = provided
                    .as_object()
                    .and_then(Class::<ReadableStream>::from_object)
                {
                    return Ok(Some(BodyTaken::Stream(stream)));
                }

                let bytes =
                    if let Some(blob) = provided.as_object().and_then(Class::<Blob>::from_object) {
                        blob.borrow().get_bytes()
                    } else {
                        let bytes = ObjectBytes::from(ctx, &provided)?;
                        bytes.as_bytes(ctx)?.to_vec()
                    };
                Ok(Some(BodyTaken::Bytes(bytes)))
            },
            BodyVariant::Empty => Ok(None),
        }
    }

    fn get_header_value(&self, _ctx: &Ctx<'js>, key: &str) -> Result<Option<String>> {
        let Some(headers) = self.headers.as_ref() else {
            return Ok(None);
        };
        Ok(headers
            .borrow()
            .iter()
            .find_map(|(k, v)| (k == key).then(|| v.to_string())))
    }
}

/// Collects all data from a ReadableStream into a Vec<u8>
async fn collect_readable_stream<'js>(
    _ctx: &Ctx<'js>,
    stream: &Class<'js, ReadableStream<'js>>,
) -> Result<Vec<u8>> {
    crate::body_helpers::collect_readable_stream(stream).await
}

fn reject_with_error<'js>(ctx: &Ctx<'js>, err: rquickjs::Error) -> Result<rquickjs::Promise<'js>> {
    // Turn a synchronous throw into a rejected Promise. Extract the actual
    // JS exception value from the Ctx's pending-exception slot so that the
    // caller sees a real TypeError (or whatever) object — not an uninitialized
    // value (which `promise_rejects_js` checks for in WPT).
    use llrt_stream_web::utils::promise::{promise_rejected_with, PromisePrimordials};
    use llrt_utils::primordials::Primordial;
    let _ = err; // err is `Error::Exception`; the real value is in ctx.catch().
    let exception_value = ctx.catch();
    let primordials = PromisePrimordials::get(ctx)?;
    promise_rejected_with(&primordials, exception_value)
}

async fn resolve_body_taken<'js>(
    ctx: &Ctx<'js>,
    body: Option<BodyTaken<'js>>,
) -> Result<Option<Vec<u8>>> {
    match body {
        None => Ok(None),
        Some(BodyTaken::Bytes(bytes)) => Ok(Some(bytes)),
        Some(BodyTaken::Stream(stream)) => collect_readable_stream(ctx, &stream).await.map(Some),
    }
}

/// True if the request's body has been disturbed (read from) or locked.
/// Covers both cases that disqualify `new Request(existingReq)`.
fn is_unusable_body(req: &Request<'_>) -> bool {
    if req.body_used() {
        return true;
    }
    if let Some(stream_val) = req.body_stream.read().unwrap().as_ref() {
        if let Some(stream) = stream_val
            .as_object()
            .and_then(Class::<ReadableStream>::from_object)
        {
            return stream.borrow().is_readable_stream_locked();
        }
    }
    false
}

fn assign_request<'js>(request: &mut Request<'js>, ctx: Ctx<'js>, obj: &Object<'js>) -> Result<()> {
    if let Some(url) = obj.get_optional("url")? {
        request.url = url;
    }
    if let Some(method) = obj.get_optional::<_, String>("method")? {
        // Validate HTTP token per RFC 7230 (no whitespace, CTLs or separators)
        if method.is_empty()
            || !method.bytes().all(|b| {
                matches!(b,
                b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.'
                | b'^' | b'_' | b'`' | b'|' | b'~' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z')
            })
        {
            return Err(Exception::throw_type(&ctx, "Invalid HTTP method name"));
        }
        let method = method.to_ascii_uppercase();
        if let "CONNECT" | "TRACE" | "TRACK" = method.as_str() {
            return Err(Exception::throw_type(&ctx, "This method is not allowed."));
        }
        request.method = method;
    }
    if let Some(mode) = obj.get_optional::<_, String>("mode")? {
        if mode == "navigate" {
            return Err(Exception::throw_type(
                &ctx,
                "Cannot construct a Request with a RequestInit whose mode member is set as 'navigate'.",
            ));
        }
        if mode == "no-cors" && !matches!(request.method.as_str(), "GET" | "HEAD" | "POST") {
            return Err(Exception::throw_type(
                &ctx,
                "'no-cors' mode requires the method to be GET, HEAD, or POST",
            ));
        }
        if let Some(cache) = obj.get_optional::<_, String>("cache")? {
            if cache == "only-if-cached" && mode != "same-origin" {
                return Err(Exception::throw_type(
                    &ctx,
                    "Cache mode 'only-if-cached' requires mode to be 'same-origin'.",
                ));
            }
        }
        request.mode = mode
            .try_into()
            .or_throw_type(&ctx, "Invalid request mode")?;
    }
    if let Some(referrer) = obj.get_optional::<_, String>("referrer")? {
        if !referrer.is_empty() && referrer != "about:client" {
            // Must be a valid URL
            if !llrt_url::url_class::URL::is_valid(&referrer) {
                return Err(Exception::throw_type(&ctx, "Referrer is not a valid URL."));
            }
        }
    }
    if let Some(rp) = obj.get_optional::<_, String>("referrerPolicy")? {
        if !matches!(
            rp.as_str(),
            "" | "no-referrer"
                | "no-referrer-when-downgrade"
                | "same-origin"
                | "origin"
                | "strict-origin"
                | "origin-when-cross-origin"
                | "strict-origin-when-cross-origin"
                | "unsafe-url"
        ) {
            return Err(Exception::throw_type(&ctx, "Invalid referrerPolicy"));
        }
    }
    if let Some(credentials) = obj.get_optional::<_, String>("credentials")? {
        if !matches!(credentials.as_str(), "omit" | "same-origin" | "include") {
            return Err(Exception::throw_type(&ctx, "Invalid credentials"));
        }
    }
    if let Some(cache) = obj.get_optional::<_, String>("cache")? {
        if !matches!(
            cache.as_str(),
            "default" | "no-store" | "reload" | "no-cache" | "force-cache" | "only-if-cached"
        ) {
            return Err(Exception::throw_type(&ctx, "Invalid cache mode"));
        }
    }
    if let Some(redirect) = obj.get_optional::<_, String>("redirect")? {
        if !matches!(redirect.as_str(), "follow" | "error" | "manual") {
            return Err(Exception::throw_type(&ctx, "Invalid redirect mode"));
        }
    }
    if let Some(duplex) = obj.get_optional::<_, Value>("duplex")? {
        if !duplex.is_undefined() && !duplex.is_null() {
            let d: String = duplex.get()?;
            if !matches!(d.as_str(), "half" | "full") {
                return Err(Exception::throw_type(&ctx, "Invalid duplex"));
            }
            if d == "full" {
                return Err(Exception::throw_type(
                    &ctx,
                    "duplex 'full' is not supported",
                ));
            }
        }
    }
    if let Some(priority) = obj.get_optional::<_, Value>("priority")? {
        if !priority.is_undefined() && !priority.is_null() {
            let p: String = priority.get()?;
            if !matches!(p.as_str(), "high" | "low" | "auto") {
                return Err(Exception::throw_type(&ctx, "Invalid priority"));
            }
        }
    }
    if let Some(window) = obj.get_optional::<_, Value>("window")? {
        if !window.is_null() {
            return Err(Exception::throw_type(&ctx, "window must be null"));
        }
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
                    // Per spec, `duplex` must be "half" when body is a stream.
                    // Skip when the init object is itself a Request (body comes
                    // from the Request's body getter, not a user-supplied stream).
                    let is_request_init = Class::<Request>::from_object(obj).is_some();
                    if !is_request_init {
                        let duplex: Option<String> = obj.get_optional("duplex")?;
                        if duplex.as_deref() != Some("half") {
                            return Err(Exception::throw_type(
                                &ctx,
                                "Request with stream body requires duplex: 'half'",
                            ));
                        }
                    }
                    if request.keepalive {
                        return Err(Exception::throw_type(
                            &ctx,
                            "keepalive requests cannot have a streaming body",
                        ));
                    }
                    crate::body_helpers::validate_stream_usable(
                        &ctx,
                        &stream,
                        "construct Request",
                    )?;
                    *request.body_stream.write().unwrap() = Some(body.clone());
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
                } else if ArrayBuffer::from_value(body.clone()).is_some()
                    || body_obj.get::<_, Value>("buffer").is_ok_and(|b| {
                        b.as_object()
                            .is_some_and(|o| ArrayBuffer::from_object(o.clone()).is_some())
                    })
                {
                    BodyVariant::Provided(Some(body))
                } else {
                    // WebIDL: fall back to stringifying the value (USVString body).
                    let s: String = Coerced::from_js(&ctx, body.clone())?.0;
                    content_type = Some(MIME_TYPE_TEXT.into());
                    BodyVariant::Provided(Some(s.into_js(&ctx)?))
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
        if !headers.contains_lower(CONTENT_TYPE.as_str()) {
            if let Some(value) = content_type {
                headers.set(
                    ctx.clone(),
                    CONTENT_TYPE.as_str().into(),
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
