// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::convert::Infallible;
use std::{collections::HashSet, time::Instant};

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::Full;
use hyper::{header::HeaderName, Method, Request, Uri};
use hyper_util::client::legacy::connect::Connect;
use hyper_util::client::legacy::Client;
use llrt_abort::AbortSignal;
use llrt_encoding::bytes_from_b64;
use llrt_utils::{bytes::ObjectBytes, mc_oneshot, result::ResultExt, VERSION};
use rquickjs::{
    atom::PredefinedAtom,
    function::{Opt, This},
    prelude::{Async, Func},
    Class, Coerced, Ctx, Exception, FromJs, Function, IntoJs, Object, Result, Value,
};
use tokio::select;

use super::{blob::Blob, headers::Headers, response::Response, security::ensure_url_access};

const MAX_REDIRECT_COUNT: u32 = 20;

pub(crate) fn init<C>(client: Client<C, BoxBody<Bytes, Infallible>>, globals: &Object) -> Result<()>
where
    C: Clone + Send + Sync + Connect + 'static,
{
    globals.set(
        "fetch",
        Func::from(Async(move |ctx, resource, args| {
            let client = client.clone();
            let start = Instant::now();
            let options = get_fetch_options(&ctx, resource, args);

            async move {
                let options = options?;

                if let Some(data_url) = options.url.strip_prefix("data:") {
                    return parse_data_url(&ctx, data_url);
                }

                let initial_uri: Uri = options.url.parse().or_throw(&ctx)?;
                let mut uri: Uri = initial_uri.clone();
                let method_string = options.method.to_string();
                let method = options.method;
                let abort_receiver = options.abort_receiver;

                ensure_url_access(&ctx, &uri)?;

                let mut redirect_count = 0;
                let mut response_status = 0;
                let res = loop {
                    let req = build_request(
                        &ctx,
                        &method,
                        &uri,
                        options.headers.as_ref(),
                        options.body.as_ref(),
                        &response_status,
                        &initial_uri,
                    )?;

                    let res = if let Some(abort_receiver) = &abort_receiver {
                        select! {
                            res = client.request(req) => res.or_throw(&ctx)?,
                            reason = abort_receiver.recv() => return Err(ctx.throw(reason))
                        }
                    } else {
                        client.request(req).await.or_throw(&ctx)?
                    };

                    match res.headers().get(HeaderName::from_static("location")) {
                        Some(location_headers) => {
                            if let Ok(location_str) = location_headers.to_str() {
                                uri = location_str.parse().or_throw(&ctx)?;
                                ensure_url_access(&ctx, &uri)?;
                            }
                        },
                        None => break res,
                    };

                    if options.redirect == "manual" {
                        break res;
                    } else if options.redirect == "error" {
                        return Err(Exception::throw_message(&ctx, "Unexpected redirect"));
                    }

                    redirect_count += 1;
                    if redirect_count >= MAX_REDIRECT_COUNT {
                        return Err(Exception::throw_message(&ctx, "Max retries exceeded"));
                    }

                    response_status = res.status().as_u16();
                };

                Response::from_incoming(
                    ctx,
                    res,
                    method_string,
                    uri.to_string(),
                    start,
                    !matches!(redirect_count, 0),
                    abort_receiver,
                )
            }
        })),
    )?;
    Ok(())
}

fn parse_data_url<'js>(ctx: &Ctx<'js>, data_url: &str) -> Result<Response<'js>> {
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

    let body = if is_base64 {
        bytes_from_b64(data.as_bytes()).or_throw(ctx)?
    } else {
        data.as_bytes().into()
    };

    let blob = Blob::from_bytes(body, Some(content_type.clone())).into_js(ctx)?;

    let headers = Object::new(ctx.clone())?;
    headers.set("content-type", content_type)?;

    let options = Object::new(ctx.clone())?;
    options.set("url", data_url)?;
    options.set("headers", headers)?;

    Response::new(ctx.clone(), Opt(Some(blob)), Opt(Some(options)))
}

fn build_request(
    ctx: &Ctx<'_>,
    method: &hyper::Method,
    uri: &Uri,
    headers: Option<&Headers>,
    body: Option<&BodyBytes>,
    prev_status: &u16,
    initial_uri: &Uri,
) -> Result<Request<BoxBody<Bytes, Infallible>>> {
    let same_origin = is_same_origin(uri, initial_uri);

    let change_method = should_change_method(*prev_status, method);

    let (method_to_use, body) = if change_method {
        (Method::GET, None)
    } else {
        (method.clone(), body)
    };

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

    if !detected_headers.contains("user-agent") {
        req = req.header("user-agent", ["llrt ", VERSION].concat());
    }
    if !detected_headers.contains("accept-encoding") {
        req = req.header("accept-encoding", "zstd, br, gzip, deflate");
    }
    if !detected_headers.contains("accept") {
        req = req.header("accept", "*/*");
    }

    req.body(BoxBody::new(
        body.map(|b| b.body.clone()).unwrap_or_default(),
    ))
    .or_throw(ctx)
}

fn is_same_origin(uri: &Uri, initial_uri: &Uri) -> bool {
    is_same_scheme(uri, initial_uri)
        && is_same_host(uri, initial_uri)
        && is_same_port(uri, initial_uri)
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
    matches!(
        key,
        "content-encoding" | "content-language" | "content-location" | "content-type"
    )
}

fn is_cors_non_wildcard_request_header_name(key: &str) -> bool {
    matches!(key, "authorization")
}

struct BodyBytes<'js> {
    #[allow(dead_code)]
    object_bytes: ObjectBytes<'js>,
    body: Full<Bytes>,
}
impl<'js> BodyBytes<'js> {
    fn new(object_bytes: ObjectBytes<'js>) -> Self {
        //this is safe since we hold on to ObjectBytes
        let raw_bytes: &'static [u8] = unsafe { std::mem::transmute(object_bytes.as_bytes()) };
        let body = Full::from(Bytes::from_static(raw_bytes));
        Self { object_bytes, body }
    }
}

struct FetchOptions<'js> {
    method: hyper::Method,
    url: String,
    headers: Option<Headers>,
    body: Option<BodyBytes<'js>>,
    abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
    redirect: String,
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
        if let Some(method_opt) =
            get_option::<String>("method", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            method = Some(match method_opt.as_str() {
                "GET" => Ok(hyper::Method::GET),
                "POST" => Ok(hyper::Method::POST),
                "PUT" => Ok(hyper::Method::PUT),
                "CONNECT" => Ok(hyper::Method::CONNECT),
                "HEAD" => Ok(hyper::Method::HEAD),
                "PATCH" => Ok(hyper::Method::PATCH),
                "DELETE" => Ok(hyper::Method::DELETE),
                _ => Err(Exception::throw_type(
                    ctx,
                    &["Invalid HTTP method: ", &method_opt].concat(),
                )),
            }?);
        }

        if let Some(body_opt) =
            get_option::<Value>("body", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            let bytes = ObjectBytes::from(ctx, &body_opt)?;
            body = Some(BodyBytes::new(bytes));
        }

        if let Some(url_opt) =
            get_option::<String>("url", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            url = Some(url_opt);
        }

        if let Some(headers_op) =
            get_option::<Value>("headers", arg_opts.as_ref(), resource_opts.as_ref())?
        {
            headers = Some(Headers::from_value(ctx, headers_op)?);
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
    }

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

    use llrt_test::test_async_with;
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
                    let mut response = response.borrow_mut();
                    let response_text = response.text(ctx.clone()).await?;

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

                    // Content-Encoding: zstd
                    let url = format!(
                        "http://{}/content-encoding/zstd/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let mut response = response.borrow_mut();
                    let response_text = response.text(ctx.clone()).await?;

                    assert_eq!(response_text, welcome_message);

                    // Content-Encoding: br
                    let url = format!(
                        "http://{}/content-encoding/br/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let mut response = response.borrow_mut();
                    let response_text = response.text(ctx.clone()).await?;

                    assert_eq!(response_text, welcome_message);

                    // Content-Encoding: gzip
                    let url = format!(
                        "http://{}/content-encoding/gzip/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let mut response = response.borrow_mut();
                    let response_text = response.text(ctx.clone()).await?;

                    assert_eq!(response_text, welcome_message);

                    // Content-Encoding: deflate
                    let url = format!(
                        "http://{}/content-encoding/deflate/",
                        mock_server.address().clone()
                    );

                    let response_promise: Promise = fetch.call((url, options.clone()))?;
                    let response: Class<Response> = response_promise.into_future().await?;
                    let mut response = response.borrow_mut();
                    let response_text = response.text(ctx.clone()).await?;

                    assert_eq!(response_text, welcome_message);

                    Ok(())
                };
                run.await.catch(&ctx).unwrap();
            })
        })
        .await;
    }
}
