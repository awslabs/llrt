use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{body::Incoming, Response};
use rquickjs::{
    class::{JsClass, Trace, Tracer},
    ArrayBuffer, Class, Ctx, Exception, IntoJs, Null, Result, TypedArray, Value,
};
use tokio::runtime::Handle;

use crate::{
    json::parse::json_parse,
    utils::{object::get_bytes, result::ResultExt},
};

use super::blob::Blob;

enum BodyVariant<'js> {
    Incoming(Option<hyper::Response<Incoming>>),
    Provided(Value<'js>),
    Bytes(Bytes),
}

#[derive(Default)]
pub struct Body<'js> {
    data: Option<BodyVariant<'js>>,
    content_type: Option<String>,
}

impl<'js> Trace<'js> for Body<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Some(BodyVariant::Provided(body)) = &self.data {
            body.trace(tracer)
        }
    }
}

impl<'js> Body<'js> {
    pub async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        if let Some(bytes) = self.take_bytes(&ctx).await? {
            return Ok(String::from_utf8_lossy(&bytes).to_string());
        }
        Ok("".into())
    }

    pub async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let bytes = self.take_bytes(&ctx).await?.unwrap_or_default();
        let json = json_parse(&ctx, bytes)?;
        Ok(json)
    }

    pub async fn array_buffer(&mut self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        let bytes = self.take_bytes(&ctx).await?.unwrap_or_default();
        ArrayBuffer::new(ctx, bytes)
    }

    pub async fn blob(&mut self, ctx: Ctx<'js>) -> Result<Blob> {
        let bytes = self.take_bytes(&ctx).await?.unwrap_or_default();
        Ok(Blob::from_bytes(bytes, self.content_type.take())) //no need to copy, we can only take bytes once
    }

    pub fn as_value(&mut self, ctx: &Ctx<'js>, clone: bool) -> Result<Value<'js>> {
        if let Some(variant) = &mut self.data {
            let data = match variant {
                BodyVariant::Incoming(incoming) => {
                    let mut body = incoming
                        .take()
                        .ok_or(Exception::throw_type(ctx, "Already read"))?;

                    let body = tokio::task::block_in_place(move || {
                        Handle::current().block_on(async move { body.body_mut().collect().await })
                    });

                    let bytes = body.or_throw(ctx)?.to_bytes();
                    if clone {
                        self.data.replace(BodyVariant::Bytes(bytes.clone()));
                    }
                    bytes.to_vec()
                }
                BodyVariant::Provided(provided) => {
                    if provided.is_null() || provided.is_undefined() {
                        return Ok(provided.clone());
                    }
                    if let Some(blob) = get_class::<Blob>(provided)? {
                        let blob = blob.borrow();
                        blob.get_bytes()
                    } else {
                        get_bytes(ctx, provided.clone())?
                    }
                }
                BodyVariant::Bytes(bytes) => bytes.to_vec(),
            };

            return Ok(TypedArray::<u8>::new(ctx.clone(), data)?.into_value());
        };

        Null.into_js(ctx)
    }

    pub fn is_used(&self) -> bool {
        self.data.is_none()
    }

    async fn take_bytes(&mut self, ctx: &Ctx<'js>) -> Result<Option<Vec<u8>>> {
        if let Some(variant) = &mut self.data {
            let bytes = match variant {
                BodyVariant::Incoming(incoming) => {
                    let mut body = incoming
                        .take()
                        .ok_or(Exception::throw_type(ctx, "Already read"))?;
                    let bytes = body.body_mut().collect().await.or_throw(ctx)?.to_bytes();
                    bytes.to_vec()
                }
                BodyVariant::Provided(provided) => {
                    if let Some(blob) = get_class::<Blob>(provided)? {
                        let blob = blob.borrow();
                        blob.get_bytes()
                    } else {
                        get_bytes(ctx, provided.clone())?
                    }
                }
                BodyVariant::Bytes(bytes) => bytes.to_vec(),
            };
            return Ok(Some(bytes));
        }
        Ok(None)
    }

    pub fn from_value(body: Option<Value<'js>>) -> Body<'js> {
        Self {
            data: body.map(BodyVariant::Provided),
            content_type: None,
        }
    }

    pub fn from_incoming(body: Response<Incoming>, content_type: Option<String>) -> Body<'js> {
        Self {
            data: Some(BodyVariant::Incoming(Some(body))),
            content_type,
        }
    }
}

#[inline(always)]
fn get_class<'js, C>(provided: &Value<'js>) -> Result<Option<Class<'js, C>>>
where
    C: JsClass<'js>,
{
    if provided
        .as_object()
        .map(|p| p.instance_of::<C>())
        .unwrap_or_default()
    {
        return Ok(Some(Class::<C>::from_value(provided.clone())?));
    }
    Ok(None)
}
