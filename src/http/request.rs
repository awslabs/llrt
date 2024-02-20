// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    class::Trace, function::Opt, methods, Class, Ctx, Exception, Object, Result, Value,
};

use crate::utils::object::ObjectExt;

use super::{body::Body, headers::Headers};

#[rquickjs::class]
pub struct Request<'js> {
    url: String,
    method: String,
    headers: Option<Class<'js, Headers>>,
    body: Body<'js>,
}

impl<'js> Trace<'js> for Request<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        if let Some(headers) = &self.headers {
            headers.trace(tracer);
        }
        self.body.trace(tracer);
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
            body: Body::default(),
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
    fn body(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        self.body.as_value(&ctx, false)
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

        let body = self.body.as_value(&ctx, true)?;

        Ok(Self {
            url: self.url.clone(),
            method: self.url.clone(),
            headers,
            body: Body::from_value(Some(body)),
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
            request.body = Body::from_value(Some(body))
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
