use http_body_util::BodyExt;
use hyper::{body::Incoming, Response};
use rquickjs::{
    class::{self, Trace, Tracer},
    Ctx, Exception, Result, TypedArray, Value,
};

use crate::{
    json::parse::json_parse,
    utils::{object::get_bytes, result::ResultExt},
};

enum BodyVariant<'js> {
    Incoming(hyper::Response<Incoming>),
    Provided(Value<'js>),
}

#[derive(Default)]
pub struct Body<'js> {
    data: Option<BodyVariant<'js>>,
}

impl<'js> Trace<'js> for Body<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let Some(data) = &self.data {
            if let BodyVariant::Provided(body) = data {
                body.trace(tracer)
            }
        }
    }
}

impl<'js> Body<'js> {
    pub async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        let bytes = self.take_bytes(&ctx).await?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        Ok(text)
    }

    pub async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let bytes = self.take_bytes(&ctx).await?;
        let json = json_parse(&ctx, bytes)?;
        Ok(json)
    }

    pub async fn array_buffer(&mut self, ctx: Ctx<'js>) -> Result<Vec<u8>> {
        let bytes = self.take_bytes(&ctx).await?;
        Ok(bytes)
    }

    pub async fn blob(&mut self, ctx: Ctx<'js>) -> Result<TypedArray<'js, u8>> {
        let bytes = self.take_bytes(&ctx).await?;
        TypedArray::new(ctx, bytes)
    }

    pub fn as_value(&self) -> Option<Value<'js>> {
        if let Some(data) = &self.data {
            if let BodyVariant::Provided(body) = data {
                return Some(body.clone());
            }
        }
        None
    }

    pub fn is_used(&self) -> bool {
        self.data.is_none()
    }

    async fn take_bytes(&mut self, ctx: &Ctx<'js>) -> Result<Vec<u8>> {
        let variant = self
            .data
            .take()
            .ok_or(Exception::throw_type(ctx, "Already read"))?;

        let bytes = match variant {
            BodyVariant::Incoming(mut body) => {
                let bytes = body.body_mut().collect().await.or_throw(ctx)?.to_bytes();
                bytes.to_vec()
            }
            BodyVariant::Provided(body) => get_bytes(ctx, body)?,
        };

        Ok(bytes)
    }

    pub fn from_value(body: Value<'js>) -> Body<'js> {
        Self {
            data: Some(BodyVariant::Provided(body)),
        }
    }

    pub fn from_incoming(body: Response<Incoming>) -> Body<'js> {
        Self {
            data: Some(BodyVariant::Incoming(body)),
        }
    }
}
