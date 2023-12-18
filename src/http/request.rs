use rquickjs::{class::Trace, function::Opt, methods, Class, Ctx, Object, Result, Value};

use crate::util::ObjectExt;

use super::headers::Headers;

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
        let mut request = Request {
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

    #[qjs(get)]
    fn body(&self) -> Option<Value<'js>> {
        self.body.clone()
    }

    #[qjs(get)]
    fn keepalive(&self) -> bool {
        true
    }
}

fn assign_request<'js>(request: &mut Request<'js>, ctx: Ctx<'js>, obj: &Object<'js>) -> Result<()> {
    if let Some(url) = obj.get_optional("url")? {
        request.url = url;
    }
    if let Some(method) = obj.get_optional("method")? {
        request.method = method;
    }

    if obj.contains_key("headers").unwrap() {
        let headers: Value = obj.get("headers")?;
        let headers = Headers::from_value(ctx.clone(), headers)?;
        let headers = Class::instance(ctx, headers)?;
        request.headers = Some(headers);
    }
    if obj.contains_key("body").unwrap_or_default() {
        let body: Value = obj.get("body").unwrap();
        request.body = Some(body)
    }

    Ok(())
}
