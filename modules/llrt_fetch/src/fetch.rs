// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, Full};
use hyper::{
    header::{
        ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, AUTHORIZATION, CONTENT_ENCODING,
        CONTENT_LANGUAGE, CONTENT_LENGTH, CONTENT_LOCATION, CONTENT_TYPE, LOCATION, ORIGIN,
        USER_AGENT,
    },
    Method, Request, Uri,
};
use llrt_abort::AbortSignal;
use llrt_context::CtxExtension;
use llrt_encoding::bytes_from_b64;
use llrt_http::Agent;
use llrt_http::HyperClient;
use llrt_stream_web::ReadableStream;
use llrt_utils::{bytes::ObjectBytes, mc_oneshot, result::ResultExt, VERSION};
use percent_encoding::percent_decode_str;
use rquickjs::{
    atom::PredefinedAtom,
    function::{Opt, This},
    prelude::{Async, Func},
    CatchResultExt, CaughtError, Class, Coerced, Ctx, Exception, FromJs, Function, IntoJs, Object,
    Result, Value,
};
use std::{
    collections::HashSet,
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Instant,
};
use tokio::{select, sync::Semaphore};

use super::{
    form_data::FormData,
    headers::{Headers, HeadersGuard},
    response::Response,
    security::ensure_url_access,
    Blob, MIME_TYPE_FORM_DATA, MIME_TYPE_FORM_URLENCODED, MIME_TYPE_TEXT,
};
use llrt_url::url_search_params::URLSearchParams;

// https://fetch.spec.whatwg.org/#port-blocking
const BLOCKED_PORTS: [u16; 83] = [
    0, 1, 7, 9, 11, 13, 15, 17, 19, 20, 21, 22, 23, 25, 37, 42, 43, 53, 69, 77, 79, 87, 95, 101,
    102, 103, 104, 109, 110, 111, 113, 115, 117, 119, 123, 135, 137, 139, 143, 161, 179, 389, 427,
    465, 512, 513, 514, 515, 526, 530, 531, 532, 540, 548, 554, 556, 563, 587, 601, 636, 989, 990,
    993, 995, 1719, 1720, 1723, 2049, 3659, 4045, 4190, 5060, 5061, 6000, 6566, 6665, 6666, 6667,
    6668, 6669, 6679, 6697, 10080,
];

const MAX_REDIRECT_COUNT: u32 = 20;

pub fn init(global_client: HyperClient, globals: &Object) -> Result<()> {
    let connections = Arc::new(Semaphore::new(500));

    globals.set(
        "fetch",
        Func::from(Async(move |ctx, resource, args| {
            let global_client = global_client.clone();
            let connections = connections.clone();
            let start = Instant::now();
            let options = get_fetch_options(&ctx, resource, args);

            async move {
                let _lock = connections.acquire().await;
                let options = options?;

                let client = options
                    .agent
                    .map(|agent| agent.borrow().client())
                    .unwrap_or(global_client);

                // https://fetch.spec.whatwg.org/#scheme-fetch
                if let Some((scheme, fragment)) = options.url.split_once(':') {
                    match scheme {
                        "http" | "https" => {},
                        "data" => return parse_data_url(&ctx, fragment, &options.method),
                        "about" | "blob" | "file" => {
                            return Err(Exception::throw_type(&ctx, "Unsupported scheme"));
                        },
                        _ => return Err(Exception::throw_type(&ctx, "Invalid scheme")),
                    }
                }

                let mut uri = options.url.parse::<Uri>().map_err(|_| {
                    Exception::throw_type(&ctx, &["Invalid URL :", &options.url].concat())
                })?;
                let initial_uri: Uri = uri.clone();

                let method_string = options.method.to_string();
                let method = options.method;
                let abort_receiver = options.abort_receiver;

                ensure_url_access(&ctx, &uri)?;

                // For streaming bodies - stream from JS to hyper via channel
                if let Some(RequestBody::Stream(stream)) = options.body {
                    return send_stream(
                        &ctx,
                        &client,
                        stream,
                        method,
                        method_string,
                        uri,
                        options.headers.as_ref(),
                        start,
                        abort_receiver,
                    )
                    .await;
                }

                let body = options.body;
                let integrity_entries = options
                    .integrity
                    .as_deref()
                    .map(crate::integrity::parse_integrity)
                    .unwrap_or_default();

                let res = send(
                    &ctx,
                    &client,
                    method,
                    &mut uri,
                    &initial_uri,
                    options.headers.as_ref(),
                    body.as_ref(),
                    abort_receiver.as_ref(),
                    &options.redirect,
                )
                .await?;

                let (res, redirected, guard) = res;

                // Subresource Integrity check: if integrity metadata was
                // provided and at least one entry parsed, buffer the body,
                // hash, and reject with TypeError on mismatch. The
                // verified bytes are then surfaced as the Response body.
                if !integrity_entries.is_empty() {
                    use http_body_util::BodyExt;
                    let (parts, body) = res.into_parts();
                    let collected = body.collect().await.or_throw_type(&ctx, "Fetch failed")?;
                    let bytes = collected.to_bytes().to_vec();
                    if !crate::integrity::verify(&integrity_entries, &bytes) {
                        return Err(Exception::throw_type(
                            &ctx,
                            "Failed integrity metadata check",
                        ));
                    }
                    return Response::from_verified_bytes(
                        ctx,
                        parts,
                        bytes,
                        method_string,
                        uri.to_string(),
                        start,
                        redirected,
                        abort_receiver,
                        guard,
                    );
                }

                Response::from_incoming(
                    ctx,
                    res,
                    method_string,
                    uri.to_string(),
                    start,
                    redirected,
                    abort_receiver,
                    guard,
                )
            }
        })),
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn send_stream<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    stream: Class<'js, ReadableStream<'js>>,
    method: Method,
    method_string: String,
    uri: Uri,
    headers: Option<&Headers>,
    start: Instant,
    abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
) -> Result<Response<'js>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(128);
    let (err_tx, err_rx) = tokio::sync::oneshot::channel::<String>();
    let ctx2 = ctx.clone();

    // Spawn stream reader in JS context
    ctx.spawn_exit_simple({
        async move {
            let get_reader: Function = stream.get("getReader")?;
            let reader: Object = get_reader.call((This(stream.clone()),))?;
            let read_fn: Function = reader.get("read")?;

            loop {
                let promise: rquickjs::Promise = read_fn.call((This(reader.clone()),))?;
                let read_result = match promise.into_future::<Object>().await.catch(&ctx2) {
                    Ok(r) => r,
                    Err(e) => {
                        let msg = match e {
                            CaughtError::Exception(ex) => ex.message().unwrap_or_default(),
                            CaughtError::Value(v) => v
                                .as_string()
                                .and_then(|s| s.to_string().ok())
                                .unwrap_or_else(|| "Stream error".into()),
                            CaughtError::Error(e) => e.to_string(),
                        };
                        let _ = err_tx.send(msg);
                        break;
                    },
                };
                let done: bool = read_result.get("done").unwrap_or(true);
                if done {
                    break;
                }
                if let Ok(value) = read_result.get::<_, Value>("value") {
                    // Per fetch spec, stream chunks must be Uint8Array. Anything
                    // else (ArrayBuffer, Blob, String, null, etc.) is an error.
                    let bytes = match rquickjs::TypedArray::<u8>::from_value(value) {
                        Ok(typed_array) => typed_array.as_bytes().map(Bytes::copy_from_slice),
                        Err(_) => {
                            let _ = err_tx
                                .send("Failed to read body: chunk is not a Uint8Array".into());
                            break;
                        },
                    };
                    if let Some(bytes) = bytes {
                        if tx.send(bytes).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Ok(())
        }
    });

    // Build request with streaming body
    let stream_body = StreamingBody { rx };
    let mut req = Request::builder().method(method.clone()).uri(uri.clone());
    let mut detected_headers = HashSet::new();
    if let Some(headers) = headers {
        for (header_name, value) in headers.iter() {
            detected_headers.insert(header_name);
            req = req.header(header_name, value)
        }
    }
    apply_default_headers(&mut req, &detected_headers);

    let box_body: BoxBody<Bytes, Infallible> = BoxBody::new(stream_body);
    let request = req.body(box_body).or_throw(ctx)?;

    // Race request against stream error
    let request_fut = client.request(request);
    tokio::pin!(request_fut);
    let mut err_rx = std::pin::pin!(err_rx);
    let mut err_done = false;

    let res = loop {
        select! {
            biased;
            result = &mut err_rx, if !err_done => {
                err_done = true;
                if let Ok(msg) = result {
                    return Err(Exception::throw_type(ctx, &msg));
                }
            }
            res = &mut request_fut => break res.or_throw(ctx)?,
        }
    };

    Response::from_incoming(
        ctx.clone(),
        res,
        method_string,
        uri.to_string(),
        start,
        false,
        abort_receiver,
        HeadersGuard::Response,
    )
}

async fn dispatch<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    req: Request<BoxBody<Bytes, Infallible>>,
    abort_receiver: Option<&mc_oneshot::Receiver<Value<'js>>>,
) -> Result<hyper::Response<hyper::body::Incoming>> {
    if let Some(abort_receiver) = abort_receiver {
        select! {
            res = client.request(req) => res.or_throw_type(ctx, "Fetch failed"),
            reason = abort_receiver.recv() => Err(ctx.throw(reason)),
        }
    } else {
        client.request(req).await.or_throw_type(ctx, "Fetch failed")
    }
}

#[allow(clippy::too_many_arguments)]
async fn send<'js>(
    ctx: &Ctx<'js>,
    client: &HyperClient,
    method: Method,
    uri: &mut Uri,
    initial_uri: &Uri,
    headers: Option<&Headers>,
    body: Option<&RequestBody<'js>>,
    abort_receiver: Option<&mc_oneshot::Receiver<Value<'js>>>,
    redirect: &str,
) -> Result<(hyper::Response<hyper::body::Incoming>, bool, HeadersGuard)> {
    let mut redirect_count = 0;
    let mut response_status = 0;

    let (res, guard) = loop {
        let (req, guard) = build_request(
            ctx,
            &method,
            uri,
            headers,
            body,
            &response_status,
            initial_uri,
        )?;

        let res = dispatch(ctx, client, req, abort_receiver).await?;

        let status = res.status();

        // Per WHATWG Fetch §4.5 step 15.4: 421 on a non-GET/HEAD request retries once on a fresh connection.
        if status.as_u16() == 421 && !matches!(method, Method::GET | Method::HEAD) {
            drop(res);
            // Force a new connection: open a new hyper client whose pool can't share TCP conns with the global one.
            let new_client = llrt_http::build_client(None)
                .or_throw_type(ctx, "Fetch failed: could not build retry client")?;
            let (req, _) = build_request(
                ctx,
                &method,
                uri,
                headers,
                body,
                &response_status,
                initial_uri,
            )?;
            let retried = dispatch(ctx, &new_client, req, abort_receiver).await?;
            break (retried, guard);
        }

        if status.is_redirection() {
            match res.headers().get(&LOCATION) {
                Some(location_headers) => {
                    if let Ok(location_str) = location_headers.to_str() {
                        *uri = location_str.parse().or_throw(ctx)?;
                        ensure_url_access(ctx, uri)?;
                    }
                },
                None => break (res, guard),
            };
        } else {
            break (res, guard);
        };

        if redirect == "manual" {
            break (res, guard);
        } else if redirect == "error" {
            return Err(Exception::throw_message(ctx, "Unexpected redirect"));
        }

        redirect_count += 1;
        if redirect_count >= MAX_REDIRECT_COUNT {
            return Err(Exception::throw_message(ctx, "Max retries exceeded"));
        }

        response_status = res.status().as_u16();
    };

    Ok((res, redirect_count > 0, guard))
}

fn apply_default_headers(
    req: &mut hyper::http::request::Builder,
    detected_headers: &HashSet<&str>,
) {
    if !detected_headers.contains(USER_AGENT.as_str()) {
        *req = std::mem::take(req).header(USER_AGENT, ["llrt ", VERSION].concat());
    }
    if !detected_headers.contains(ACCEPT_ENCODING.as_str()) {
        *req = std::mem::take(req).header(ACCEPT_ENCODING, "zstd, br, gzip, deflate");
    }
    if !detected_headers.contains(ACCEPT_LANGUAGE.as_str()) {
        *req = std::mem::take(req).header(ACCEPT_LANGUAGE, "*");
    }
    if !detected_headers.contains(ACCEPT.as_str()) {
        *req = std::mem::take(req).header(ACCEPT, "*/*");
    }
}

fn parse_data_url<'js>(ctx: &Ctx<'js>, data_url: &str, method: &Method) -> Result<Response<'js>> {
    let (mime_type, data) = data_url
        .split_once(',')
        .ok_or_else(|| Exception::throw_type(ctx, "Invalid data URL format"))?;

    let mut is_base64 = false;
    let mut content_type = String::with_capacity(10);
    for (i, part) in mime_type.split(';').enumerate() {
        let part = part.trim();
        if i == 1 || i == 2 {
            if part == "base64" {
                is_base64 = true;
                break;
            }
            content_type.push(';');
        }
        content_type.push_str(part)
    }

    let content_type = if content_type.starts_with(';') {
        ["text/plain", &content_type].concat()
    } else if content_type.is_empty() {
        "text/plain;charset=US-ASCII".to_string()
    } else {
        content_type
    };

    let body = if method == Method::HEAD {
        vec![]
    } else if is_base64 {
        bytes_from_b64(data.as_bytes()).or_throw(ctx)?
    } else {
        let data = percent_decode_str(data).decode_utf8().or_throw(ctx)?;
        data.as_bytes().into()
    };

    let blob = Blob::from_bytes(ctx, body, Some(content_type.clone()))?.into_js(ctx)?;

    let headers = Object::new(ctx.clone())?;
    headers.set(CONTENT_TYPE.as_str(), content_type)?;

    let options = Object::new(ctx.clone())?;
    options.set("url", data_url)?;
    options.set("headers", headers)?;
    // data: URL spec requires "OK" reason phrase.
    options.set("statusText", "OK")?;

    Response::new(ctx.clone(), Opt(Some(blob)), Opt(Some(options)))
}

fn build_request<'js>(
    ctx: &Ctx<'js>,
    method: &hyper::Method,
    uri: &Uri,
    headers: Option<&Headers>,
    body: Option<&RequestBody<'js>>,
    prev_status: &u16,
    initial_uri: &Uri,
) -> Result<(Request<BoxBody<Bytes, Infallible>>, HeadersGuard)> {
    if let Some(scheme) = uri.scheme_str() {
        if !matches!(scheme, "http" | "https") {
            return Err(Exception::throw_type(ctx, "Invalid scheme in URL"));
        }

        if let ("http", Some(port)) = (scheme, uri.authority().and_then(|a| a.port_u16())) {
            if BLOCKED_PORTS.contains(&port) {
                return Err(Exception::throw_type(ctx, "Invalid port in URL"));
            }
        }
    }

    let same_origin = is_same_origin(uri, initial_uri);
    let guard = HeadersGuard::Immutable;

    let change_method = should_change_method(*prev_status, method);

    let (method_to_use, body) = if change_method {
        (Method::GET, None)
    } else {
        (method.clone(), body)
    };
    let is_get_or_head = matches!(method_to_use, Method::GET | Method::HEAD);

    let mut req = Request::builder().method(method_to_use).uri(uri.clone());

    let mut detected_headers = HashSet::new();

    if let Some(headers) = headers {
        for (header_name, value) in headers.iter() {
            detected_headers.insert(header_name);
            if change_method && is_request_body_header_name(header_name) {
                continue;
            }
            if !same_origin && is_cors_non_wildcard_request_header_name(header_name) {
                continue;
            }
            req = req.header(header_name, value)
        }
    }

    apply_default_headers(&mut req, &detected_headers);

    // Per spec, the Origin header is included for non-CORS-safelisted methods
    // (anything other than GET/HEAD). Browsers compute origin from the caller;
    // here we approximate with the initial request URI's origin.
    if !is_get_or_head && !detected_headers.contains(ORIGIN.as_str()) {
        if let Some(origin) = uri_origin(initial_uri) {
            req = req.header(ORIGIN, origin);
        }
    }

    // `Content-Length: 0` is sent for POST/PUT/PATCH requests with no body.
    if body.is_none()
        && matches!(*method, Method::POST | Method::PUT | Method::PATCH)
        && !detected_headers.contains(CONTENT_LENGTH.as_str())
    {
        req = req.header(CONTENT_LENGTH, "0");
    }

    // Build the body
    let box_body: BoxBody<Bytes, Infallible> = match body {
        Some(RequestBody::Static { body, .. }) => BoxBody::new(body.clone()),
        Some(RequestBody::Stream(_)) => {
            unreachable!("Streaming bodies are handled by send_stream")
        },
        None => BoxBody::new(Full::default()),
    };

    let request = req.body(box_body).or_throw(ctx)?;

    Ok((request, guard))
}

struct StreamingBody {
    rx: tokio::sync::mpsc::Receiver<Bytes>,
}

impl http_body::Body for StreamingBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<std::result::Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(bytes)) => Poll::Ready(Some(Ok(http_body::Frame::data(bytes)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.rx.is_closed() && self.rx.is_empty()
    }
}

fn is_same_origin(uri: &Uri, initial_uri: &Uri) -> bool {
    is_same_scheme(uri, initial_uri)
        && is_same_host(uri, initial_uri)
        && is_same_port(uri, initial_uri)
}

fn uri_origin(uri: &Uri) -> Option<String> {
    let scheme = uri.scheme_str()?;
    let authority = uri.authority()?;
    let host = authority.host();
    let default_port = matches!(
        (scheme, authority.port_u16()),
        (_, None) | ("http", Some(80)) | ("https", Some(443))
    );
    if default_port {
        Some([scheme, "://", host].concat())
    } else {
        let mut buf = itoa::Buffer::new();
        let port_str = buf.format(authority.port_u16().unwrap());
        Some([scheme, "://", host, ":", port_str].concat())
    }
}

fn is_same_scheme(uri: &Uri, initial_uri: &Uri) -> bool {
    uri.scheme() == initial_uri.scheme()
}

fn is_same_host(uri: &Uri, initial_uri: &Uri) -> bool {
    uri.host() == initial_uri.host()
}

fn is_same_port(uri: &Uri, initial_uri: &Uri) -> bool {
    uri.authority().and_then(|a| a.port()) == initial_uri.authority().and_then(|a| a.port())
}

fn should_change_method(prev_status: u16, method: &Method) -> bool {
    if matches!(prev_status, 301 | 302) {
        return *method == Method::POST;
    }

    if prev_status == 303 {
        return !matches!(*method, Method::GET | Method::HEAD);
    }

    false
}

fn is_request_body_header_name(key: &str) -> bool {
    key == CONTENT_ENCODING
        || key == CONTENT_LANGUAGE
        || key == CONTENT_LOCATION
        || key == CONTENT_TYPE
}

fn is_cors_non_wildcard_request_header_name(key: &str) -> bool {
    key == AUTHORIZATION
}

enum RequestBody<'js> {
    Static {
        #[allow(dead_code)]
        object_bytes: ObjectBytes<'js>,
        body: Full<Bytes>,
    },
    Stream(Class<'js, ReadableStream<'js>>),
}

impl<'js> RequestBody<'js> {
    fn from_bytes(ctx: Ctx<'js>, object_bytes: ObjectBytes<'js>) -> Result<Self> {
        //this is safe since we hold on to ObjectBytes
        let raw_bytes: &'static [u8] = unsafe { std::mem::transmute(object_bytes.as_bytes(&ctx)?) };
        let body = Full::from(Bytes::from_static(raw_bytes));
        Ok(Self::Static { object_bytes, body })
    }
}

struct FetchOptions<'js> {
    method: hyper::Method,
    url: String,
    headers: Option<Headers>,
    body: Option<RequestBody<'js>>,
    abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
    redirect: String,
    agent: Option<Class<'js, Agent>>,
    integrity: Option<String>,
}

fn get_fetch_options<'js>(
    ctx: &Ctx<'js>,
    resource: Value<'js>,
    opts: Opt<Value<'js>>,
) -> Result<FetchOptions<'js>> {
    let mut url = None;
    let mut resource_opts = None;
    let mut arg_opts = None;
    let mut headers = None;
    let mut method = None;
    let mut body = None;
    let mut abort_receiver = None;
    let mut redirect = String::from("");
    let mut agent = None;

    if let Some(obj) = resource.as_object() {
        let obj = obj.clone();
        if obj.instance_of::<crate::Request>() {
            resource_opts = Some(obj);
        } else if let Some(to_string) = obj.get::<_, Option<Function>>(PredefinedAtom::ToString)? {
            url = Some(to_string.call::<_, String>((This(obj),))?);
        } else {
            resource_opts = Some(obj);
        }
    } else {
        url = Some(resource.get::<Coerced<String>>()?.to_string());
    }

    if let Some(options) = opts.0 {
        arg_opts = options.into_object();
    }

    if resource_opts.is_some() || arg_opts.is_some() {
        if let Some(priority) =
            get_option::<Value>("priority", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            if !priority.is_undefined() && !priority.is_null() {
                let p: String = priority.get()?;
                if !matches!(p.as_str(), "high" | "low" | "auto") {
                    return Err(Exception::throw_type(ctx, "Invalid priority"));
                }
            }
        }
        if let Some(method_opt) =
            get_option::<String>("method", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            method = Some(match method_opt.as_str() {
                "GET" => hyper::Method::GET,
                "POST" => hyper::Method::POST,
                "PUT" => hyper::Method::PUT,
                "CONNECT" => hyper::Method::CONNECT,
                "HEAD" => hyper::Method::HEAD,
                "PATCH" => hyper::Method::PATCH,
                "DELETE" => hyper::Method::DELETE,
                "OPTIONS" => hyper::Method::OPTIONS,
                other => hyper::Method::from_bytes(other.as_bytes()).map_err(|_| {
                    Exception::throw_type(ctx, &["Invalid HTTP method: ", other].concat())
                })?,
            });
        }

        // If the resource is a Request and init didn't override `body`,
        // take the body natively so that `request.bodyUsed` flips to true
        // synchronously per spec (otherwise `request.arrayBuffer()` etc. still
        // return the body after `fetch(request)`).
        let init_has_body = arg_opts
            .as_ref()
            .and_then(|o| o.get::<_, Value>("body").ok())
            .is_some_and(|v| !v.is_undefined() && !v.is_null());
        if !init_has_body {
            if let Some(req_obj) = resource_opts.as_ref() {
                if let Some(req) = Class::<crate::Request>::from_object(req_obj) {
                    use crate::request::BodyTaken;
                    if let Some(taken) = req.borrow().take_body_sync(ctx)? {
                        body = Some(match taken {
                            BodyTaken::Stream(stream) => RequestBody::Stream(stream),
                            BodyTaken::Bytes(bytes) => {
                                RequestBody::from_bytes(ctx.clone(), ObjectBytes::Vec(bytes))?
                            },
                        });
                    }
                }
            }
        }

        if body.is_none() {
            if let Some(body_opt) =
                get_option::<Value>("body", arg_opts.as_ref(), resource_opts.as_ref())?
            {
                if !body_opt.is_undefined() && !body_opt.is_null() {
                    if let Some(m) = &method {
                        if *m == hyper::Method::GET || *m == hyper::Method::HEAD {
                            return Err(Exception::throw_type(
                                ctx,
                                "Request with GET/HEAD method cannot have body.",
                            ));
                        }
                    }
                }
                // Check if body is a ReadableStream
                if let Ok(stream) = Class::<ReadableStream>::from_value(&body_opt) {
                    // Per WHATWG Fetch spec, streaming request bodies on HTTP/1.x
                    // require explicit `duplex: 'half'` (the client needs to know
                    // it can't wait for the whole request before reading the
                    // response). WPT `request-upload.any.js` "Streaming upload
                    // shouldn't work on Http/1.1" expects rejection when duplex
                    // is not set.
                    let duplex: Option<String> =
                        get_option::<String>("duplex", arg_opts.as_ref(), resource_opts.as_ref())?;
                    if duplex.as_deref() != Some("half") {
                        return Err(Exception::throw_type(
                            ctx,
                            "Request with stream body requires duplex: 'half'",
                        ));
                    }
                    body = Some(RequestBody::Stream(stream));
                } else {
                    let (bytes, default_ct) = if let Ok(blob) = Class::<Blob>::from_value(&body_opt)
                    {
                        let blob_ref = blob.borrow();
                        let mime = blob_ref.mime_type();
                        let ct = if mime.is_empty() { None } else { Some(mime) };
                        // Zero-copy: borrow the Blob's ArrayBuffer instead of
                        // materialising a Vec. `RequestBody::from_bytes` holds
                        // the `ObjectBytes` alive for the duration of the
                        // request, keeping the ArrayBuffer pinned.
                        let ab = blob_ref.array_buffer_ref();
                        let len = ab.len();
                        (ObjectBytes::DataView(ab, 0, len), ct)
                    } else if body_opt.is_string() {
                        (
                            ObjectBytes::from(ctx, &body_opt)?,
                            Some(MIME_TYPE_TEXT.into()),
                        )
                    } else if let Ok(fd) = Class::<FormData>::from_value(&body_opt) {
                        let (parts, boundary) = fd.borrow().to_multipart_bytes(ctx)?;
                        let ct = [MIME_TYPE_FORM_DATA, &boundary].concat();
                        (ObjectBytes::Vec(parts), Some(ct))
                    } else if let Ok(usp) = Class::<URLSearchParams>::from_value(&body_opt) {
                        (
                            ObjectBytes::Vec(usp.borrow().to_string().into_bytes()),
                            Some(MIME_TYPE_FORM_URLENCODED.into()),
                        )
                    } else {
                        (ObjectBytes::from(ctx, &body_opt)?, None)
                    };
                    body = Some(RequestBody::from_bytes(ctx.clone(), bytes)?);
                    if let Some(ct) = default_ct {
                        let hdrs = headers.get_or_insert_with(Headers::default);
                        if !hdrs.contains_lower(CONTENT_TYPE.as_str()) {
                            let v: Value = ct.into_js(ctx)?;
                            hdrs.set(ctx.clone(), CONTENT_TYPE.as_str().into(), v)?;
                        }
                    }
                }
            }
        }

        if let Some(url_opt) =
            get_option::<String>("url", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            url = Some(url_opt);
        }

        if let Some(headers_op) =
            get_option::<Value>("headers", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            headers = Some(Headers::from_value(ctx, headers_op, HeadersGuard::None)?);
        }

        if let Some(signal) =
            get_option::<Class<AbortSignal>>("signal", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            abort_receiver = Some(signal.borrow().sender.subscribe());
        }

        if let Some(redirect_opt) =
            get_option::<String>("redirect", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            let redirect_str = redirect_opt.as_str();
            if !matches!(redirect_str, "follow" | "manual" | "error") {
                return Err(Exception::throw_type(
                    ctx,
                    &["Invalid redirect option: ", redirect_str].concat(),
                ));
            }
            redirect.push_str(redirect_str);
        }

        if let Some(agent_opt) =
            get_option::<Class<'js, Agent>>("agent", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            agent = Some(agent_opt);
        }
    }

    // Per WHATWG Fetch spec, integrity is a stringified SRI (subresource
    // integrity) metadata. After the response body is received, the fetch
    // machinery verifies one of the listed hashes matches; on failure the
    // returned promise rejects with a TypeError.
    let integrity = get_option::<String>("integrity", arg_opts.as_ref(), resource_opts.as_ref())?;

    let url = match url {
        Some(url) => url,
        None => return Err(Exception::throw_reference(ctx, "Missing required url")),
    };

    Ok(FetchOptions {
        method: method.unwrap_or_default(),
        url,
        headers,
        body,
        abort_receiver,
        redirect,
        agent,
        integrity,
    })
}

fn get_option<'js, V: FromJs<'js> + Sized>(
    arg: &str,
    a: Option<&Object<'js>>,
    b: Option<&Object<'js>>,
) -> Result<Option<V>> {
    if let Some(opt) = a {
        if let Some(value) = opt.get::<_, Option<V>>(arg)? {
            return Ok(Some(value));
        }
    }

    if let Some(opt) = b {
        if let Some(value) = opt.get::<_, Option<V>>(arg)? {
            return Ok(Some(value));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        all(feature = "tls-graviola", target_arch = "x86_64")
    ))]
    use llrt_http::HttpsModule;
    use llrt_test::test_async_with;
    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        all(feature = "tls-graviola", target_arch = "x86_64")
    ))]
    use llrt_test::{call_test, ModuleEvaluator};
    use rquickjs::{prelude::Promise, CatchResultExt};
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    use super::*;

    #[test]
    fn test_should_change_method() {
        // Test cases for prev_status being 301 or 302
        assert!(should_change_method(301, &Method::POST));
        assert!(should_change_method(302, &Method::POST));

        assert!(!should_change_method(301, &Method::GET));
        assert!(!should_change_method(302, &Method::GET));
        assert!(!should_change_method(301, &Method::HEAD));
        assert!(!should_change_method(302, &Method::HEAD));

        // Test cases for prev_status being 303
        assert!(should_change_method(303, &Method::POST));

        assert!(!should_change_method(303, &Method::GET));
        assert!(!should_change_method(303, &Method::HEAD));

        // Test case for other prev_status values
        assert!(!should_change_method(200, &Method::POST));
        assert!(!should_change_method(404, &Method::GET));
    }

    #[test]
    fn test_is_request_body_header_name() {
        assert!(is_request_body_header_name("content-encoding"));
        assert!(is_request_body_header_name("content-language"));
        assert!(is_request_body_header_name("content-location"));
        assert!(is_request_body_header_name("content-type"));

        assert!(!is_request_body_header_name("content-length"));
        assert!(!is_request_body_header_name("accept"));
    }

    #[test]
    fn test_is_same_origin() {
        let uri1 = Uri::from_static("https://example.com:8080/path");
        let uri2 = Uri::from_static("https://example.com:8080/path");

        assert!(is_same_origin(&uri1, &uri2));

        let uri3 = Uri::from_static("http://example.com/path");
        let uri4 = Uri::from_static("https://example.com/path");

        assert!(!is_same_origin(&uri3, &uri4));

        let uri5 = Uri::from_static("https://example.com:8080/path");
        let uri6 = Uri::from_static("https://example.org:8080/path");

        assert!(!is_same_origin(&uri5, &uri6));

        let uri7 = Uri::from_static("https://example.com:8080/path");
        let uri8 = Uri::from_static("https://example.com:8081/path");

        assert!(!is_same_origin(&uri7, &uri8));
    }

    #[test]
    fn test_is_same_scheme() {
        let uri1 = Uri::from_static("https://example.com");
        let uri2 = Uri::from_static("https://example.com");

        assert!(is_same_scheme(&uri1, &uri2));

        let uri3 = Uri::from_static("http://example.com");
        let uri4 = Uri::from_static("https://example.com");

        assert!(!is_same_scheme(&uri3, &uri4));
    }

    #[test]
    fn test_is_same_host() {
        let uri1 = Uri::from_static("https://example.com");
        let uri2 = Uri::from_static("https://example.com");

        assert!(is_same_host(&uri1, &uri2));

        let uri3 = Uri::from_static("https://example.com");
        let uri4 = Uri::from_static("https://example.org");

        assert!(!is_same_host(&uri3, &uri4));
    }

    #[test]
    fn test_is_same_port() {
        let uri1 = Uri::from_static("https://example.com:8080");
        let uri2 = Uri::from_static("https://example.com:8080");

        assert!(is_same_port(&uri1, &uri2));

        let uri3 = Uri::from_static("https://example.com:8080");
        let uri4 = Uri::from_static("https://example.com:9090");

        assert!(!is_same_port(&uri3, &uri4));

        let uri5 = Uri::from_static("https://example.com");
        let uri6 = Uri::from_static("https://example.com");

        assert!(is_same_port(&uri5, &uri6));

        let uri7 = Uri::from_static("https://example.com:8080");
        let uri8 = Uri::from_static("https://example.com");

        assert!(!is_same_port(&uri7, &uri8));
    }

    #[tokio::test]
    async fn test_fetch_function() {
        let mock_server = MockServer::start().await;
        let welcome_message = "Hello, LLRT!";

        Mock::given(matchers::path("expect/200/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(String::from_utf8(welcome_message.into()).unwrap()),
            )
            .mount(&mock_server)
            .await;

        Mock::given(matchers::path("expect/301/"))
            .respond_with(ResponseTemplate::new(301).insert_header(
                "location",
                format!("http://{}/{}", mock_server.address(), "expect/200/"),
            ))
            .mount(&mock_server)
            .await;

        Mock::given(matchers::path("expect/302/"))
            .respond_with(ResponseTemplate::new(302).insert_header(
                "location",
                format!("http://{}/{}", mock_server.address(), "expect/200/"),
            ))
            .mount(&mock_server)
            .await;

        Mock::given(matchers::path("expect/303/"))
            .respond_with(ResponseTemplate::new(303).insert_header(
                "location",
                format!("http://{}/{}", mock_server.address(), "expect/200/"),
            ))
            .mount(&mock_server)
            .await;

        Mock::given(matchers::path("expect/304/"))
            .respond_with(ResponseTemplate::new(304).insert_header(
                "location",
                format!("http://{}/{}", mock_server.address(), "expect/200/"),
            ))
            .mount(&mock_server)
            .await;

        Mock::given(matchers::path("expect/location/"))
            .respond_with(ResponseTemplate::new(200).insert_header(
                "location",
                format!("http://{}/{}", mock_server.address(), "expect/200/"),
            ))
            .mount(&mock_server)
            .await;

        let mut data: Vec<u8> = Vec::new();
        llrt_compression::zstd::encoder(welcome_message.as_bytes(), 3)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        Mock::given(matchers::path("content-encoding/zstd/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-encoding", "zstd")
                    .set_body_raw(data.to_owned(), "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let mut data: Vec<u8> = Vec::new();
        llrt_compression::brotli::encoder(welcome_message.as_bytes())
            .read_to_end(&mut data)
            .unwrap();
        Mock::given(matchers::path("content-encoding/br/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-encoding", "br")
                    .set_body_raw(data.to_owned(), "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let mut data: Vec<u8> = Vec::new();
        llrt_compression::gz::encoder(
            welcome_message.as_bytes(),
            llrt_compression::gz::Compression::default(),
        )
        .read_to_end(&mut data)
        .unwrap();
        Mock::given(matchers::path("content-encoding/gzip/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-encoding", "gzip")
                    .set_body_raw(data.to_owned(), "text/plain"),
            )
            .mount(&mock_server)
            .await;

        let mut data: Vec<u8> = Vec::new();
        llrt_compression::zlib::encoder(
            welcome_message.as_bytes(),
            llrt_compression::zlib::Compression::default(),
        )
        .read_to_end(&mut data)
        .unwrap();
        Mock::given(matchers::path("content-encoding/deflate/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .append_header("content-encoding", "deflate")
                    .set_body_raw(data.to_owned(), "text/plain"),
            )
            .mount(&mock_server)
            .await;

        // NOTE: A minimum redirect test pattern was created. Please add more as needed.
        test_async_with(|ctx| {
            crate::init(&ctx).unwrap();
            Box::pin(async move {
                let globals = ctx.globals();
                let run = async {
                    let fetch: Function = globals.get("fetch")?;

                    let headers = Object::new(ctx.clone())?;
                    headers.set("content-encoding", "gzip")?;
                    headers.set("content-language", "en")?;
                    headers.set("content-location", "/documents/foo.txt")?;
                    headers.set("content-type", "text/plain")?;
                    headers.set("authorization", "Basic YWxhZGRpbjpvcGVuc2VzYW1l")?;

                    let options = Object::new(ctx.clone())?;
                    options.set("redirect", "follow")?;
                    options.set("headers", headers.clone())?;

                    // Method: GET, Redirect Pattern: None
                    options.set("method", "GET")?;
                    let url = format!("http://{}/expect/200/", mock_server.address().clone());

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response_text: String =
                        Response::text(This(response.clone()), ctx.clone())?
                            .into_future()
                            .await?;
                    let response = response.borrow();
                    assert_eq!(response.status(), 200);
                    assert_eq!(
                        response.url(),
                        format!("http://{}/expect/200/", mock_server.address().clone())
                    );
                    assert!(!response.redirected());
                    assert_eq!(response_text, welcome_message);

                    // Method: GET, Redirect Pattern: 301 -> 200
                    options.set("method", "GET")?;
                    let url = format!("http://{}/expect/301/", mock_server.address().clone());

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response = response.borrow();

                    assert_eq!(response.status(), 200);
                    assert_eq!(
                        response.url(),
                        format!("http://{}/expect/200/", mock_server.address().clone())
                    );
                    assert!(response.redirected());

                    // Method: GET, Redirect Pattern: 302 -> 200
                    options.set("method", "GET")?;
                    let url = format!("http://{}/expect/302/", mock_server.address().clone());

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response = response.borrow();

                    assert_eq!(response.status(), 200);
                    assert_eq!(
                        response.url(),
                        format!("http://{}/expect/200/", mock_server.address().clone())
                    );
                    assert!(response.redirected());

                    // Method: GET, Redirect Pattern: 303 -> 200
                    options.set("method", "GET")?;
                    let url = format!("http://{}/expect/303/", mock_server.address().clone());

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response = response.borrow();

                    assert_eq!(response.status(), 200);
                    assert_eq!(
                        response.url(),
                        format!("http://{}/expect/200/", mock_server.address().clone())
                    );
                    assert!(response.redirected());

                    // Method: GET, Non-redirect status with location header (304) - should NOT follow
                    options.set("method", "GET")?;
                    let url = format!("http://{}/expect/location/", mock_server.address().clone());

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response = response.borrow();

                    assert_eq!(response.status(), 200);
                    assert_eq!(
                        response.url(),
                        format!("http://{}/expect/location/", mock_server.address().clone())
                    );
                    assert!(!response.redirected());

                    // Content-Encoding: zstd
                    let url = format!(
                        "http://{}/content-encoding/zstd/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response_text: String = Response::text(This(response), ctx.clone())?
                        .into_future()
                        .await?;

                    assert_eq!(response_text, welcome_message);

                    // Content-Encoding: br
                    let url = format!(
                        "http://{}/content-encoding/br/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response_text: String = Response::text(This(response), ctx.clone())?
                        .into_future()
                        .await?;

                    assert_eq!(response_text, welcome_message);

                    // Content-Encoding: gzip
                    let url = format!(
                        "http://{}/content-encoding/gzip/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response_text: String = Response::text(This(response), ctx.clone())?
                        .into_future()
                        .await?;

                    assert_eq!(response_text, welcome_message);

                    // Content-Encoding: deflate
                    let url = format!(
                        "http://{}/content-encoding/deflate/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let response_text: String = Response::text(This(response), ctx.clone())?
                        .into_future()
                        .await?;

                    assert_eq!(response_text, welcome_message);

                    Ok(())
                };
                run.await.catch(&ctx).unwrap();
            })
        })
        .await;
    }

    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        all(feature = "tls-graviola", target_arch = "x86_64")
    ))]
    #[tokio::test]
    async fn test_fetch_tls() {
        let mock_server = llrt_test_tls::MockServer::start().await.unwrap();
        let addr = mock_server.address().to_string();
        let ca = mock_server.ca().to_string();

        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<HttpsModule>(ctx.clone(), "https")
                    .await
                    .unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { Agent } from 'https';

                        export async function test(addr, ca) {
                            const response = await fetch(`https://${addr}/echo`, {
                                method: "POST",
                                body: "Hello, LLRT!",
                                agent: new Agent({
                                    ca: ca
                                }),
                            });
                            const text = await response.text();
                            return text;
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<String, _>(&ctx, &module, (addr, ca)).await;

                assert_eq!(result, "Hello, LLRT!");
            })
        })
        .await;
    }

    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        all(feature = "tls-graviola", target_arch = "x86_64")
    ))]
    #[tokio::test]
    async fn test_fetch_ignore_certificate_errors() {
        let mock_server = llrt_test_tls::MockServer::start().await.unwrap();
        let addr = mock_server.address().to_string();

        test_async_with(|ctx| {
            Box::pin(async move {
                crate::init(&ctx).unwrap();
                ModuleEvaluator::eval_rust::<HttpsModule>(ctx.clone(), "https")
                    .await
                    .unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { Agent } from 'https';

                        export async function test(addr) {
                            const response = await fetch(`https://${addr}/echo`, {
                                method: "POST",
                                body: "Hello, LLRT!",
                                agent: new Agent({
                                    rejectUnauthorized: false,
                                }),
                            });
                            const text = await response.text();
                            return text;
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<String, _>(&ctx, &module, (addr,)).await;

                assert_eq!(result, "Hello, LLRT!");
            })
        })
        .await;
    }
}
