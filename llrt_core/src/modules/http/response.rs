// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{collections::HashMap, io::Read, time::Instant};

use http_body_util::BodyExt;
use hyper::{body::Incoming, header::HeaderName};
use rquickjs::{
    class::{Trace, Tracer},
    function::Opt,
    ArrayBuffer, Class, Ctx, Exception, Null, Object, Result, Value,
};
use tokio::{runtime::Handle, select};

use crate::{
    json::parse::json_parse,
    modules::events::AbortSignal,
    utils::{class::get_class, mc_oneshot, object::get_bytes, result::ResultExt},
};

use super::{blob::Blob, headers::Headers};

use once_cell::sync::Lazy;

use brotlic::DecompressorReader as BrotliDecoder;
use flate2::read::{GzDecoder, ZlibDecoder};
use zstd::stream::read::Decoder as ZstdDecoder;

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
    Incoming(Option<hyper::Response<Incoming>>),
    Provided(Value<'js>),
}

impl<'js> Response<'js> {
    pub fn from_incoming(
        ctx: Ctx<'js>,
        response: hyper::Response<Incoming>,
        method: String,
        url: String,
        start: Instant,
        redirected: bool,
        abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
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

        let mut content_encoding = None;
        if let Some(content_encoding_header) =
            response_headers.get(HeaderName::from_static("content-encoding"))
        {
            if let Ok(content_encoding_header) = content_encoding_header.to_str() {
                content_encoding = Some(content_encoding_header.to_owned())
            }
        }

        let headers = Headers::from_http_headers(response.headers())?;
        let headers = Class::instance(ctx.clone(), headers)?;

        let status = response.status();

        let body_attributes = BodyAttributes {
            content_type,
            content_encoding,
        };

        Ok(Self {
            body: Some(BodyVariant::Incoming(Some(response))),
            method,
            url,
            start,
            status: status.as_u16(),
            status_text: None,
            redirected,
            headers,
            body_attributes,
            abort_receiver,
        })
    }

    async fn take_bytes(&mut self, ctx: &Ctx<'js>) -> Result<Option<Vec<u8>>> {
        let bytes = match &mut self.body {
            Some(BodyVariant::Incoming(incoming)) => {
                let mut body = incoming
                    .take()
                    .ok_or(Exception::throw_type(ctx, "Already read"))?;
                let bytes = if let Some(abort_signal) = &self.abort_receiver {
                    select! {
                        err = abort_signal.recv() => return Err(ctx.throw(err)),
                        collected_body = body.body_mut().collect() => collected_body.or_throw(ctx)?.to_bytes()
                    }
                } else {
                    body.body_mut().collect().await.or_throw(ctx)?.to_bytes()
                };

                if let Some(content_encoding) = &self.body_attributes.content_encoding {
                    let mut data: Vec<u8> = Vec::with_capacity(bytes.len());
                    match content_encoding.as_str() {
                        "zstd" => ZstdDecoder::new(&bytes[..])?.read_to_end(&mut data)?,
                        "br" => BrotliDecoder::new(&bytes[..]).read_to_end(&mut data)?,
                        "gzip" => GzDecoder::new(&bytes[..]).read_to_end(&mut data)?,
                        "deflate" => ZlibDecoder::new(&bytes[..]).read_to_end(&mut data)?,
                        _ => return Err(Exception::throw_message(ctx, "Unsupported encoding")),
                    };
                    data
                } else {
                    bytes.to_vec()
                }
            },
            Some(BodyVariant::Provided(provided)) => {
                if let Some(blob) = get_class::<Blob>(provided)? {
                    let blob = blob.borrow();
                    blob.get_bytes()
                } else {
                    get_bytes(ctx, provided.clone())?
                }
            },
            None => return Ok(None),
        };

        Ok(Some(bytes))
    }
}

#[rquickjs::class]
pub struct Response<'js> {
    body: Option<BodyVariant<'js>>,
    method: String,
    url: String,
    start: Instant,
    status: u16,
    status_text: Option<String>,
    redirected: bool,
    headers: Class<'js, Headers>,
    body_attributes: BodyAttributes,
    abort_receiver: Option<mc_oneshot::Receiver<Value<'js>>>,
}

#[derive(Clone, Debug)]
struct BodyAttributes {
    content_type: Option<String>,
    content_encoding: Option<String>,
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Response<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, body: Opt<Value<'js>>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut status = 200;
        let mut headers = None;
        let mut status_text = None;
        let mut abort_receiver = None;

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

            if let Some(signal) = opt.get::<_, Option<Class<AbortSignal>>>("signal")? {
                abort_receiver = Some(signal.borrow().sender.subscribe())
            }
        }

        let headers = if let Some(headers) = headers {
            Class::instance(ctx.clone(), headers)
        } else {
            Class::instance(ctx.clone(), Headers::default())
        }?;

        let body = body.0.and_then(|body| {
            if body.is_null() || body.is_undefined() {
                None
            } else {
                Some(BodyVariant::Provided(body))
            }
        });

        let body_attributes = BodyAttributes {
            content_type: headers.get("content-type")?,
            content_encoding: headers.get("content-encoding")?,
        };

        Ok(Self {
            body,
            method: "GET".into(),
            url: "".into(),
            start: Instant::now(),
            status,
            status_text,
            redirected: false,
            headers,
            body_attributes,
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

    //FIXME return readable stream when implemented
    #[qjs(get)]
    pub fn body(&self) -> Null {
        Null
    }

    #[qjs(get)]
    fn headers(&self) -> Class<'js, Headers> {
        self.headers.clone()
    }

    pub async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(String::from_utf8_lossy(&bytes).to_string());
        }
        Ok("".into())
    }

    pub async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return json_parse(&ctx, bytes);
        }
        Err(Exception::throw_syntax(&ctx, "JSON input is empty"))
    }

    async fn array_buffer(&mut self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return ArrayBuffer::new(ctx, bytes);
        }
        ArrayBuffer::new(ctx, Vec::<u8>::new())
    }

    async fn blob(&mut self, ctx: Ctx<'js>) -> Result<Blob> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(Blob::from_bytes(
                bytes,
                self.body_attributes.content_type.clone(),
            ));
        }
        Ok(Blob::from_bytes(Vec::<u8>::new(), None))
    }

    fn clone(&mut self, ctx: Ctx<'js>) -> Result<Self> {
        let body = if self.body.is_some() {
            let array_buffer_future = self.array_buffer(ctx.clone());
            let array_buffer = tokio::task::block_in_place(move || {
                Handle::current().block_on(array_buffer_future)
            })?;
            Some(BodyVariant::Provided(array_buffer.into_value()))
        } else {
            None
        };

        Ok(Self {
            body,
            method: self.method.clone(),
            url: self.url.clone(),
            start: self.start,
            status: self.status,
            status_text: self.status_text.clone(),
            redirected: self.redirected,
            headers: Class::<Headers>::instance(ctx, self.headers.borrow().clone())?,
            body_attributes: self.body_attributes.clone(),
            abort_receiver: self.abort_receiver.clone(),
        })
    }

    #[qjs(get, rename = "type")]
    fn response_type(&self) -> &'js str {
        "basic"
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
        if let Some(BodyVariant::Incoming(body)) = &self.body {
            return body.is_none();
        }
        false
    }
}

impl<'js> Trace<'js> for Response<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.headers.trace(tracer);
        if let Some(BodyVariant::Provided(body)) = &self.body {
            body.trace(tracer);
        }
    }
}
