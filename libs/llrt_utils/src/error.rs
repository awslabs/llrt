// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{CatchResultExt, CaughtError, Ctx, Error, IntoJs, Result, Value};

pub trait ErrorExtensions<'js> {
    fn into_value(self, ctx: &Ctx<'js>) -> Result<Value<'js>>;
}

impl<'js> ErrorExtensions<'js> for Error {
    fn into_value(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        Err::<(), _>(self).catch(ctx).unwrap_err().into_value(ctx)
    }
}

impl<'js> ErrorExtensions<'js> for CaughtError<'js> {
    fn into_value(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        Ok(match self {
            CaughtError::Error(err) => err.to_string().into_js(ctx)?,
            CaughtError::Exception(ex) => ex.into_value(),
            CaughtError::Value(val) => val,
        })
    }
}
