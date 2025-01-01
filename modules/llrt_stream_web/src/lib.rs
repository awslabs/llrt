use std::{cell::Cell, rc::Rc};

use llrt_utils::module::{export_default, ModuleInfo};
use queueing_strategy::{ByteLengthQueuingStrategy, CountQueuingStrategy};
use readable::{
    ReadableByteStreamController, ReadableStream, ReadableStreamBYOBReader,
    ReadableStreamBYOBRequest, ReadableStreamClass, ReadableStreamDefaultController,
    ReadableStreamDefaultReader,
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{JsClass, OwnedBorrow, OwnedBorrowMut, Trace, Tracer},
    function::Constructor,
    module::{Declarations, Exports, ModuleDef},
    prelude::{IntoArg, OnceFn, This},
    promise::PromiseState,
    Class, Ctx, Error, FromJs, Function, IntoAtom, IntoJs, JsLifetime, Object, Promise, Result,
    Type, Value,
};
use writable::{
    WritableStream, WritableStreamClass, WritableStreamDefaultController,
    WritableStreamDefaultWriter,
};

mod queueing_strategy;
mod readable;
mod writable;

struct ReadableWritablePair<'js> {
    readable: ReadableStreamClass<'js>,
    writable: WritableStreamClass<'js>,
}

impl<'js> FromJs<'js> for ReadableWritablePair<'js> {
    fn from_js(_ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let readable = obj.get::<_, ReadableStreamClass<'js>>("readable")?;
        let writable = obj.get::<_, Class<'js, WritableStream>>("writable")?;

        Ok(Self { readable, writable })
    }
}

pub struct StreamWebModule;

// https://nodejs.org/api/webstreams.html
impl ModuleDef for StreamWebModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(ReadableStream))?;
        declare.declare(stringify!(ReadableStreamDefaultReader))?;
        declare.declare(stringify!(ReadableStreamBYOBReader))?;
        declare.declare(stringify!(ReadableStreamDefaultController))?;
        declare.declare(stringify!(ReadableByteStreamController))?;
        declare.declare(stringify!(ReadableStreamBYOBRequest))?;

        declare.declare(stringify!(WritableStream))?;
        declare.declare(stringify!(WritableStreamDefaultWriter))?;
        declare.declare(stringify!(WritableStreamDefaultController))?;

        declare.declare(stringify!(ByteLengthQueuingStrategy))?;
        declare.declare(stringify!(CountQueuingStrategy))?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            Class::<ReadableStream>::define(default)?;
            Class::<ReadableStreamDefaultReader>::define(default)?;
            Class::<ReadableStreamBYOBReader>::define(default)?;
            Class::<ReadableStreamDefaultController>::define(default)?;
            Class::<ReadableByteStreamController>::define(default)?;
            Class::<ReadableStreamBYOBRequest>::define(default)?;

            Class::<WritableStream>::define(default)?;
            Class::<WritableStreamDefaultWriter>::define(default)?;
            Class::<WritableStreamDefaultController>::define(default)?;

            Class::<ByteLengthQueuingStrategy>::define(default)?;
            Class::<CountQueuingStrategy>::define(default)?;

            Ok(())
        })?;

        Ok(())
    }
}

impl From<StreamWebModule> for ModuleInfo<StreamWebModule> {
    fn from(val: StreamWebModule) -> Self {
        ModuleInfo {
            name: "stream/web",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx) -> Result<()> {
    let globals = &ctx.globals();

    // https://min-common-api.proposal.wintercg.org/#api-index
    Class::<ByteLengthQueuingStrategy>::define(globals)?;
    Class::<CountQueuingStrategy>::define(globals)?;

    Class::<ReadableByteStreamController>::define(globals)?;
    Class::<ReadableStream>::define(globals)?;
    Class::<ReadableStreamBYOBReader>::define(globals)?;
    Class::<ReadableStreamBYOBRequest>::define(globals)?;
    Class::<ReadableStreamDefaultReader>::define(globals)?;

    Class::<WritableStream>::define(globals)?;
    Class::<WritableStreamDefaultController>::define(globals)?;

    Ok(())
}

/// Helper type for treating an undefined value as None, but not null
#[derive(Clone)]
struct Undefined<T>(pub Option<T>);

impl<'js, T: FromJs<'js>> FromJs<'js> for Undefined<T> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.type_of() == Type::Undefined {
            Ok(Self(None))
        } else {
            Ok(Self(Some(FromJs::from_js(ctx, value)?)))
        }
    }
}

impl<T> Default for Undefined<T> {
    fn default() -> Self {
        Self(None)
    }
}

unsafe impl<'js, T: JsLifetime<'js>> JsLifetime<'js> for Undefined<T> {
    type Changed<'to> = Undefined<T::Changed<'to>>;
}

impl<'js, T: Trace<'js>> Trace<'js> for Undefined<T> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.0.trace(tracer)
    }
}

/// Helper type for converting an option into null instead of undefined.
#[derive(Clone)]
struct Null<T>(pub Option<T>);

impl<'js, T: IntoJs<'js>> IntoJs<'js> for Null<T> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self.0 {
            None => Ok(Value::new_null(ctx.clone())),
            Some(val) => val.into_js(ctx),
        }
    }
}

unsafe impl<'js, T: JsLifetime<'js>> JsLifetime<'js> for Null<T> {
    type Changed<'to> = Null<T::Changed<'to>>;
}

impl<'js, T: Trace<'js>> Trace<'js> for Null<T> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.0.trace(tracer)
    }
}

impl<'js, T: IntoJs<'js>> IntoJs<'js> for Undefined<T> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self.0 {
            None => Ok(Value::new_undefined(ctx.clone())),
            Some(val) => val.into_js(ctx),
        }
    }
}

fn downgrade_owned_borrow_mut<'js, T: JsClass<'js>>(
    borrow: OwnedBorrowMut<'js, T>,
) -> OwnedBorrow<'js, T> {
    OwnedBorrow::from_class(borrow.into_inner())
}

fn class_from_owned_borrow_mut<'js, T: JsClass<'js>>(
    borrow: OwnedBorrowMut<'js, T>,
) -> (Class<'js, T>, OwnedBorrowMut<'js, T>) {
    let class = borrow.into_inner();
    let borrow = OwnedBorrowMut::from_class(class.clone());
    (class, borrow)
}

// the trait used elsewhere in this repo accepts null values as 'None', which causes many web platform tests to fail as they
// like to check that undefined is accepted and null isn't.
pub trait ObjectExt<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>>;
}

impl<'js> ObjectExt<'js> for Object<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js> + Sized>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        let value = self.get::<K, Value<'js>>(k)?;
        Ok(Undefined::from_js(self.ctx(), value)?.0)
    }
}

impl<'js> ObjectExt<'js> for Value<'js> {
    fn get_optional<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(&self, k: K) -> Result<Option<V>> {
        if let Some(obj) = self.as_object() {
            return obj.get_optional(k);
        }
        Ok(None)
    }
}

fn promise_rejected_with<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Promise<'js>> {
    let promise: Constructor<'js> = ctx.globals().get(PredefinedAtom::Promise)?;
    let promise_reject: Function<'js> = promise.get("reject")?;

    promise_reject.call((This(promise), value))
}

fn promise_resolved_with<'js>(ctx: &Ctx<'js>, value: Result<Value<'js>>) -> Result<Promise<'js>> {
    let promise: Constructor<'js> = ctx.globals().get(PredefinedAtom::Promise)?;

    match value {
        Ok(value) => {
            let promise_resolve: Function<'js> = promise.get("resolve")?;
            promise_resolve.call((This(promise.clone()), value))
        },
        Err(Error::Exception) => {
            let promise_reject: Function<'js> = promise.get("reject")?;
            promise_reject.call((This(promise.clone()), ctx.catch()))
        },
        Err(err) => Err(err),
    }
}

// https://webidl.spec.whatwg.org/#dfn-perform-steps-once-promise-is-settled
fn upon_promise<'js, Input: FromJs<'js> + 'js, Output: IntoJs<'js> + 'js>(
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

fn is_non_negative_number(value: Value<'_>) -> Option<f64> {
    // If Type(v) is not Number, return false.
    let number = value.as_number()?;
    // If v is NaN, return false.
    if number.is_nan() {
        return None;
    }

    // If v < 0, return false.
    if number < 0.0 {
        return None;
    }

    // Return true.
    Some(number)
}

#[derive(JsLifetime, Trace, Clone)]
struct ValueWithSize<'js> {
    value: Value<'js>,
    size: f64,
}

#[derive(Debug, JsLifetime, Clone)]
struct ResolveablePromise<'js> {
    promise: Promise<'js>,
    resolve: Function<'js>,
    reject: Function<'js>,
}

impl<'js> ResolveablePromise<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self> {
        let (promise, resolve, reject) = Promise::new(ctx)?;
        Ok(Self {
            promise,
            resolve,
            reject,
        })
    }

    fn resolved_with(ctx: &Ctx<'js>, value: Result<Value<'js>>) -> Result<Self> {
        Ok(Self {
            promise: promise_resolved_with(ctx, value)?,
            resolve: Function::new(ctx.clone(), || {})?,
            reject: Function::new(ctx.clone(), || {})?,
        })
    }

    fn rejected_with(ctx: &Ctx<'js>, error: Value<'js>) -> Result<Self> {
        Ok(Self {
            promise: promise_rejected_with(ctx, error)?,
            resolve: Function::new(ctx.clone(), || {})?,
            reject: Function::new(ctx.clone(), || {})?,
        })
    }

    fn resolve(&self, value: impl IntoArg<'js>) -> Result<()> {
        let () = self.resolve.call((value,))?;
        Ok(())
    }

    fn reject(&self, value: impl IntoArg<'js>) -> Result<()> {
        let () = self.reject.call((value,))?;
        Ok(())
    }

    fn is_pending(&self) -> bool {
        self.promise.state() == PromiseState::Pending
    }

    fn set_is_handled(&self) -> Result<()> {
        self.promise.then()?.call((
            This(self.promise.clone()),
            Value::new_undefined(self.promise.ctx().clone()),
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
