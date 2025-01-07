use std::{cell::Cell, rc::Rc};

use llrt_utils::primordials::Primordial;
use rquickjs::{
    atom::PredefinedAtom,
    class::{Trace, Tracer},
    function::Constructor,
    prelude::{IntoArg, OnceFn, This},
    promise::PromiseState,
    Ctx, Error, FromJs, Function, IntoJs, JsLifetime, Promise, Result, Value,
};

pub fn promise_rejected_with<'js>(
    primordials: &PromisePrimordials<'js>,
    value: Value<'js>,
) -> Result<Promise<'js>> {
    primordials
        .promise_reject
        .call((This(primordials.promise_constructor.clone()), value))
}

pub fn promise_resolved_with<'js>(
    ctx: &Ctx<'js>,
    primordials: &PromisePrimordials<'js>,
    value: Result<Value<'js>>,
) -> Result<Promise<'js>> {
    match value {
        Ok(value) => primordials
            .promise_resolve
            .call((This(primordials.promise_constructor.clone()), value)),
        Err(Error::Exception) => primordials
            .promise_reject
            .call((This(primordials.promise_constructor.clone()), ctx.catch())),
        Err(err) => Err(err),
    }
}

#[derive(JsLifetime, Clone)]
pub struct PromisePrimordials<'js> {
    pub promise_constructor: Constructor<'js>,
    pub promise_resolve: Function<'js>,
    pub promise_reject: Function<'js>,
    pub promise_all: Function<'js>,
    pub promise_resolved_with_undefined: Promise<'js>,
}

impl<'js> Primordial<'js> for PromisePrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self>
    where
        Self: Sized,
    {
        let promise_constructor: Constructor<'js> = ctx.globals().get(PredefinedAtom::Promise)?;
        let promise_resolve: Function<'js> = promise_constructor.get("resolve")?;
        let promise_reject: Function<'js> = promise_constructor.get("reject")?;
        let promise_all: Function<'js> = promise_constructor.get("all")?;

        let promise_resolved_with_undefined = promise_resolve.call((
            This(promise_constructor.clone()),
            Value::new_undefined(ctx.clone()),
        ))?;

        Ok(Self {
            promise_constructor,
            promise_resolve,
            promise_reject,
            promise_all,
            promise_resolved_with_undefined,
        })
    }
}

// https://webidl.spec.whatwg.org/#dfn-perform-steps-once-promise-is-settled
pub fn upon_promise<'js, Input: FromJs<'js> + 'js, Output: IntoJs<'js> + 'js>(
    ctx: Ctx<'js>,
    promise: Promise<'js>,
    then: impl FnOnce(Ctx<'js>, std::result::Result<Input, Value<'js>>) -> Result<Output> + 'js,
) -> Result<Promise<'js>> {
    let then = Rc::new(Cell::new(Some(then)));
    let then2 = then.clone();
    promise.then()?.call((
        This(promise.clone()),
        Function::new(
            ctx.clone(),
            OnceFn::new(move |ctx, input| {
                then.take()
                    .expect("Promise.then should only call either resolve or reject")(
                    ctx,
                    Ok(input),
                )
            }),
        ),
        Function::new(
            ctx,
            OnceFn::new(move |ctx, e: Value<'js>| {
                then2
                    .take()
                    .expect("Promise.then should only call either resolve or reject")(
                    ctx, Err(e)
                )
            }),
        ),
    ))
}

pub fn upon_promise_fulfilment<'js, Input: FromJs<'js> + 'js, Output: IntoJs<'js> + 'js>(
    ctx: Ctx<'js>,
    promise: Promise<'js>,
    then: impl FnOnce(Ctx<'js>, Input) -> Result<Output> + 'js,
) -> Result<Promise<'js>> {
    promise.then()?.call((
        This(promise.clone()),
        Function::new(ctx.clone(), OnceFn::new(then)),
    ))
}

#[derive(Debug, JsLifetime, Clone)]
pub struct ResolveablePromise<'js> {
    pub promise: Promise<'js>,
    resolve: Option<Function<'js>>,
    reject: Option<Function<'js>>,
}

impl<'js> ResolveablePromise<'js> {
    pub fn new(ctx: &Ctx<'js>) -> Result<Self> {
        let (promise, resolve, reject) = Promise::new(ctx)?;
        Ok(Self {
            promise,
            resolve: Some(resolve),
            reject: Some(reject),
        })
    }

    pub fn resolved_with_undefined(primordials: &PromisePrimordials<'js>) -> Self {
        Self {
            promise: primordials.promise_resolved_with_undefined.clone(),
            resolve: None,
            reject: None,
        }
    }

    pub fn rejected_with(primordials: &PromisePrimordials<'js>, error: Value<'js>) -> Result<Self> {
        Ok(Self {
            promise: promise_rejected_with(primordials, error)?,
            resolve: None,
            reject: None,
        })
    }

    pub fn resolve(&self, value: impl IntoArg<'js>) -> Result<()> {
        if let Some(resolve) = &self.resolve {
            let () = resolve.call((value,))?;
        }
        Ok(())
    }

    pub fn resolve_undefined(&self) -> Result<()> {
        if let Some(resolve) = &self.resolve {
            let () = resolve.call((rquickjs::Undefined,))?;
        }
        Ok(())
    }

    pub fn reject(&self, value: impl IntoArg<'js>) -> Result<()> {
        if let Some(reject) = &self.reject {
            let () = reject.call((value,))?;
        }
        Ok(())
    }

    pub fn is_pending(&self) -> bool {
        self.promise.state() == PromiseState::Pending
    }

    pub fn set_is_handled(&self) -> Result<()> {
        self.promise.catch()?.call((
            This(self.promise.clone()),
            Function::new(self.promise.ctx().clone(), || {}),
        ))
    }
}

impl<'js> Trace<'js> for ResolveablePromise<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.promise.trace(tracer);
        self.resolve.trace(tracer);
        self.reject.trace(tracer);
    }
}
