// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{function::Opt, Ctx, IntoJs, Result, Value};
use tokio::fs;

use crate::{
    buffer::Buffer,
    utils::{object::ObjectExt, result::ResultExt},
};

pub async fn read_file<'js>(
    ctx: Ctx<'js>,
    path: String,
    options: Opt<Value<'js>>,
) -> Result<Value<'js>> {
    let bytes = fs::read(&path)
        .await
        .or_throw_msg(&ctx, &format!("Can't read \"{}\"", &path))?;

    handle_read_file_bytes(&ctx, options, bytes)
}

pub fn read_file_sync<'js>(
    ctx: Ctx<'js>,
    path: String,
    options: Opt<Value<'js>>,
) -> Result<Value<'js>> {
    let bytes = std::fs::read(&path).or_throw_msg(&ctx, &format!("Can't read \"{}\"", &path))?;

    handle_read_file_bytes(&ctx, options, bytes)
}
fn handle_read_file_bytes<'a>(
    ctx: &Ctx<'a>,
    options: Opt<Value>,
    bytes: Vec<u8>,
) -> Result<Value<'a>> {
    let buffer = Buffer(bytes);

    if let Some(options) = options.0 {
        let encoding = if options.is_string() {
            options.as_string().unwrap().to_string().map(Some)?
        } else {
            options.get_optional::<_, String>("encoding")?
        };

        if let Some(encoding) = encoding {
            return buffer
                .to_string(ctx, &encoding)
                .and_then(|s| s.into_js(ctx));
        }
    }

    buffer.into_js(ctx)
}
