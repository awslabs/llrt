// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    class::Trace, function::Opt, ArrayBuffer, Class, Ctx, Exception, FromJs, IntoJs, Null, Object,
    Result, TypedArray, Value,
};

use crate::{
    json::parse::json_parse,
    modules::events::AbortSignal,
    utils::{class::get_class, object::get_bytes, object::ObjectExt},
};

use super::{blob::Blob, headers::Headers};

impl<'js> Request<'js> {
    async fn take_bytes(&mut self, ctx: &Ctx<'js>) -> Result<Option<Vec<u8>>> {
        let bytes = match &mut self.body {
            Some(provided) => {
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
pub struct Request<'js> {
    url: String,
    method: String,
    headers: Option<Class<'js, Headers>>,
    body: Option<Value<'js>>,
    content_type: Option<String>,
    signal: Option<Class<'js, AbortSignal<'js>>>,
}

impl<'js> Trace<'js> for Request<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        if let Some(headers) = &self.headers {
            headers.trace(tracer);
        }
        if let Some(body) = &self.body {
            body.trace(tracer);
        }
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Request<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut request = Self {
            url: String::from(""),
            method: "GET".to_string(),
            headers: None,
            body: None,
            content_type: None,
            signal: None,
        };

        if input.is_string() {
            request.url = input.get()?;
        } else if input.is_object() {
            assign_request(&mut request, ctx.clone(), input.as_object().unwrap())?;
        }
        if let Some(options) = options.0 {
            assign_request(&mut request, ctx.clone(), &options)?;
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

    //TODO should implement readable stream
    #[qjs(get)]
    fn body(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(body) = &self.body {
            return Ok(body.clone());
        }
        Null.into_js(&ctx)
    }

    #[qjs(get)]
    fn keepalive(&self) -> bool {
        true
    }

    #[qjs(get)]
    fn signal(&self) -> Option<Class<'js, AbortSignal<'js>>> {
        self.signal.clone()
    }

    #[qjs(get)]
    fn body_used(&self) -> bool {
        self.body.is_some()
    }

    #[qjs(get)]
    fn mode(&self) -> String {
        "navigate".to_string()
    }

    #[qjs(get)]
    fn cache(&self) -> String {
        "no-store".to_string()
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

    async fn bytes(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return TypedArray::new(ctx, bytes).map(|m| m.into_value());
        }
        TypedArray::new(ctx, Vec::<u8>::new()).map(|m| m.into_value())
    }

    async fn blob(&mut self, ctx: Ctx<'js>) -> Result<Blob> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(Blob::from_bytes(bytes, self.content_type.clone()));
        }
        Ok(Blob::from_bytes(Vec::<u8>::new(), None))
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

        Ok(Self {
            url: self.url.clone(),
            method: self.url.clone(),
            headers,
            body: self.body.clone(),
            content_type: self.content_type.clone(),
            signal: self.signal.clone(),
        })
    }
}

fn assign_request<'js>(request: &mut Request<'js>, ctx: Ctx<'js>, obj: &Object<'js>) -> Result<()> {
    if let Some(url) = obj.get_optional("url")? {
        request.url = url;
    }
    if let Some(method) = obj.get_optional("method")? {
        request.method = method;
    }

    if obj.contains_key("signal").unwrap() {
        let signal: Value = obj.get("signal")?;
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

    if obj.contains_key("body").unwrap_or_default() {
        let body: Value = obj.get("body").unwrap();
        if !body.is_undefined() && !body.is_null() {
            if let "GET" | "HEAD" = request.method.as_str() {
                return Err(Exception::throw_type(
                    &ctx,
                    "Failed to construct 'Request': Request with GET/HEAD method cannot have body.",
                ));
            }

            match get_class::<Blob>(&body)? {
                Some(blob) => {
                    let blob = blob.borrow();
                    request.body =
                        Some(TypedArray::<u8>::new(ctx.clone(), blob.get_bytes())?.into_value());
                    request.content_type = Some(blob.mime_type());
                },
                None => {
                    request.body = Some(body);
                    request.content_type = None;
                },
            }
        }
    }

    if obj.contains_key("headers").unwrap() {
        let headers: Value = obj.get("headers")?;
        let headers = Headers::from_value(&ctx, headers)?;
        let headers = Class::instance(ctx, headers)?;
        request.headers = Some(headers);
    }

    Ok(())
}
