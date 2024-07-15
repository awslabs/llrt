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
        let encoding = label
            .0
            .filter(|lbl| !lbl.is_empty())
            .unwrap_or_else(|| String::from("utf-8"));
        let mut fatal = false;
        let mut ignore_bom = false;

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
        let start_pos = if !self.ignore_bom && bytes.len() >= 2 && bytes[..2] == [0xFF, 0xFE] {
            2
        } else if !self.ignore_bom && bytes.len() >= 3 && bytes[..3] == [0xEF, 0xBB, 0xBF] {
            3
        } else {
            0
        };

        self.encoder
            .encode_to_string(&bytes[start_pos..], !self.fatal)
            .or_throw(&ctx)
    }
}
