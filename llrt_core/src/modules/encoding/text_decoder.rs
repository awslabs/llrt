// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{encoding::Encoder, object::ObjectExt};
use rquickjs::{function::Opt, Ctx, Object, Result, Value};

use crate::utils::{object::get_bytes, result::ResultExt};

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct TextDecoder {
    #[qjs(skip_trace)]
    encoder: Encoder,
    fatal: bool,
    ignore_bom: bool,
}

#[rquickjs::methods]
impl<'js> TextDecoder {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, label: Opt<String>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut encoding = label.0.unwrap_or(String::from("utf-8"));
        let mut fatal = false;
        let mut ignore_bom = false;

        if encoding.is_empty() {
            encoding = String::from("utf-8");
        }

        let encoder = Encoder::from_str(&encoding).or_throw(&ctx)?;

        if let Some(options) = options.0 {
            if let Some(opt) = options.get_optional("fatal")? {
                fatal = opt;
            }
            if let Some(opt) = options.get_optional("ignoreBOM")? {
                ignore_bom = opt;
            }
        }

        Ok(TextDecoder {
            encoder,
            fatal,
            ignore_bom,
        })
    }

    #[qjs(get)]
    fn encoding(&self) -> &str {
        self.encoder.as_label()
    }

    #[qjs(get)]
    fn fatal(&self) -> bool {
        self.fatal
    }

    #[qjs(get, rename = "ignoreBOM")]
    fn ignore_bom(&self) -> bool {
        self.ignore_bom
    }

    pub fn decode(&self, ctx: Ctx<'js>, buffer: Value<'js>) -> Result<String> {
        let bytes = get_bytes(&ctx, buffer)?;

        self.encoder.encode_to_string(&bytes).or_throw(&ctx)
    }
}
