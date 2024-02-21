// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    class::Trace, function::Opt, methods, Class, Ctx, Exception, IntoJs, Null, Object, Result,
    TypedArray, Value,
};

use crate::utils::{class::get_class, object::ObjectExt};

use super::{blob::Blob, headers::Headers};

#[rquickjs::class]
pub struct Request<'js> {
    url: String,
    method: String,
    headers: Option<Class<'js, Headers>>,
    body: Option<Value<'js>>,
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

#[methods]
impl<'js> Request<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut request = Self {
            url: String::from(""),
            method: "GET".to_string(),
            headers: None,
            body: None,
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

    if obj.contains_key("body").unwrap_or_default() {
        let body: Value = obj.get("body").unwrap();
        if !body.is_undefined() && !body.is_null() {
            if let "GET" | "HEAD" = request.method.as_str() {
                return Err(Exception::throw_type(
                    &ctx,
                    "Failed to construct 'Request': Request with GET/HEAD method cannot have body.",
                ));
            }

            request.body = if let Some(blob) = get_class::<Blob>(&body)? {
                let blob = blob.borrow();
                Some(TypedArray::<u8>::new(ctx.clone(), blob.get_bytes())?.into_value())
            } else {
                Some(body)
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
