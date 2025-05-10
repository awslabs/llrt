// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_abort::AbortSignal;
use llrt_json::parse::json_parse;
use llrt_url::url_class::URL;
use llrt_utils::{bytes::ObjectBytes, class::get_class, object::ObjectExt, result::ResultExt};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, ArrayBuffer, Class, Ctx, Exception, FromJs,
    IntoJs, Null, Object, Result, TypedArray, Value,
};

use super::{headers::Headers, Blob};

impl<'js> Request<'js> {
    async fn take_bytes(&mut self, ctx: &Ctx<'js>) -> Result<Option<ObjectBytes<'js>>> {
        match &self.body {
            Some(provided) => {
                let bytes = if let Some(blob) = get_class::<Blob>(provided)? {
                    ObjectBytes::Vec(blob.borrow().get_bytes())
                } else {
                    ObjectBytes::from(ctx, provided)?
                };
                Ok(Some(bytes))
            },
            None => Ok(None),
        }
    }
}

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct Request<'js> {
    url: String,
    method: String,
    headers: Option<Class<'js, Headers>>,
    body: Option<Value<'js>>,
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
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, options: Opt<Value<'js>>) -> Result<Self> {
        let mut request = Self {
            url: String::from(""),
            method: "GET".to_string(),
            headers: None,
            body: None,
            signal: None,
        };

        if input.is_string() {
            request.url = input.get()?;
        } else if let Ok(url) = URL::from_js(&ctx, input.clone()) {
            request.url = url.to_string();
        } else if input.is_object() {
            assign_request(&mut request, ctx.clone(), unsafe {
                input.as_object().unwrap_unchecked()
            })?;
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
    fn mode(&self) -> &'static str {
        "navigate"
    }

    #[qjs(get)]
    fn cache(&self) -> &'static str {
        "no-store"
    }

    pub async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            let bytes = bytes.as_bytes(&ctx)?;
            return Ok(String::from_utf8_lossy(bytes).to_string());
        }
        Ok("".into())
    }

    pub async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return json_parse(&ctx, bytes.as_bytes(&ctx)?);
        }
        Err(Exception::throw_syntax(&ctx, "JSON input is empty"))
    }

    async fn array_buffer(&mut self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        let ctx2 = ctx.clone();
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return ArrayBuffer::new(ctx, bytes.as_bytes(&ctx2)?);
        }
        ArrayBuffer::new(ctx, Vec::<u8>::new())
    }

    async fn bytes(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let ctx2 = ctx.clone();
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return TypedArray::new(ctx, bytes.as_bytes(&ctx2)?).map(|m| m.into_value());
        }
        TypedArray::new(ctx, Vec::<u8>::new()).map(|m| m.into_value())
    }

    async fn blob(&mut self, ctx: Ctx<'js>) -> Result<Blob> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            let headers = Headers::from_value(&ctx, self.headers().unwrap().as_value().clone())?;
            let mime_type = headers
                .iter()
                .find_map(|(k, v)| (k == "content-type").then(|| v.to_string()));
            return Ok(Blob::from_bytes(
                bytes.try_into().or_throw(&ctx)?,
                mime_type,
            ));
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

    if let Some(body) = obj.get_optional::<_, Value>("body")? {
        if !body.is_undefined() && !body.is_null() {
            if let "GET" | "HEAD" = request.method.as_str() {
                return Err(Exception::throw_type(
                    &ctx,
                    "Failed to construct 'Request': Request with GET/HEAD method cannot have body.",
                ));
            }

            request.body = if let Some(blob) = body.as_object().and_then(Class::<Blob>::from_object)
            {
                let blob = blob.borrow();
                Some(TypedArray::<u8>::new(ctx.clone(), blob.get_bytes())?.into_value())
            } else {
                Some(body)
            }
        }
    }

    if let Some(headers) = obj.get_optional("headers")? {
        let headers = Headers::from_value(&ctx, headers)?;
        let headers = Class::instance(ctx, headers)?;
        request.headers = Some(headers);
    }

    Ok(())
}
