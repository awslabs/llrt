// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::time::Instant;

use hyper::{
    body::{Bytes, Incoming},
    header::HeaderName,
};
use rquickjs::{
    class::{Trace, Tracer},
    function::Opt,
    ArrayBuffer, Class, Ctx, IntoJs, Object, Result, TypedArray, Value,
};

use super::{blob::Blob, body::Body, headers::Headers};

use once_cell::sync::Lazy;
use std::collections::HashMap;

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

pub struct ResponseData<'js> {
    body: Option<Class<'js, Body<'js>>>,
    method: String,
    url: String,
    start: Instant,
    status: u16,
    status_text: Option<String>,
    headers: Class<'js, Headers>,
}

impl<'js> ResponseData<'js> {
    pub fn from_incoming(
        ctx: Ctx<'js>,
        response: hyper::Response<Incoming>,
        method: String,
        url: String,
        start: Instant,
    ) -> Result<Self> {
        let response_headers = response.headers();
        let mut content_type = None;
        if let Some(content_type_header) =
            response_headers.get(HeaderName::from_static("content-type"))
        {
            if let Ok(content_type_header) = content_type_header.to_str() {
                content_type = Some(content_type_header.to_owned())
            }
        }

        let headers = Headers::from_http_headers(&ctx, response.headers())?;
        let headers = Class::instance(ctx.clone(), headers)?;

        let status = response.status();

        Ok(Self {
            body: Some(Body::from_incoming(ctx, response, content_type)?),
            method,
            url,
            status_text: None,
            start,
            status: status.as_u16(),
            headers,
        })
    }
}

struct Uint8ArrayJsValue(Bytes);

impl Uint8ArrayJsValue {
    fn into_js_obj<'js>(self, ctx: &Ctx<'js>) -> Result<Value<'js>>
    where
        Self: Sized,
    {
        let array_buffer = TypedArray::new(ctx.clone(), self.0.to_vec())?;
        array_buffer.into_js(ctx)
    }
}

impl<'js> IntoJs<'js> for Uint8ArrayJsValue {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        self.into_js_obj(ctx)
    }
}

#[rquickjs::class]
pub struct Response<'js> {
    pub data: ResponseData<'js>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Response<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, body: Opt<Value<'js>>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut status = 200;
        let mut headers = None;
        let mut status_text = None;

        if let Some(opt) = options.0 {
            if let Some(status_opt) = opt.get("status")? {
                status = status_opt;
            }
            if let Some(headers_opt) = opt.get("headers")? {
                headers = Some(Headers::from_value(&ctx, headers_opt)?);
            }
            if let Some(status_text_opt) = opt.get("statusText")? {
                status_text = Some(status_text_opt);
            }
        }

        let headers = if let Some(headers) = headers {
            Class::instance(ctx.clone(), headers)
        } else {
            Class::instance(ctx.clone(), Headers::default())
        }?;

        Ok(Self {
            data: ResponseData {
                body: Body::from_value(&ctx, body.0)?,
                method: "GET".into(),
                url: "".into(),
                start: Instant::now(),
                status,
                headers,
                status_text,
            },
        })
    }

    #[qjs(get)]
    pub fn status(&self) -> u64 {
        self.data.status.into()
    }

    #[qjs(get)]
    pub fn url(&self) -> String {
        self.data.url.clone()
    }

    #[qjs(get)]
    pub fn body(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Body::get_body(ctx, &self.data.body)
    }

    #[qjs(get)]
    pub fn ok(&self) -> bool {
        self.data.status > 199 && self.data.status < 300
    }

    #[qjs(get)]
    fn headers(&self) -> Class<'js, Headers> {
        self.data.headers.clone()
    }

    async fn text(&self, ctx: Ctx<'js>) -> Result<String> {
        Body::get_text(ctx, &self.data.body).await
    }

    async fn json(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        Body::get_json(ctx, &self.data.body).await
    }

    async fn array_buffer(&self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        Body::get_array_buffer(ctx, &self.data.body).await
    }

    async fn blob(&self, ctx: Ctx<'js>) -> Result<Blob> {
        Body::get_blob(ctx, &self.data.body).await
    }

    fn clone(&self, ctx: Ctx<'js>) -> Result<Self> {
        Ok(Self {
            data: ResponseData {
                body: self.data.body.clone(),
                method: self.data.method.clone(),
                url: self.data.url.clone(),
                start: self.data.start,
                status: self.data.status,
                status_text: self.data.status_text.clone(),
                headers: Class::<Headers>::instance(ctx, self.data.headers.borrow().clone())?,
            },
        })
    }

    #[qjs(get, rename = "type")]
    fn response_type(&self) -> &'js str {
        "basic"
    }

    #[qjs(get)]
    fn status_text(&self) -> String {
        if let Some(text) = &self.data.status_text {
            return text.to_string();
        }
        STATUS_TEXTS
            .get(&self.data.status)
            .unwrap_or(&"")
            .to_string()
    }

    #[qjs(get)]
    fn body_used(&self) -> bool {
        if let Some(body) = &self.data.body {
            return body.borrow().is_used();
        }
        false
    }
}

impl<'js> Trace<'js> for Response<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.data.headers.trace(tracer);
        if let Some(body) = &self.data.body {
            body.trace(tracer);
        }
    }
}
