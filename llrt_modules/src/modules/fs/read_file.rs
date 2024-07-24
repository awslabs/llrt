// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use either::Either;
use llrt_utils::{object::ObjectExt, result::ResultExt};
use rquickjs::{function::Opt, Ctx, Error, FromJs, IntoJs, Result, Value};
use tokio::fs;

use crate::modules::buffer::Buffer;

pub async fn read_file(
    ctx: Ctx<'_>,
    path: String,
    options: Opt<Either<String, ReadFileOptions>>,
) -> Result<Value<'_>> {
    let bytes = fs::read(&path)
        .await
        .or_throw_msg(&ctx, &["Can't read \"", &path, "\""].concat())?;

    handle_read_file_bytes(&ctx, options, bytes)
}

pub fn read_file_sync(
    ctx: Ctx<'_>,
    path: String,
    options: Opt<Either<String, ReadFileOptions>>,
) -> Result<Value<'_>> {
    let bytes =
        std::fs::read(&path).or_throw_msg(&ctx, &["Can't read \"", &path, "\""].concat())?;

    handle_read_file_bytes(&ctx, options, bytes)
}

pub(crate) fn handle_read_file_bytes<'a>(
    ctx: &Ctx<'a>,
    options: Opt<Either<String, ReadFileOptions>>,
    bytes: Vec<u8>,
) -> Result<Value<'a>> {
    let buffer = Buffer(bytes);

    if let Some(options) = options.0 {
        let encoding = match options {
            Either::Left(encoding) => Some(encoding),
            Either::Right(options) => options.encoding,
        };

        if let Some(encoding) = encoding {
            return buffer
                .to_string(ctx, &encoding)
                .and_then(|s| s.into_js(ctx));
        }
    }

    buffer.into_js(ctx)
}

pub(crate) struct ReadFileOptions {
    pub encoding: Option<String>,
}

impl<'js> FromJs<'js> for ReadFileOptions {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let encoding = obj.get_optional::<_, String>("encoding")?;

        Ok(Self { encoding })
    }
}
