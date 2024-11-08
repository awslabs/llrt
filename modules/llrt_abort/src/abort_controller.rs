// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    prelude::{Opt, This},
    Class, Ctx, JsLifetime, Result, Value,
};

use super::AbortSignal;

#[rquickjs::class]
#[derive(rquickjs::class::Trace)]
pub struct AbortController<'js> {
    signal: Class<'js, AbortSignal<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for AbortController<'js> {
    type Changed<'to> = AbortController<'to>;
}

#[rquickjs::methods]
impl<'js> AbortController<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>) -> Result<Self> {
        let signal = AbortSignal::new();

        let abort_controller = Self {
            signal: Class::instance(ctx, signal)?,
        };
        Ok(abort_controller)
    }

    #[qjs(get)]
    pub fn signal(&self) -> Class<'js, AbortSignal<'js>> {
        self.signal.clone()
    }

    pub fn abort(
        ctx: Ctx<'js>,
        this: This<Class<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<()> {
        let instance = this.0.borrow();
        let signal = instance.signal.clone();
        let mut signal_borrow = signal.borrow_mut();
        if signal_borrow.aborted {
            //only once
            return Ok(());
        }
        signal_borrow.set_reason(reason);
        drop(signal_borrow);
        AbortSignal::send_aborted(This(signal), ctx)?;

        Ok(())
    }
}
