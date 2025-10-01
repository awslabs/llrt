use std::{
    fs,
    path::{Path, PathBuf},
};

use rquickjs::{
    async_with,
    function::IntoArgs,
    loader::{BuiltinLoader, Resolver},
    markers::ParallelSend,
    module::{Evaluated, ModuleDef},
    promise::MaybePromise,
    AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, Ctx, FromJs, Function, Module, Result,
};

pub async fn given_file(content: &str) -> PathBuf {
    let tmp_dir = std::env::temp_dir();
    let path = tmp_dir.join(uuid::Uuid::new_v4().to_string());
    tokio::fs::write(&path, content).await.unwrap();
    path
}

struct TestResolver;

impl Resolver for TestResolver {
    fn resolve(&mut self, _ctx: &Ctx<'_>, base: &str, name: &str) -> Result<String> {
        if !name.starts_with(".") {
            return Ok(name.into());
        }
        let base = Path::new(base);
        let combined_path = base.join(name);
        Ok(fs::canonicalize(combined_path)
            .unwrap()
            .to_string_lossy()
            .to_string())
    }
}

pub async fn given_runtime() -> (AsyncRuntime, AsyncContext) {
    let rt = AsyncRuntime::new().unwrap();
    rt.set_loader((TestResolver,), (BuiltinLoader::default(),))
        .await;
    let ctx = AsyncContext::full(&rt).await.unwrap();

    (rt, ctx)
}

pub async fn test_async_with<F>(func: F)
where
    F: for<'js> FnOnce(Ctx<'js>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'js>>
        + Send,
{
    test_async_with_opts(func, TestOptions::default()).await;
}

#[derive(Default)]
pub struct TestOptions {
    no_pending_jobs: bool,
}

impl TestOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn no_pending_jobs(mut self) -> Self {
        self.no_pending_jobs = true;
        self
    }
}

pub async fn test_async_with_opts<F>(func: F, options: TestOptions)
where
    F: for<'js> FnOnce(Ctx<'js>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'js>>
        + Send,
{
    let (rt, ctx) = given_runtime().await;

    async_with!(ctx => |ctx| {
        func(ctx).await
    })
    .await;

    if options.no_pending_jobs {
        assert!(!rt.is_job_pending().await);
    }
}

pub async fn test_sync_with<F>(func: F)
where
    F: for<'js> FnOnce(Ctx<'js>) -> Result<()> + ParallelSend,
{
    let (_rt, ctx) = given_runtime().await;

    ctx.with(|ctx| func(ctx.clone()).catch(&ctx).unwrap()).await;
}

pub async fn call_test<'js, T, A>(ctx: &Ctx<'js>, module: &Module<'js, Evaluated>, args: A) -> T
where
    T: FromJs<'js>,
    A: IntoArgs<'js>,
{
    call_test_err(ctx, module, args).await.unwrap()
}

pub async fn call_test_err<'js, T, A>(
    ctx: &Ctx<'js>,
    module: &Module<'js, Evaluated>,
    args: A,
) -> std::result::Result<T, CaughtError<'js>>
where
    T: FromJs<'js>,
    A: IntoArgs<'js>,
{
    module
        .get::<_, Function>("test")
        .catch(ctx)?
        .call::<_, MaybePromise>(args)
        .catch(ctx)?
        .into_future::<T>()
        .await
        .catch(ctx)
}

pub struct ModuleEvaluator;

impl ModuleEvaluator {
    pub async fn eval_js<'js>(
        ctx: Ctx<'js>,
        name: &str,
        source: &str,
    ) -> Result<Module<'js, Evaluated>> {
        let (module, module_eval) = Module::declare(ctx, name, source)?.eval()?;
        module_eval.into_future::<()>().await?;
        Ok(module)
    }

    pub async fn eval_rust<'js, M>(ctx: Ctx<'js>, name: &str) -> Result<Module<'js, Evaluated>>
    where
        M: ModuleDef,
    {
        let (module, module_eval) = Module::evaluate_def::<M, _>(ctx, name)?;
        module_eval.into_future::<()>().await?;
        Ok(module)
    }
}
