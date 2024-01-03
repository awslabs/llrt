use std::time::Instant;

use hyper::{body::Bytes, Body};
use rquickjs::{
    class::{Trace, Tracer},
    Class, Ctx, Exception, IntoJs, Result, TypedArray, Value,
};
use tracing::trace;

use crate::{json::parse::json_parse, util::ResultExt};

use super::headers::Headers;

pub struct ResponseData<'js> {
    response: Option<hyper::Response<Body>>,
    method: String,
    url: String,
    start: Instant,
    status: hyper::StatusCode,
    headers: Class<'js, Headers>,
}

impl<'js> ResponseData<'js> {
    pub fn new(
        ctx: Ctx<'js>,
        response: hyper::Response<Body>,
        method: String,
        url: String,
        start: Instant,
    ) -> Result<Self> {
        let headers = Headers::from_http_headers(&ctx, response.headers())?;
        let headers = Class::instance(ctx, headers)?;

        let status = response.status();

        Ok(Self {
            response: Some(response),
            method,
            url,
            start,
            status,
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
    #[qjs(get)]
    pub fn status(&self) -> u64 {
        self.data.status.as_u16().into()
    }

    #[qjs(get)]
    fn headers(&self) -> Class<'js, Headers> {
        self.data.headers.clone()
    }

    async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        let bytes = self.take_bytes(&ctx).await?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        Ok(text)
    }

    async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let bytes = self.take_bytes(&ctx).await?.to_vec();
        let json = json_parse(&ctx, bytes)?;
        Ok(json)
    }

    async fn array_buffer(&mut self, ctx: Ctx<'js>) -> Result<Vec<u8>> {
        let bytes = self.take_bytes(&ctx).await?;
        Ok(bytes.to_vec())
    }

    async fn blob(&mut self, ctx: Ctx<'js>) -> Result<TypedArray<'js, u8>> {
        let bytes = self.take_bytes(&ctx).await?;
        TypedArray::new(ctx, bytes.to_vec())
    }

    #[qjs(get, rename = "type")]
    fn reponse_type(&self) -> &'js str {
        "basic"
    }

    #[qjs(get)]
    fn status_text(&self) -> &'js str {
        ""
    }

    #[qjs(get)]
    fn body_used(&self) -> bool {
        self.data.response.is_none()
    }

    #[qjs(skip)]
    async fn take_bytes(&mut self, ctx: &Ctx<'js>) -> Result<Bytes> {
        let mut body = self
            .data
            .response
            .take()
            .ok_or(Exception::throw_type(ctx, "Already read"))?;

        let bytes = hyper::body::to_bytes(body.body_mut()).await.or_throw(ctx)?;
        trace!(
            "{} {}: {}ms",
            self.data.method,
            self.data.url,
            self.data.start.elapsed().as_millis()
        );
        Ok(bytes)
    }
}

impl<'js> Trace<'js> for Response<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.data.headers.trace(tracer);
    }
}
