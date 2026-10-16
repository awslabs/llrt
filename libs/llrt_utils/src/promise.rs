// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{promise::PromiseFuture, qjs, FromJs, Promise};

// Bridge until the upstream rquickjs fix for marking promises as handled is released.
pub trait PromiseExt<'js> {
    /// Native code that reads a promise through `result()` or `into_future()` never attaches a
    /// handler to an already rejected promise, so the host rejection tracker would report a
    /// rejection that native code surfaces itself as unhandled.
    fn mark_handled(&self);

    fn into_future_handled<T: FromJs<'js>>(self) -> PromiseFuture<'js, T>;
}

impl<'js> PromiseExt<'js> for Promise<'js> {
    fn mark_handled(&self) {
        // SAFETY: the context pointer and the promise value both come from this live `Promise`,
        // which owns a reference to the value and borrows its context for `'js`.
        unsafe { qjs::JS_PromiseMarkAsHandled(self.ctx().as_raw().as_ptr(), self.as_raw()) }
    }

    fn into_future_handled<T: FromJs<'js>>(self) -> PromiseFuture<'js, T> {
        self.mark_handled();
        self.into_future()
    }
}
