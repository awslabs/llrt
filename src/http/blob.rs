use std::{ops::RangeInclusive, time::Instant};

use http_body_util::BodyExt;
use hyper::body::{Bytes, Incoming};
use rquickjs::{
    class::{Trace, Tracer},
    function::Opt,
    qjs, Class, Coerced, Ctx, Exception, IntoJs, Object, Result, TypedArray, Value,
};
use tracing::trace;

use crate::{
    json::parse::json_parse,
    utils::{object::get_bytes, result::ResultExt},
};

use super::{body::Body, headers::Headers};

use once_cell::sync::Lazy;
use std::collections::HashMap;

enum EndingType {
    Native,
    Transparent,
}

#[rquickjs::class]
#[derive(Trace)]
pub struct Blob {
    data: Vec<u8>,
    mime_type: String,
}

fn normalize_type(mut mime_type: String) -> String {
    static INVALID_RANGE: RangeInclusive<u8> = 0x0020..=0x007E;

    let bytes = unsafe { mime_type.as_bytes_mut() };
    for byte in bytes {
        if !INVALID_RANGE.contains(byte) {
            return String::new();
        }
        byte.make_ascii_lowercase();
    }
    mime_type
}

#[rquickjs::methods]
impl Blob {
    #[qjs(constructor)]
    fn new<'js>(ctx: Ctx<'js>, parts: Opt<Value<'js>>, options: Opt<Object<'js>>) -> Result<Self> {
        let mut endings = EndingType::Transparent;
        let mut mime_type = String::new();

        if let Some(opts) = options.0 {
            if let Some(x) = opts.get::<_, Option<Coerced<String>>>("type")? {
                mime_type = normalize_type(x.to_string());
            }
            if let Some(Coerced(endings_opt)) = opts.get::<_, Option<Coerced<String>>>("endings")? {
                if endings_opt == "native" {
                    endings = EndingType::Native;
                } else if endings_opt != "transparent" {
                    return Err(Exception::throw_type(
                        &ctx,
                        r#"expected 'endings' to be either 'transparent' or 'native'"#,
                    ));
                }
            }
        }

        let data = if let Some(parts) = parts.0 {
            let array = parts
				.into_array()
				.ok_or_else(|| Exception::throw_type(&ctx, "Failed to construct 'Blob': The provided value cannot be converted to a sequence."))?;

            for elem in array.iter::<Value>() {
                let elem = elem?;
                //append(&ctx, elem, endings, &mut buffer)?;
            }
            Vec::new()
        } else {
            Vec::new()
        };

        Ok(Self { data, mime_type })
    }

    async fn text(&mut self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }

    async fn array_buffer(&self) -> Vec<u8> {
        self.data.clone()
    }
}
