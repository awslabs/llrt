// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::future::Future;
use std::sync::OnceLock;

use rquickjs::{
    atom::PredefinedAtom, function::Constructor, CatchResultExt, CaughtError, Ctx, Object, Result,
};
use tokio::sync::oneshot::{self, Receiver};
use tracing::trace;

#[allow(clippy::type_complexity)]
static ERROR_HANDLER: OnceLock<Box<dyn for<'js> Fn(&Ctx<'js>, CaughtError<'js>) + Sync + Send>> =
    OnceLock::new();

pub trait CtxExtension<'js> {
    fn spawn_exit<F, R>(&self, future: F) -> Result<Receiver<R>>
    where
        F: Future<Output = Result<R>> + 'js,
        R: 'js;
}

impl<'js> CtxExtension<'js> for Ctx<'js> {
    fn spawn_exit<F, R>(&self, future: F) -> Result<Receiver<R>>
    where
        F: Future<Output = Result<R>> + 'js,
        R: 'js,
    {
        let ctx = self.clone();

        let type_error_ctor: Constructor = ctx.globals().get(PredefinedAtom::TypeError)?;
        let type_error: Object = type_error_ctor.construct(())?;
        let stack: Option<String> = type_error.get(PredefinedAtom::Stack).ok();

        let (join_channel_tx, join_channel_rx) = oneshot::channel();

        self.spawn(async move {
            match future.await.catch(&ctx) {
                Ok(res) => {
                    //result here dosn't matter if receiver has dropped
                    let _ = join_channel_tx.send(res);
                },
                Err(err) => {
                    let error_handler = match ERROR_HANDLER.get() {
                        Some(handler) => handler,
                        None => {
                            trace!("Future error: {:?}", err);
                            return;
                        },
                    };
                    if let CaughtError::Exception(err) = err {
                        if err.stack().is_none() {
                            if let Some(stack) = stack {
                                err.set(PredefinedAtom::Stack, stack).unwrap();
                            }
                        }
                        error_handler(&ctx, CaughtError::Exception(err));
                    } else {
                        error_handler(&ctx, err);
                    }
                },
            }
        });
        Ok(join_channel_rx)
    }
}

pub fn set_error_handler<F>(handler: F)
where
    F: for<'js> Fn(&Ctx<'js>, CaughtError<'js>) + Sync + Send + 'static,
{
    _ = ERROR_HANDLER.set(Box::new(handler));
}
