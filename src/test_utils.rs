// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[cfg(test)]
pub mod utils {

    use rquickjs::{
        loader::{BuiltinLoader, BuiltinResolver, FileResolver, ScriptLoader},
        markers::ParallelSend,
        AsyncContext, AsyncRuntime, CatchResultExt, Ctx, Result,
    };

    pub async fn new_js_runtime() -> (AsyncRuntime, AsyncContext) {
        use rquickjs::{AsyncContext, AsyncRuntime};

        let runtime = AsyncRuntime::new().unwrap();
        runtime.set_max_stack_size(512 * 1024).await;
        runtime
            .set_loader(
                (
                    FileResolver::default().with_path("/").with_path("."),
                    BuiltinResolver::default(),
                ),
                (ScriptLoader::default(), BuiltinLoader::default()),
            )
            .await;
        let ctx = AsyncContext::full(&runtime).await.unwrap();

        (runtime, ctx)
    }

    pub async fn with_js_runtime<F>(f: F)
    where
        F: for<'js> FnOnce(Ctx<'js>) -> Result<()> + ParallelSend,
    {
        let (_, ctx) = new_js_runtime().await;

        ctx.with(|ctx| {
            f(ctx.clone()).catch(&ctx).unwrap();
        })
        .await;
    }
}
