use std::path::PathBuf;

use rquickjs::{
    async_with,
    function::IntoArgs,
    module::{Evaluated, ModuleDef},
    promise::MaybePromise,
    AsyncContext, AsyncRuntime, CatchResultExt, Ctx, FromJs, Function, Module, Result,
};

pub async fn given_file(content: &str) -> PathBuf {
    let tmp_dir = std::env::temp_dir();
    let path = tmp_dir.join(nanoid::nanoid!());
    tokio::fs::write(&path, content).await.unwrap();
    path
}

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

pub async fn call_test<'js, T, A>(ctx: &Ctx<'js>, module: &Module<'js, Evaluated>, args: A) -> T
where
    T: FromJs<'js>,
    A: IntoArgs<'js>,
{
    module
        .get::<_, Function>("test")
        .catch(ctx)
        .unwrap()
        .call::<_, MaybePromise>(args)
        .catch(ctx)
        .unwrap()
        .into_future::<T>()
        .await
        .catch(ctx)
        .unwrap()
}

pub struct ModuleEvaluator;

impl ModuleEvaluator {
    pub async fn eval_js<'js>(
        ctx: Ctx<'js>,
        name: &str,
        source: &str,
    ) -> Result<Module<'js, Evaluated>> {
        let (module, module_eval) = Module::declare(ctx, name, source)?.eval()?;
        module_eval.into_future().await?;
        Ok(module)
    }

    pub async fn eval_rust<'js, M>(ctx: Ctx<'js>, name: &str) -> Result<Module<'js, Evaluated>>
    where
        M: ModuleDef,
    {
        let (module, module_eval) = Module::evaluate_def::<M, _>(ctx, name)?;
        module_eval.into_future().await?;
        Ok(module)
    }
}
