use std::{cell::RefCell, rc::Rc};

use llrt_utils::option::{Null, Undefined};
use rquickjs::{
    class::Trace, prelude::This, Class, Ctx, Function, JsLifetime, Object, Promise, Result, Value,
};

use crate::{
    readable::controller::ReadableStreamControllerClass,
    utils::promise::{promise_resolved_with, PromisePrimordials},
};

use super::tee::TeeState;

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

type PullRustFn<'js> =
    Box<dyn Fn(Ctx<'js>, ReadableStreamControllerClass<'js>) -> Result<Promise<'js>> + 'js>;

pub enum PullAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
    RustFunction(Rc<PullRustFn<'js>>),
    Tee(Class<'js, TeeState<'js>>),
}

impl<'js> Clone for PullAlgorithm<'js> {
    fn clone(&self) -> Self {
        match self {
            Self::ReturnPromiseUndefined => Self::ReturnPromiseUndefined,
            Self::Function {
                f,
                underlying_source,
            } => Self::Function {
                f: f.clone(),
                underlying_source: underlying_source.clone(),
            },
            Self::RustFunction(rc) => Self::RustFunction(rc.clone()),
            Self::Tee(state) => Self::Tee(state.clone()),
        }
    }
}

impl<'js> Trace<'js> for PullAlgorithm<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        match self {
            Self::ReturnPromiseUndefined => {},
            Self::Function {
                f,
                underlying_source,
            } => {
                f.trace(tracer);
                underlying_source.trace(tracer);
            },
            Self::RustFunction(_) => {},
            Self::Tee(state) => state.trace(tracer),
        }
    }
}

unsafe impl<'js> JsLifetime<'js> for PullAlgorithm<'js> {
    type Changed<'to> = PullAlgorithm<'to>;
}

impl<'js> PullAlgorithm<'js> {
    pub fn from_fn(
        f: impl Fn(Ctx<'js>, ReadableStreamControllerClass<'js>) -> Result<Promise<'js>> + 'js,
    ) -> Self {
        Self::RustFunction(Rc::new(Box::new(f)))
    }

    pub fn from_tee_state(state: Class<'js, TeeState<'js>>) -> Self {
        Self::Tee(state)
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
            PullAlgorithm::Tee(state) => {
                crate::readable::stream::tee::tee_pull_algorithm(ctx, state.clone())
            },
        }
    }
}

type CancelRustFn<'js> = Box<dyn FnOnce(Value<'js>) -> Result<Promise<'js>> + 'js>;

pub enum CancelAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
    RustFunction(Rc<RefCell<Option<CancelRustFn<'js>>>>),
    Tee1(Class<'js, TeeState<'js>>),
    Tee2(Class<'js, TeeState<'js>>),
}

impl<'js> Clone for CancelAlgorithm<'js> {
    fn clone(&self) -> Self {
        match self {
            Self::ReturnPromiseUndefined => Self::ReturnPromiseUndefined,
            Self::Function {
                f,
                underlying_source,
            } => Self::Function {
                f: f.clone(),
                underlying_source: underlying_source.clone(),
            },
            Self::RustFunction(rc) => Self::RustFunction(rc.clone()),
            Self::Tee1(state) => Self::Tee1(state.clone()),
            Self::Tee2(state) => Self::Tee2(state.clone()),
        }
    }
}

impl<'js> Trace<'js> for CancelAlgorithm<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        match self {
            Self::ReturnPromiseUndefined => {},
            Self::Function {
                f,
                underlying_source,
            } => {
                f.trace(tracer);
                underlying_source.trace(tracer);
            },
            Self::RustFunction(_) => {},
            Self::Tee1(state) | Self::Tee2(state) => state.trace(tracer),
        }
    }
}

unsafe impl<'js> JsLifetime<'js> for CancelAlgorithm<'js> {
    type Changed<'to> = CancelAlgorithm<'to>;
}

impl<'js> CancelAlgorithm<'js> {
    pub fn from_fn(f: impl FnOnce(Value<'js>) -> Result<Promise<'js>> + 'js) -> Self {
        Self::RustFunction(Rc::new(RefCell::new(Some(Box::new(f)))))
    }

    pub fn from_tee_state_1(state: Class<'js, TeeState<'js>>) -> Self {
        Self::Tee1(state)
    }

    pub fn from_tee_state_2(state: Class<'js, TeeState<'js>>) -> Self {
        Self::Tee2(state)
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
                promise_resolved_with(&ctx, promise_primordials, result)
            },
            CancelAlgorithm::RustFunction(f) => {
                let f = f
                    .borrow_mut()
                    .take()
                    .expect("cancel algorithm must only be called once");
                f(reason)
            },
            CancelAlgorithm::Tee1(state) => {
                crate::readable::stream::tee::tee_cancel_1_algorithm(ctx, state.clone(), reason)
            },
            CancelAlgorithm::Tee2(state) => {
                crate::readable::stream::tee::tee_cancel_2_algorithm(ctx, state.clone(), reason)
            },
        }
    }
}
