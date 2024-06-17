// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{function::Opt, Ctx, Exception, Object, Result, Value};

use std::borrow::Cow;

use crate::utils::{object::get_bytes, object::ObjectExt, result::ResultExt};
use encoding_rs::Encoding;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct TextDecoder {
    #[qjs(skip_trace)]
    encoding: String,
    fatal: bool,
    ignore_bom: bool,
}

#[rquickjs::methods]
impl<'js> TextDecoder {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, label: Opt<String>, options: Opt<Object<'js>>) -> Result<Self> {
        let label = label
            .0
            .filter(|lbl| !lbl.is_empty())
            .unwrap_or_else(|| String::from("utf-8"));
        let mut fatal = false;
        let mut ignore_bom = false;

        let encoding = Encoding::for_label(label.as_bytes())
            .map(|enc| enc.name().to_string())
            .or_throw_msg(&ctx, "Unsupported encoding label")?;

        if let Some(options) = options.0 {
            if let Some(opt) = options.get_optional("fatal")? {
                fatal = opt;
            }
            if let Some(opt) = options.get_optional("ignoreBOM")? {
                ignore_bom = opt;
            }
        }

        Ok(TextDecoder {
            encoding,
            fatal,
            ignore_bom,
        })
    }

    #[qjs(get)]
    fn encoding(&self) -> String {
        let s = self.encoding.clone();
        s.replace('_', "-").to_ascii_lowercase()
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

        let decoder = Encoding::for_label(self.encoding.as_bytes()).unwrap();

        let str: Cow<str>;
        let has_error: bool;

        if decoder == encoding_rs::UTF_8 {
            (str, has_error) = match self.ignore_bom {
                false => decoder.decode_with_bom_removal(&bytes),
                true => decoder.decode_without_bom_handling(&bytes),
            }
        } else {
            (str, _, has_error) = decoder.decode(&bytes);
        }

        if self.fatal && has_error {
            return Err(Exception::throw_message(&ctx, "Fatal error"));
        }

        Ok(str.into_owned())
    }
}
