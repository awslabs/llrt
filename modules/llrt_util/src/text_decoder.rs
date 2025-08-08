// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_encoding::Encoder;
use llrt_utils::{bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{function::Opt, Ctx, Object, Result};

#[rquickjs::class]
#[derive(rquickjs::class::Trace, rquickjs::JsLifetime)]
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
        let mut fatal = false;
        let mut ignore_bom = false;

        let encoder = Encoder::from_optional_str(label.as_deref()).or_throw_range(&ctx, "")?;

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

    pub fn decode(&self, ctx: Ctx<'js>, bytes: ObjectBytes<'js>) -> Result<String> {
        let bytes = bytes.as_bytes(&ctx)?;
        let start_pos = if !self.ignore_bom {
            match bytes.get(..3) {
                Some([0xFF, 0xFE, ..]) if self.encoder == Encoder::Utf16le => 2,
                Some([0xFE, 0xFF, ..]) if self.encoder == Encoder::Utf16be => 2,
                Some([0xEF, 0xBB, 0xBF]) if self.encoder == Encoder::Utf8 => 3,
                _ => 0,
            }
        } else {
            0
        };

        self.encoder
            .encode_to_string(&bytes[start_pos..], !self.fatal)
            .or_throw_type(&ctx, "")
    }
}
