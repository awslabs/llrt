// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::result::Result as StdResult;

use llrt_events::Emitter;
use rquickjs::{prelude::This, Class, Ctx, IntoJs, Result, Value};
use tokio::sync::broadcast::error::RecvError;

pub mod readable;
pub mod writable;

pub fn set_destroyed_and_error<'js>(
    is_destroyed: &mut bool,
    error_value: &mut Option<Value<'js>>,
    error: StdResult<Option<Value<'js>>, RecvError>,
) {
    *is_destroyed = true;
    if let Ok(error) = error {
        *error_value = error
    }
}
const DEFAULT_BUFFER_SIZE: usize = 1024 * 16;

pub trait SteamEvents<'js>
where
    Self: Emitter<'js>,
{
    fn emit_close(this: Class<'js, Self>, ctx: &Ctx<'js>, had_error: bool) -> Result<()> {
        Self::emit_str(
            This(this),
            ctx,
            "close",
            vec![had_error.into_js(ctx)?],
            false,
        )
    }

    #[allow(dead_code)]
    fn emit_end(this: Class<'js, Self>, ctx: &Ctx<'js>) -> Result<()> {
        Self::emit_str(This(this), ctx, "end", vec![], false)
    }
}

#[macro_export]
macro_rules! impl_stream_events {

    ($($struct:ident),*) => {
        $(
            impl<'js> $crate::SteamEvents<'js> for $struct<'js> {}
        )*
    };
}
