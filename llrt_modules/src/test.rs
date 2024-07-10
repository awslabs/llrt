use rquickjs::{async_with, AsyncContext, AsyncRuntime, Ctx};

pub async fn test_async_with<F>(func: F)
where
    F: for<'js> FnOnce(Ctx<'js>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'js>>
        + Send,
{
    let rt = AsyncRuntime::new().unwrap();
    let ctx = AsyncContext::full(&rt).await.unwrap();

    async_with!(ctx => |ctx| {
        func(ctx).await
    })
    .await;
}
