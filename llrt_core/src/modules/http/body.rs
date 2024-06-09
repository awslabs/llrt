// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use http_body_util::BodyExt;
use hyper::{body::Incoming, Response};
use rquickjs::{
    class::{Trace, Tracer},
    ArrayBuffer, Class, Ctx, Exception, IntoJs, Null, Result, TypedArray, Value,
};

use crate::{
    json::parse::json_parse,
    utils::{class::get_class, object::get_bytes, result::ResultExt},
};

use super::blob::Blob;

enum BodyVariant<'js> {
    Incoming(Option<hyper::Response<Incoming>>),
    Provided(Value<'js>),
}

#[rquickjs::class]
pub struct Body<'js> {
    data: BodyVariant<'js>,
    content_type: Option<String>,
}

impl<'js> Trace<'js> for Body<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        if let BodyVariant::Provided(body) = &self.data {
            body.trace(tracer)
        }
    }
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> Body<'js> {
    pub async fn text(&mut self, ctx: Ctx<'js>) -> Result<String> {
        let bytes = self.take_bytes(&ctx).await?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    pub async fn json(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let bytes = self.take_bytes(&ctx).await?;
        let json = json_parse(&ctx, bytes)?;
        Ok(json)
    }

    pub async fn array_buffer(&mut self, ctx: Ctx<'js>) -> Result<ArrayBuffer<'js>> {
        let bytes = self.take_bytes(&ctx).await?;
        ArrayBuffer::new(ctx, bytes)
    }

    pub async fn typed_array(&mut self, ctx: Ctx<'js>) -> Result<TypedArray<'js, u8>> {
        let bytes = self.take_bytes(&ctx).await?;
        TypedArray::<u8>::new(ctx, bytes)
    }

    pub async fn blob(&mut self, ctx: Ctx<'js>) -> Result<Blob> {
        let bytes = self.take_bytes(&ctx).await?;
        Ok(Blob::from_bytes(bytes, self.content_type.take())) //no need to copy, we can only take bytes once
    }

    pub async fn bytes(&mut self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let bytes = self.take_bytes(&ctx).await?;
        TypedArray::new(ctx, bytes).map(|m| m.into_value())
    }

    pub fn is_used(&self) -> bool {
        if let BodyVariant::Incoming(data) = &self.data {
            return data.is_none();
        }
        false
    }
}

impl<'js> Body<'js> {
    pub async fn get_text(ctx: Ctx<'js>, body: &Option<Class<'js, Self>>) -> Result<String> {
        if let Some(body) = body {
            return body.borrow_mut().text(ctx).await;
        }
        Ok("".into())
    }

    pub async fn get_json(ctx: Ctx<'js>, body: &Option<Class<'js, Self>>) -> Result<Value<'js>> {
        if let Some(body) = body {
            return body.borrow_mut().json(ctx).await;
        }
        Err(Exception::throw_syntax(&ctx, "JSON input is empty"))
    }

    pub async fn get_array_buffer(
        ctx: Ctx<'js>,
        body: &Option<Class<'js, Self>>,
    ) -> Result<ArrayBuffer<'js>> {
        if let Some(body) = body {
            return body.borrow_mut().array_buffer(ctx).await;
        }
        ArrayBuffer::new(ctx, Vec::<u8>::new())
    }

    pub async fn get_blob(ctx: Ctx<'js>, body: &Option<Class<'js, Self>>) -> Result<Blob> {
        if let Some(body) = body {
            return body.borrow_mut().blob(ctx).await;
        }
        Ok(Blob::from_bytes(Vec::<u8>::new(), None))
    }

    pub fn get_body(ctx: Ctx<'js>, body: &Option<Class<'js, Self>>) -> Result<Value<'js>> {
        if let Some(body) = body {
            return Ok(body.clone().into_value());
        }
        Null.into_js(&ctx)
    }

    pub fn from_value(
        ctx: &Ctx<'js>,
        body: Option<Value<'js>>,
    ) -> Result<Option<Class<'js, Self>>> {
        if let Some(body) = body {
            if body.is_null() || body.is_undefined() {
                return Ok(None);
            }

            return Ok(Some(Class::instance(
                ctx.clone(),
                Self {
                    data: BodyVariant::Provided(body),
                    content_type: None,
                },
            )?));
        }
        Ok(None)
    }

    pub fn from_incoming(
        ctx: Ctx<'js>,
        response: Response<Incoming>,
        content_type: Option<String>,
    ) -> Result<Class<'js, Self>> {
        Class::instance(
            ctx,
            Self {
                data: BodyVariant::Incoming(Some(response)),
                content_type,
            },
        )
    }

    pub async fn take_bytes(&mut self, ctx: &Ctx<'js>) -> Result<Vec<u8>> {
        let bytes = match &mut self.data {
            BodyVariant::Incoming(incoming) => {
                let mut body = incoming
                    .take()
                    .ok_or(Exception::throw_type(ctx, "Already read"))?;
                let bytes = body.body_mut().collect().await.or_throw(ctx)?.to_bytes();
                bytes.to_vec()
            },
            BodyVariant::Provided(provided) => {
                if let Some(blob) = get_class::<Blob>(provided)? {
                    let blob = blob.borrow();
                    blob.get_bytes()
                } else {
                    get_bytes(ctx, provided.clone())?
                }
            },
        };
        Ok(bytes)
    }
}
