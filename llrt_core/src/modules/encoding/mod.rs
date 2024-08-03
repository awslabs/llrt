// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
pub mod encoder {
    pub use llrt_utils::encoding::*;
}
pub mod text_decoder;
pub mod text_encoder;

use rquickjs::{Class, Ctx, Result};

use self::text_decoder::TextDecoder;
use self::text_encoder::TextEncoder;

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    Class::<TextEncoder>::define(&globals)?;
    Class::<TextDecoder>::define(&globals)?;

    Ok(())
}
