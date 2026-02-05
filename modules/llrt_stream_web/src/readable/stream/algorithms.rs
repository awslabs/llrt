use std::rc::Rc;

use llrt_utils::option::{Null, Undefined};
use rquickjs::prelude::OnceFn;
use rquickjs::{
    class::Trace, prelude::This, Ctx, Function, JsLifetime, Object, Promise, Result, Value,
};

use crate::{
    readable::controller::ReadableStreamControllerClass,
    utils::promise::{promise_resolved_with, PromisePrimordials},
};

#[derive(Clone)]
pub enum StartAlgorithm<'js> {
    ReturnUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> StartAlgorithm<'js> {
    pub(crate) fn call(
        &self,
        ctx: Ctx<'js>,
        controller: ReadableStreamControllerClass<'js>,
    ) -> Result<Value<'js>> {
        match self {
            StartAlgorithm::ReturnUndefined => Ok(Value::new_undefined(ctx.clone())),
            StartAlgorithm::Function {
                f,
                underlying_source,
            } => f.call::<_, Value>((This(underlying_source.clone()), controller)),
        }
    }
}

#[derive(Trace, Clone)]
pub enum PullAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
    RustFunction(#[qjs(skip_trace)] Rc<PullRustFunction<'js>>),
}

unsafe impl<'js> JsLifetime<'js> for PullAlgorithm<'js> {
    type Changed<'to> = PullAlgorithm<'to>;
}

type PullRustFunction<'js> =
    Box<dyn Fn(Ctx<'js>, ReadableStreamControllerClass<'js>) -> Result<Promise<'js>> + 'js>;

impl<'js> PullAlgorithm<'js> {
    pub fn from_fn(
        f: impl Fn(Ctx<'js>, ReadableStreamControllerClass<'js>) -> Result<Promise<'js>> + 'js,
    ) -> Self {
        Self::RustFunction(Rc::new(Box::new(f)))
    }

    pub(crate) fn call(
        &self,
        ctx: Ctx<'js>,
        promise_primordials: &PromisePrimordials<'js>,
        controller: ReadableStreamControllerClass<'js>,
    ) -> Result<Promise<'js>> {
        match self {
            PullAlgorithm::ReturnPromiseUndefined => {
                Ok(promise_primordials.promise_resolved_with_undefined.clone())
            },
            PullAlgorithm::Function {
                f,
                underlying_source,
            } => promise_resolved_with(
                &ctx,
                promise_primordials,
                f.call::<_, Value>((This(underlying_source.clone()), controller)),
            ),
            PullAlgorithm::RustFunction(f) => f(ctx, controller),
        }
    }
}

#[derive(Clone, Trace)]
pub enum CancelAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
    RustFunction(#[qjs(skip_trace)] Rc<OnceFn<CancelRustFunction<'js>>>),
}

unsafe impl<'js> JsLifetime<'js> for CancelAlgorithm<'js> {
    type Changed<'to> = CancelAlgorithm<'to>;
}

type CancelRustFunction<'js> = Box<dyn FnOnce(Value<'js>) -> Result<Promise<'js>> + 'js>;

impl<'js> CancelAlgorithm<'js> {
    pub fn from_fn(f: impl FnOnce(Value<'js>) -> Result<Promise<'js>> + 'js) -> Self {
        Self::RustFunction(Rc::new(OnceFn::new(Box::new(f))))
    }

    pub(crate) fn call(
        &self,
        ctx: Ctx<'js>,
        promise_primordials: &PromisePrimordials<'js>,
        reason: Value<'js>,
    ) -> Result<Promise<'js>> {
        match self {
            CancelAlgorithm::ReturnPromiseUndefined => {
                Ok(promise_primordials.promise_resolved_with_undefined.clone())
            },
            CancelAlgorithm::Function {
                f,
                underlying_source,
            } => {
                let result: Result<Value> = f.call((This(underlying_source.clone()), reason));
                let promise = promise_resolved_with(&ctx, promise_primordials, result);
                promise
            },
            CancelAlgorithm::RustFunction(f) => {
                f.take().expect("cancel algorithm must only be called once")(reason)
            },
        }
    }
}
