// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::RwLock;

use llrt_abort::AbortSignal;
use llrt_json::parse::json_parse;
use llrt_url::{url_class::URL, url_search_params::URLSearchParams};
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{
    atom::PredefinedAtom, class::Trace, function::Opt, ArrayBuffer, Class, Ctx, Exception, FromJs,
    IntoJs, Null, Object, Result, TypedArray, Value,
};

use super::{
    headers::{Headers, HeadersGuard, HEADERS_KEY_CONTENT_TYPE},
    strip_bom, Blob, MIME_TYPE_APPLICATION, MIME_TYPE_TEXT,
};

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
        Ok(match s.to_ascii_lowercase().as_str() {
            "cors" => RequestMode::Cors,
            "no-cors" => RequestMode::NoCors,
            "same-origin" => RequestMode::SameOrigin,
            "navigate" => RequestMode::Navigate,
            _ => return Err(["Invalid requrest mode: ", s.as_str()].concat()),
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

#[rquickjs::class]
#[derive(rquickjs::JsLifetime)]
pub struct Request<'js> {
    url: String,
    method: String,
    headers: Option<Class<'js, Headers>>,
    body: RwLock<BodyVariant<'js>>,
    signal: Option<Class<'js, AbortSignal<'js>>>,
    mode: RequestMode,
    keepalive: bool,
}

impl<'js> Trace<'js> for Request<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        if let Some(headers) = &self.headers {
            headers.trace(tracer);
        }
        let body = self.body.read().unwrap();
        let body = &*body;
        if let BodyVariant::Provided(Some(body)) = body {
            body.trace(tracer);
        }
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Request<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, input: Value<'js>, options: Opt<Value<'js>>) -> Result<Self> {
        let mut request = Self {
            url: "".into(),
            method: "GET".into(),
            headers: None,
            body: RwLock::new(BodyVariant::Empty),
            signal: None,
            mode: RequestMode::Cors,
            keepalive: false,
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
        let body = self.body.read().unwrap();
        let body = &*body;
        match body {
            BodyVariant::Provided(value) => value.into_js(&ctx),
            BodyVariant::Empty => Null.into_js(&ctx),
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
        let body = self.body.read().unwrap();
        let body = &*body;
        match body {
            BodyVariant::Provided(value) => value.is_none(),
            BodyVariant::Empty => false,
        }
    }

    #[qjs(get)]
    fn mode(&self) -> &str {
        self.mode.as_str()
    }

    #[qjs(get)]
    fn cache(&self) -> &'static str {
        "no-store"
    }

    pub async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(String::from_utf8_lossy(&strip_bom(&bytes)).to_string());
        }
        Ok("".into())
    }

    pub async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return json_parse(&ctx, strip_bom(&bytes));
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
        let mime_type = self
            .headers()
            .map(|headers| {
                Headers::from_value(&ctx, headers.as_value().clone(), HeadersGuard::None)
            })
            .transpose()?
            .and_then(|headers| {
                headers
                    .iter()
                    .find_map(|(k, v)| (k == HEADERS_KEY_CONTENT_TYPE).then(|| v.to_string()))
            });

        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(Blob::from_bytes(bytes, mime_type));
        }
        Ok(Blob::from_bytes(Vec::<u8>::new(), mime_type))
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

        //not async so should not block
        let body = self.body.read().unwrap();
        let body = &*body;
        let body = match body {
            BodyVariant::Provided(provided) => BodyVariant::Provided(provided.clone()),
            BodyVariant::Empty => BodyVariant::Empty,
        };

        Ok(Self {
            url: self.url.clone(),
            method: self.url.clone(),
            headers,
            body: RwLock::new(body),
            signal: self.signal.clone(),
            mode: self.mode.clone(),
            keepalive: self.keepalive,
        })
    }
}

impl<'js> Request<'js> {
    #[allow(clippy::await_holding_lock)] //clippy complains about guard being held across await points but we drop the guard before awaiting
    #[allow(clippy::readonly_write_lock)] //clippy complains about lock being read only but we mutate the value
    async fn take_bytes(&self, ctx: &Ctx<'js>) -> Result<Option<Vec<u8>>> {
        let mut body_guard = self.body.write().unwrap();
        let body = &mut *body_guard;
        let bytes = match body {
            BodyVariant::Provided(provided) => {
                let provided = provided
                    .take()
                    .ok_or(Exception::throw_message(ctx, "Already read"))?;
                drop(body_guard);
                if let Some(blob) = provided.as_object().and_then(Class::<Blob>::from_object) {
                    let blob = blob.borrow();
                    blob.get_bytes()
                } else {
                    let bytes = ObjectBytes::from(ctx, &provided)?;
                    bytes.as_bytes(ctx)?.to_vec()
                }
            },
            BodyVariant::Empty => return Ok(None),
        };

        Ok(Some(bytes))
    }
}

fn assign_request<'js>(request: &mut Request<'js>, ctx: Ctx<'js>, obj: &Object<'js>) -> Result<()> {
    if let Some(url) = obj.get_optional("url")? {
        request.url = url;
    }
    if let Some(method) = obj.get_optional::<_, String>("method")? {
        let method = method.to_ascii_uppercase();
        if let "CONNECT" | "TRACE" | "TRACK" = method.as_str() {
            return Err(Exception::throw_type(&ctx, "This method is not allowed."));
        }
        request.method = method;
    }
    if let Some(mode) = obj.get_optional::<_, String>("mode")? {
        request.mode = mode.try_into().or_throw(&ctx)?;
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
            } else if let Some(obj) = body.as_object() {
                if let Some(blob) = Class::<Blob>::from_object(obj) {
                    let blob = blob.borrow();
                    if !blob.mime_type().is_empty() {
                        content_type = Some(blob.mime_type());
                    }
                    BodyVariant::Provided(Some(body))
                } else if obj.instance_of::<URLSearchParams>() {
                    content_type = Some(MIME_TYPE_APPLICATION.into());
                    BodyVariant::Provided(Some(body))
                } else {
                    BodyVariant::Provided(Some(body))
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
        if !headers.has(ctx.clone(), HEADERS_KEY_CONTENT_TYPE.into())? {
            if let Some(value) = content_type {
                headers.set(
                    ctx.clone(),
                    HEADERS_KEY_CONTENT_TYPE.into(),
                    value.into_js(&ctx)?,
                )?;
            }
        }
        Class::instance(ctx, headers)?
    };
    request.headers = Some(headers);

    Ok(())
}
