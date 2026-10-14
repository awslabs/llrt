// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    prelude::{Func, This},
    Promise, Result, Value,
};

pub trait PromiseExt {
    /// Native code that reads a promise through `result()` or `into_future()` never attaches a
    /// handler to an already rejected promise, so the host rejection tracker would report it as
    /// unhandled. Call this before such a read.
    fn mark_handled(&self) -> Result<()>;
}

impl<'js> PromiseExt for Promise<'js> {
    fn mark_handled(&self) -> Result<()> {
        self.catch()?
            .call::<_, Value>((This(self.clone()), Func::from(|| {})))?;
        Ok(())
    }
}
