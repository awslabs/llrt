// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::future::Future;
use std::sync::OnceLock;

use llrt_utils::primordials::{BasePrimordials, Primordial};
use rquickjs::{atom::PredefinedAtom, CatchResultExt, CaughtError, Ctx, Object, Result};
use tokio::sync::oneshot::{self, Receiver};
use tracing::trace;

#[allow(clippy::type_complexity)]
static ERROR_HANDLER: OnceLock<Box<dyn for<'js> Fn(&Ctx<'js>, CaughtError<'js>) + Sync + Send>> =
    OnceLock::new();

pub trait CtxExtension<'js> {
    /// Despite naming, this will not necessarily exit the parent process.
    /// It depends on the handler set by `set_spawn_error_handler`.
    fn spawn_exit<F, R>(&self, future: F) -> Result<Receiver<R>>
    where
        F: Future<Output = Result<R>> + 'js,
        R: 'js;

    fn spawn_exit_simple<F>(&self, future: F)
    where
        F: Future<Output = Result<()>> + 'js;
}

impl<'js> CtxExtension<'js> for Ctx<'js> {
    fn spawn_exit<F, R>(&self, future: F) -> Result<Receiver<R>>
    where
        F: Future<Output = Result<R>> + 'js,
        R: 'js,
    {
        let ctx = self.clone();

        let primordials = BasePrimordials::get(self)?;
        let type_error: Object = primordials.constructor_type_error.construct(())?;
        let stack: Option<String> = type_error.get(PredefinedAtom::Stack).ok();

        let (join_channel_tx, join_channel_rx) = oneshot::channel();

        self.spawn(async move {
            match future.await.catch(&ctx) {
                Ok(res) => {
                    //result here doesn't matter if receiver has dropped
                    let _ = join_channel_tx.send(res);
                },
                Err(err) => handle_spawn_error(&ctx, err, stack),
            }
        });
        Ok(join_channel_rx)
    }

    /// Same as above but fire & forget and without a forced stack trace collection
    fn spawn_exit_simple<F>(&self, future: F)
    where
        F: Future<Output = Result<()>> + 'js,
    {
        let ctx = self.clone();
        self.spawn(async move {
            if let Err(err) = future.await.catch(&ctx) {
                handle_spawn_error(&ctx, err, None)
            }
        });
    }
}

fn handle_spawn_error<'js>(ctx: &Ctx<'js>, err: CaughtError<'js>, stack: Option<String>) {
    let error_handler = if let Some(handler) = ERROR_HANDLER.get() {
        handler
    } else {
        trace!("Future error: {:?}", err);
        return;
    };
    if let CaughtError::Exception(err) = err {
        if err.stack().is_none() {
            if let Some(stack) = stack {
                err.set(PredefinedAtom::Stack, stack).unwrap();
            }
        }
        error_handler(ctx, CaughtError::Exception(err));
    } else {
        error_handler(ctx, err);
    }
}

pub fn set_spawn_error_handler<F>(handler: F)
where
    F: for<'js> Fn(&Ctx<'js>, CaughtError<'js>) + Sync + Send + 'static,
{
    _ = ERROR_HANDLER.set(Box::new(handler));
}
