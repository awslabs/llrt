#[cfg(test)]
pub mod utils {
    use rquickjs::{markers::ParallelSend, CatchResultExt, Ctx, Result};

    pub async fn with_runtime<F>(f: F)
    where
        F: for<'js> FnOnce(Ctx<'js>) -> Result<()> + ParallelSend,
    {
        use rquickjs::{AsyncContext, AsyncRuntime};

        use crate::allocator::MimallocAllocator;

        let runtime = AsyncRuntime::new_with_alloc(MimallocAllocator).unwrap();
        runtime.set_max_stack_size(512 * 1024).await;
        let ctx = AsyncContext::full(&runtime).await.unwrap();

        ctx.with(|ctx| {
            f(ctx.clone()).catch(&ctx).unwrap();
        })
        .await;
    }
}
