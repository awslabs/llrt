use llrt_utils::{
    module::{export_default, ModuleInfo},
    option::Undefined,
    primordials::Primordial,
};
use queuing_strategy::{ByteLengthQueuingStrategy, CountQueuingStrategy, SizeValue};
use readable::{
    ReadableByteStreamController, ReadableStream, ReadableStreamBYOBReader,
    ReadableStreamBYOBRequest, ReadableStreamClass, ReadableStreamDefaultController,
    ReadableStreamDefaultReader,
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{JsClass, OwnedBorrowMut, Trace, Tracer},
    function::Constructor,
    module::{Declarations, Exports, ModuleDef},
    prelude::{IntoArg, OnceFn, This},
    promise::PromiseState,
    Class, Ctx, Error, Exception, FromJs, Function, IntoAtom, IntoJs, JsLifetime, Object, Promise,
    Result, Value,
};
use std::collections::VecDeque;
use std::{cell::Cell, rc::Rc};
use writable::{
    WritableStream, WritableStreamClass, WritableStreamDefaultController,
    WritableStreamDefaultWriter,
};

mod queuing_strategy;
mod readable;
mod writable;

struct ReadableWritablePair<'js> {
    readable: ReadableStreamClass<'js>,
    writable: WritableStreamClass<'js>,
}

impl<'js> FromJs<'js> for ReadableWritablePair<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
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

fn class_from_owned_borrow_mut<'js, T: JsClass<'js>>(
    borrow: OwnedBorrowMut<'js, T>,
) -> (Class<'js, T>, OwnedBorrowMut<'js, T>) {
    let class = borrow.into_inner();
    let borrow = OwnedBorrowMut::from_class(class.clone());
    (class, borrow)
}

// the trait used elsewhere in this repo accepts null values as 'None', which causes many web platform tests to fail as they
// like to check that undefined is accepted and null isn't.
trait ValueOrUndefined<'js> {
    fn get_value_or_undefined<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(
        &self,
        k: K,
    ) -> Result<Option<V>>;
}

impl<'js> ValueOrUndefined<'js> for Object<'js> {
    fn get_value_or_undefined<K: IntoAtom<'js> + Clone, V: FromJs<'js> + Sized>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        let value = self.get::<K, Value<'js>>(k)?;
        Ok(Undefined::from_js(self.ctx(), value)?.0)
    }
}

impl<'js> ValueOrUndefined<'js> for Value<'js> {
    fn get_value_or_undefined<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        if let Some(obj) = self.as_object() {
            return obj.get_value_or_undefined(k);
        }
        Ok(None)
    }
}

trait UnwrapOrUndefined<'js> {
    fn unwrap_or_undefined(self, ctx: &Ctx<'js>) -> Value<'js>;
}

impl<'js> UnwrapOrUndefined<'js> for Option<Value<'js>> {
    fn unwrap_or_undefined(self, ctx: &Ctx<'js>) -> Value<'js> {
        self.unwrap_or_else(|| Value::new_undefined(ctx.clone()))
    }
}

fn promise_rejected_with<'js>(
    primordials: &PromisePrimordials<'js>,
    value: Value<'js>,
) -> Result<Promise<'js>> {
    primordials
        .promise_reject
        .call((This(primordials.promise_constructor.clone()), value))
}

fn promise_resolved_with<'js>(
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
struct PromisePrimordials<'js> {
    promise_constructor: Constructor<'js>,
    promise_resolve: Function<'js>,
    promise_reject: Function<'js>,
    promise_all: Function<'js>,
    promise_resolved_with_undefined: Promise<'js>,
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

fn upon_promise_fulfilment<'js, Input: FromJs<'js> + 'js, Output: IntoJs<'js> + 'js>(
    ctx: Ctx<'js>,
    promise: Promise<'js>,
    then: impl FnOnce(Ctx<'js>, Input) -> Result<Output> + 'js,
) -> Result<Promise<'js>> {
    promise.then()?.call((
        This(promise.clone()),
        Function::new(ctx.clone(), OnceFn::new(then)),
    ))
}

fn is_non_negative_number(value: SizeValue<'_>) -> Option<f64> {
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
    resolve: Option<Function<'js>>,
    reject: Option<Function<'js>>,
}

impl<'js> ResolveablePromise<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self> {
        let (promise, resolve, reject) = Promise::new(ctx)?;
        Ok(Self {
            promise,
            resolve: Some(resolve),
            reject: Some(reject),
        })
    }

    fn resolved_with_undefined(primordials: &PromisePrimordials<'js>) -> Self {
        Self {
            promise: primordials.promise_resolved_with_undefined.clone(),
            resolve: None,
            reject: None,
        }
    }

    fn rejected_with(primordials: &PromisePrimordials<'js>, error: Value<'js>) -> Result<Self> {
        Ok(Self {
            promise: promise_rejected_with(primordials, error)?,
            resolve: None,
            reject: None,
        })
    }

    fn resolve(&self, value: impl IntoArg<'js>) -> Result<()> {
        if let Some(resolve) = &self.resolve {
            let () = resolve.call((value,))?;
        }
        Ok(())
    }

    fn resolve_undefined(&self) -> Result<()> {
        if let Some(resolve) = &self.resolve {
            let () = resolve.call((rquickjs::Undefined,))?;
        }
        Ok(())
    }

    fn reject(&self, value: impl IntoArg<'js>) -> Result<()> {
        if let Some(reject) = &self.reject {
            let () = reject.call((value,))?;
        }
        Ok(())
    }

    fn is_pending(&self) -> bool {
        self.promise.state() == PromiseState::Pending
    }

    fn set_is_handled(&self) -> Result<()> {
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

#[derive(JsLifetime, Trace)]
struct Container<'js> {
    queue: VecDeque<ValueWithSize<'js>>,
    queue_total_size: f64,
}

impl<'js> Container<'js> {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            queue_total_size: 0.0,
        }
    }

    fn enqueue_value_with_size(
        &mut self,
        ctx: &Ctx<'js>,
        value: Value<'js>,
        size: SizeValue<'js>,
    ) -> Result<()> {
        let size = match is_non_negative_number(size) {
            None => {
                // If ! IsNonNegativeNumber(size) is false, throw a RangeError exception.
                return Err(Exception::throw_range(
                    ctx,
                    "Size must be a finite, non-NaN, non-negative number.",
                ));
            },
            Some(size) => size,
        };

        // If size is +∞, throw a RangeError exception.
        if size.is_infinite() {
            return Err(Exception::throw_range(
                ctx,
                "Size must be a finite, non-NaN, non-negative number.",
            ));
        };

        // Append a new value-with-size with value value and size size to container.[[queue]].
        self.queue.push_back(ValueWithSize { value, size });

        // Set container.[[queueTotalSize]] to container.[[queueTotalSize]] + size.
        self.queue_total_size += size;

        Ok(())
    }

    fn dequeue_value(&mut self) -> Value<'js> {
        // Let valueWithSize be container.[[queue]][0].
        // Remove valueWithSize from container.[[queue]].
        let value_with_size = self
            .queue
            .pop_front()
            .expect("DequeueValue called with empty queue");
        // Set container.[[queueTotalSize]] to container.[[queueTotalSize]] − valueWithSize’s size.
        self.queue_total_size -= value_with_size.size;
        // If container.[[queueTotalSize]] < 0, set container.[[queueTotalSize]] to 0. (This can occur due to rounding errors.)
        if self.queue_total_size < 0.0 {
            self.queue_total_size = 0.0
        }
        value_with_size.value
    }
}
