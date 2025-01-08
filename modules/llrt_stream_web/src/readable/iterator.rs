use std::{
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use llrt_utils::{
    object::CreateSymbol,
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{JsClass, OwnedBorrow, OwnedBorrowMut, Trace, Tracer},
    function::Constructor,
    methods,
    prelude::{Opt, This},
    Class, Ctx, Error, Exception, FromJs, Function, IntoAtom, IntoJs, JsLifetime, Object, Promise,
    Result, Symbol, Type, Value,
};

use super::{
    controller::ReadableStreamControllerOwned, reader::ReadableStreamGenericReader,
    ReadableStreamClassObjects, ReadableStreamDefaultReader, ReadableStreamObjects,
    ReadableStreamReadResult,
};
use crate::{
    readable::default_reader::ReadableStreamDefaultReaderOwned,
    utils::{
        class_from_owned_borrow_mut,
        promise::{promise_resolved_with, PromisePrimordials},
    },
};
use crate::{
    readable::{objects::ReadableStreamDefaultReaderObjects, ReadableStreamReadRequest},
    utils::{
        promise::{upon_promise, upon_promise_fulfilment, ResolveablePromise},
        UnwrapOrUndefined,
    },
};

pub(super) enum IteratorKind {
    Async,
}

pub(super) struct IteratorRecord<'js> {
    pub(super) iterator: Object<'js>,
    next_method: Function<'js>,
    done: AtomicBool,

    sync_to_async_iterator: Function<'js>,
}

impl<'js> IteratorRecord<'js> {
    pub(super) fn get_iterator(
        ctx: &Ctx<'js>,
        obj: Value<'js>,
        kind: IteratorKind,
    ) -> Result<Self> {
        let method: Option<Function<'js>> = match kind {
            // If kind is async, then
            IteratorKind::Async => {
                // Let method be ? GetMethod(obj, %Symbol.asyncIterator%).
                let method = get_method(ctx, obj.clone(), Symbol::async_iterator(ctx.clone()))?;
                // If method is undefined, then
                if method.is_none() {
                    // Let syncMethod be ? GetMethod(obj, %Symbol.iterator%).
                    let sync_method = get_method(ctx, obj.clone(), Symbol::iterator(ctx.clone()))?;

                    // If syncMethod is undefined, throw a TypeError exception.
                    let sync_method = match sync_method {
                        None => {
                            return Err(Exception::throw_type(ctx, "Object is not an iterator"));
                        },
                        Some(sync_method) => sync_method,
                    };

                    // Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
                    let sync_iterator_record =
                        Self::get_iterator_from_method(ctx, &obj, sync_method)?;

                    // Return CreateAsyncFromSyncIterator(syncIteratorRecord).
                    return sync_iterator_record.create_async_from_sync_iterator(ctx);
                }

                method
            },
        };

        // If method is undefined, throw a TypeError exception.
        match method {
            None => Err(Exception::throw_type(ctx, "Object is not an iterator")),
            Some(method) => {
                // Return ? GetIteratorFromMethod(obj, method).
                Self::get_iterator_from_method(ctx, &obj, method)
            },
        }
    }

    fn get_iterator_from_method(
        ctx: &Ctx<'js>,
        obj: &Value<'js>,
        method: Function<'js>,
    ) -> Result<Self> {
        // Let iterator be ? Call(method, obj).
        let iterator: Value<'js> = method.call((This(obj),))?;
        let iterator = match iterator.into_object() {
            Some(iterator) => iterator,
            None => {
                return Err(Exception::throw_type(
                    ctx,
                    "The iterator method must return an object",
                ));
            },
        };
        // Let nextMethod be ? Get(iterator, "next").
        let next_method = iterator.get(PredefinedAtom::Next)?;
        // Let iteratorRecord be the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
        // Return iteratorRecord.
        Ok(Self {
            iterator,
            next_method,
            done: AtomicBool::new(false),
            sync_to_async_iterator: IteratorPrimordials::get(ctx)?
                .sync_to_async_iterator
                .clone(),
        })
    }

    fn create_async_from_sync_iterator(self, ctx: &Ctx<'js>) -> Result<Self> {
        let sync_iterable = Object::new(ctx.clone())?;
        sync_iterable.set(
            Symbol::iterator(ctx.clone()),
            Function::new(ctx.clone(), {
                let iterator = self.iterator.clone();
                move || iterator.clone()
            }),
        )?;

        let async_iterator: Object<'js> = self.sync_to_async_iterator.call((sync_iterable,))?;

        let next_method = async_iterator.get(PredefinedAtom::Next)?;

        Ok(Self {
            iterator: async_iterator,
            next_method,
            done: AtomicBool::new(false),
            sync_to_async_iterator: self.sync_to_async_iterator,
        })
    }

    pub(super) fn iterator_next(
        &self,
        ctx: &Ctx<'js>,
        value: Option<Value<'js>>,
    ) -> Result<Object<'js>> {
        let result: Result<Value<'js>> = match value {
            // If value is not present, then
            None => {
                // Let result be Completion(Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]])).

                self.next_method.call((This(self.iterator.clone()),))
            },
            // Else,
            Some(value) => {
                // Let result be Completion(Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « value »)).
                self.next_method.call((This(self.iterator.clone()), value))
            },
        };

        let result = match result {
            // If result is a throw completion, then
            Err(Error::Exception) => {
                // Set iteratorRecord.[[Done]] to true.
                self.done.store(true, Ordering::Relaxed);
                // Return ? result.
                return Err(Error::Exception);
            },
            Err(err) => return Err(err),
            // Set result to ! result.
            Ok(result) => result,
        };

        let result = match result.into_object() {
            // If result is not an Object, then
            None => {
                // Set iteratorRecord.[[Done]] to true.
                self.done.store(true, Ordering::Relaxed);
                return Err(Exception::throw_type(
                    ctx,
                    "The iterator.next() method must return an object",
                ));
            },
            Some(result) => result,
        };
        // Return result.
        Ok(result)
    }

    pub(super) fn iterator_complete(iterator_result: &Object<'js>) -> Result<bool> {
        let done: Value<'js> = iterator_result.get(PredefinedAtom::Done)?;
        Ok(match done.type_of() {
            Type::Bool => done.as_bool().unwrap(),
            Type::Undefined => false,
            Type::Null => false,
            Type::Float => match done.as_float().unwrap() {
                0.0 => false,
                val if val.is_nan() => false,
                _ => true,
            },
            Type::Int => !matches!(done.as_int().unwrap(), 0),
            Type::String => !matches!(done.as_string().unwrap().to_string().as_deref(), Ok("")),
            _ => true,
        })
    }

    pub(super) fn iterator_value(iterator_result: &Object<'js>) -> Result<Value<'js>> {
        iterator_result.get(PredefinedAtom::Value)
    }
}

pub(super) struct ReadableStreamAsyncIterator<'js> {
    objects: ReadableStreamClassObjects<
        'js,
        ReadableStreamControllerOwned<'js>,
        ReadableStreamDefaultReaderOwned<'js>,
    >,
    prevent_cancel: bool,
    is_finished: Rc<AtomicBool>,
    ongoing_promise: Option<Promise<'js>>,

    promise_primordials: PromisePrimordials<'js>,
    end_of_iteration: Symbol<'js>,
}

impl<'js> Trace<'js> for ReadableStreamAsyncIterator<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        Trace::<'js>::trace(&self.objects, tracer);
        if let Some(ongoing_promise) = &self.ongoing_promise {
            ongoing_promise.trace(tracer);
        }
        Trace::<'js>::trace(&self.end_of_iteration, tracer);
    }
}

unsafe impl<'js> JsLifetime<'js> for ReadableStreamAsyncIterator<'js> {
    type Changed<'to> = ReadableStreamAsyncIterator<'to>;
}

impl<'js> ReadableStreamAsyncIterator<'js> {
    pub(super) fn new(
        ctx: Ctx<'js>,
        objects: ReadableStreamClassObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
        promise_primordials: PromisePrimordials<'js>,
        prevent_cancel: bool,
    ) -> Result<Class<'js, Self>> {
        let end_of_iteration = IteratorPrimordials::get(&ctx)?.end_of_iteration.clone();

        Class::instance(
            ctx,
            Self {
                objects,
                prevent_cancel,
                is_finished: Rc::new(AtomicBool::new(false)),
                ongoing_promise: None,
                promise_primordials,
                end_of_iteration,
            },
        )
    }
}

// Custom JsClass implementation needed until prototype, function names and function lengths can be influenced in the class derivation macro
impl<'js> JsClass<'js> for ReadableStreamAsyncIterator<'js> {
    const NAME: &'static str = "ReadableStreamAsyncIterator";
    type Mutable = rquickjs::class::Writable;
    fn prototype(ctx: &Ctx<'js>) -> Result<Option<Object<'js>>> {
        use rquickjs::class::impl_::MethodImplementor;
        let proto = Object::new(ctx.clone())?;
        let primordial = IteratorPrimordials::get(ctx)?;
        proto.set_prototype(Some(&primordial.async_iterator_prototype))?;
        let implementor = rquickjs::class::impl_::MethodImpl::<Self>::new();
        implementor.implement(&proto)?;
        let next_fn: Function<'js> = proto.get("next")?;
        // yup, the wpt tests really do check these.
        next_fn.set_name("next")?;
        let return_fn: Function<'js> = proto.get("return")?;
        return_fn.set_name("return")?;
        return_fn.set_length(1)?;
        Ok(Some(proto))
    }
    fn constructor(ctx: &Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        use rquickjs::class::impl_::ConstructorCreator;
        let implementor = rquickjs::class::impl_::ConstructorCreate::<Self>::new();
        (&implementor).create_constructor(ctx)
    }
}
impl<'js> IntoJs<'js> for ReadableStreamAsyncIterator<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let cls = Class::<Self>::instance(ctx.clone(), self)?;
        rquickjs::IntoJs::into_js(cls, ctx)
    }
}

impl<'js> FromJs<'js> for ReadableStreamAsyncIterator<'js>
where
    for<'a> rquickjs::class::impl_::CloneWrapper<'a, Self>:
        rquickjs::class::impl_::CloneTrait<Self>,
{
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        use rquickjs::class::impl_::CloneTrait;
        let value = Class::<Self>::from_js(ctx, value)?;
        let borrow = value.try_borrow()?;
        Ok(rquickjs::class::impl_::CloneWrapper(&*borrow).wrap_clone())
    }
}

#[methods]
impl<'js> ReadableStreamAsyncIterator<'js> {
    fn next(ctx: Ctx<'js>, iterator: This<OwnedBorrowMut<'js, Self>>) -> Result<Promise<'js>> {
        let is_finished = iterator.is_finished.clone();

        let next_steps = move |ctx: Ctx<'js>, iterator: &Self, iterator_class: Class<'js, Self>| {
            if is_finished.load(Ordering::Relaxed) {
                return promise_resolved_with(
                    &ctx,
                    &iterator.promise_primordials,
                    Ok(ReadableStreamReadResult {
                        value: None,
                        done: true,
                    }
                    .into_js(&ctx)?),
                );
            }

            let next_promise = Self::next_steps(&ctx, iterator)?;

            upon_promise(
                ctx,
                next_promise,
                move |ctx, result: std::result::Result<Value<'js>, _>| {
                    let mut iterator = OwnedBorrowMut::from_class(iterator_class);
                    match result {
                        Ok(next) => {
                            iterator.ongoing_promise = None;
                            if next.as_symbol() == Some(&iterator.end_of_iteration) {
                                iterator.is_finished.store(true, Ordering::Relaxed);
                                Ok(ReadableStreamReadResult {
                                    value: None,
                                    done: true,
                                })
                            } else {
                                Ok(ReadableStreamReadResult {
                                    value: Some(next),
                                    done: false,
                                })
                            }
                        },
                        Err(reason) => {
                            iterator.ongoing_promise = None;
                            iterator.is_finished.store(true, Ordering::Relaxed);
                            Err(ctx.throw(reason))
                        },
                    }
                },
            )
        };

        let (iterator_class, mut iterator) = class_from_owned_borrow_mut(iterator.0);
        let ongoing_promise = iterator.ongoing_promise.take();

        let ongoing_promise = match ongoing_promise {
            Some(ongoing_promise) => upon_promise(
                ctx,
                ongoing_promise,
                move |ctx, _: std::result::Result<Value<'js>, _>| {
                    let iterator = OwnedBorrow::from_class(iterator_class.clone());
                    next_steps(ctx, &iterator, iterator_class)
                },
            )?,
            None => next_steps(ctx, &iterator, iterator_class)?,
        };

        Ok(iterator.ongoing_promise.insert(ongoing_promise).clone())
    }

    #[qjs(rename = "return")]
    fn r#return(
        ctx: Ctx<'js>,
        iterator: This<OwnedBorrowMut<'js, Self>>,
        value: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        let is_finished = iterator.is_finished.clone();
        let value = value.0.unwrap_or_undefined(&ctx);

        let return_steps = {
            let value = value.clone();
            move |ctx: Ctx<'js>, iterator: &Self| {
                if is_finished.swap(true, Ordering::Relaxed) {
                    return promise_resolved_with(
                        &ctx,
                        &iterator.promise_primordials,
                        Ok(ReadableStreamReadResult {
                            value: Some(value),
                            done: true,
                        }
                        .into_js(&ctx)?),
                    );
                }

                Self::return_steps(ctx.clone(), iterator, value)
            }
        };

        let (iterator_class, mut iterator) = class_from_owned_borrow_mut(iterator.0);
        let ongoing_promise = iterator.ongoing_promise.take();

        let ongoing_promise = match ongoing_promise {
            Some(ongoing_promise) => upon_promise(
                ctx.clone(),
                ongoing_promise,
                move |ctx, _: std::result::Result<Value<'js>, _>| {
                    let iterator = OwnedBorrow::from_class(iterator_class.clone());
                    return_steps(ctx, &iterator)
                },
            )?,
            None => return_steps(ctx.clone(), &iterator)?,
        };

        iterator.ongoing_promise = Some(ongoing_promise.clone());

        upon_promise_fulfilment(ctx, ongoing_promise, move |_, ()| {
            Ok(ReadableStreamReadResult {
                value: Some(value),
                done: true,
            })
        })
    }
}

impl<'js> ReadableStreamAsyncIterator<'js> {
    // The get the next iteration result steps for a ReadableStream, given stream and iterator, are:
    fn next_steps(ctx: &Ctx<'js>, iterator: &Self) -> Result<Promise<'js>> {
        // Let reader be iterator’s reader.
        let objects = iterator.objects.clone();

        // Let promise be a new promise.
        let promise = ResolveablePromise::new(ctx)?;

        // Let readRequest be a new read request with the following items:
        #[derive(Trace)]
        struct ReadRequest<'js> {
            promise: ResolveablePromise<'js>,
            end_of_iteration: Symbol<'js>,
        }

        impl<'js> ReadableStreamReadRequest<'js> for ReadRequest<'js> {
            fn chunk_steps(
                &self,
                objects: ReadableStreamDefaultReaderObjects<'js>,
                chunk: Value<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                // Resolve promise with chunk.
                self.promise.resolve(chunk)?;
                Ok(objects)
            }

            fn close_steps(
                &self,
                _ctx: &Ctx<'js>,
                mut objects: ReadableStreamDefaultReaderObjects<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                // Perform ! ReadableStreamDefaultReaderRelease(reader).
                objects =
                    ReadableStreamDefaultReader::readable_stream_default_reader_release(objects)?;

                // Resolve promise with end of iteration.
                self.promise.resolve(self.end_of_iteration.clone())?;
                Ok(objects)
            }

            fn error_steps(
                &self,
                mut objects: ReadableStreamDefaultReaderObjects<'js>,
                reason: Value<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                // Perform ! ReadableStreamDefaultReaderRelease(reader).
                objects =
                    ReadableStreamDefaultReader::readable_stream_default_reader_release(objects)?;

                // Reject promise with e.
                self.promise.reject(reason)?;
                Ok(objects)
            }
        }

        let objects = ReadableStreamObjects::from_class(objects);

        // Perform ! ReadableStreamDefaultReaderRead(this, readRequest).
        ReadableStreamDefaultReader::readable_stream_default_reader_read(
            ctx,
            objects,
            ReadRequest {
                promise: promise.clone(),
                end_of_iteration: iterator.end_of_iteration.clone(),
            },
        )?;

        // Return promise.
        Ok(promise.promise)
    }

    // The asynchronous iterator return steps for a ReadableStream, given stream, iterator, and arg, are:
    fn return_steps(ctx: Ctx<'js>, iterator: &Self, arg: Value<'js>) -> Result<Promise<'js>> {
        // Let reader be iterator’s reader.
        let objects = ReadableStreamObjects::from_class(iterator.objects.clone());

        // If iterator’s prevent cancel is false:
        if !iterator.prevent_cancel {
            // Let result be ! ReadableStreamReaderGenericCancel(reader, arg).
            let (result, objects) =
                ReadableStreamGenericReader::readable_stream_reader_generic_cancel(
                    ctx.clone(),
                    objects,
                    arg,
                )?;

            // Perform ! ReadableStreamDefaultReaderRelease(reader).
            ReadableStreamDefaultReader::readable_stream_default_reader_release(objects)?;

            // Return result.
            return Ok(result);
        }

        // Perform ! ReadableStreamDefaultReaderRelease(reader).
        ReadableStreamDefaultReader::readable_stream_default_reader_release(objects)?;

        // Return a promise resolved with undefined.
        Ok(iterator
            .promise_primordials
            .promise_resolved_with_undefined
            .clone())
    }
}

#[derive(Clone, JsLifetime, Trace)]
struct IteratorPrimordials<'js> {
    end_of_iteration: Symbol<'js>,
    sync_to_async_iterator: Function<'js>,
    async_iterator_prototype: Object<'js>,
}

impl<'js> Primordial<'js> for IteratorPrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self>
    where
        Self: Sized,
    {
        let sync_to_async_iterator = ctx.eval::<Function<'js>, _>(
            r#"
            (syncIterable) => (async function* () {
              return yield* syncIterable;
            })()
        "#,
        )?;

        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/AsyncIterator
        // ```js
        // const AsyncIteratorPrototype = Object.getPrototypeOf(
        //   Object.getPrototypeOf(Object.getPrototypeOf((async function* () {})())),
        // );
        // ```
        let async_iterator_prototype = ctx
            .eval::<Object<'js>, _>("(async function* () {})()")?
            .get_prototype()
            .as_ref()
            .and_then(Object::get_prototype)
            .as_ref()
            .and_then(Object::get_prototype)
            .expect("async iterator prototype not found");

        Ok(Self {
            end_of_iteration: Symbol::for_description(ctx, "async iterator end of iteration")?,
            sync_to_async_iterator,
            async_iterator_prototype,
        })
    }
}

// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-getmethod
fn get_method<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    property: impl IntoAtom<'js>,
) -> Result<Option<Function<'js>>> {
    // 1. Let func be ? GetV(V, P).
    let func = get_v(ctx, value, property)?;

    // 2. If func is either undefined or null, return undefined.
    if func.is_undefined() || func.is_null() {
        return Ok(None);
    }

    match func.into_function() {
        // 3. If IsCallable(func) is false, throw a TypeError exception.
        None => Err(Exception::throw_type(ctx, "not a function")),
        // 4. Return func.
        Some(func) => Ok(Some(func)),
    }
}

// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-getv
fn get_v<'js>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
    property: impl IntoAtom<'js>,
) -> Result<Value<'js>> {
    // 1. Let O be ? ToObject(V).
    let o: Object<'js> = to_object(ctx, value)?;

    // 2. Return ? O.[[Get]](P, V).
    o.get(property)
}

// https://tc39.es/ecma262/multipage/abstract-operations.html#sec-toobject
fn to_object<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Object<'js>> {
    let base_primordials = BasePrimordials::get(ctx)?;

    match value.type_of() {
        // Return a new Boolean object whose [[BooleanData]] internal slot is set to argument
        Type::Bool => base_primordials.constructor_bool.construct((value,))?,
        // Return a new Number object whose [[NumberData]] internal slot is set to argument
        Type::Int | Type::Float => base_primordials.constructor_number.construct((value,))?,
        // Return a new String object whose [[StringData]] internal slot is set to argument
        Type::String => base_primordials.constructor_string.construct((value,))?,
        // Return a new Symbol object whose [[SymbolData]] internal slot is set to argument
        // `new Symbol` is invalid but we can use `Object(symbol)
        Type::Symbol => base_primordials.constructor_object.call((value,))?,
        // Return a new BigInt object whose [[BigIntData]] internal slot is set to argument
        // `new BigInt` is invalid but we can use `Object(bigInt)
        Type::BigInt => base_primordials.constructor_object.call((value,))?,
        // Return argument
        typ if typ.interpretable_as(Type::Object) => Ok(value.into_object().unwrap()),
        // Throw a TypeError exception.
        typ => Err(Exception::throw_type(
            ctx,
            &format!("{typ} cannot be converted to an object"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use llrt_test::test_sync_with;
    use rquickjs::BigInt;

    #[tokio::test]
    async fn test_to_object() {
        test_sync_with(|ctx| {
            let good_values: [Value; 7] = [
                Value::new_bool(ctx.clone(), false),
                Value::new_int(ctx.clone(), 123),
                Value::new_float(ctx.clone(), 1.5),
                rquickjs::String::from_str(ctx.clone(), "abc")?.into_value(),
                Symbol::for_description(&ctx, "def")?.into_value(),
                BigInt::from_i64(ctx.clone(), 123456)?.into_value(),
                Object::new(ctx.clone())?.into_value(),
            ];

            for value in good_values {
                to_object(&ctx, value)?;
            }

            let bad_values: [Value; 3] = [
                Value::new_uninitialized(ctx.clone()),
                Value::new_undefined(ctx.clone()),
                Value::new_null(ctx.clone()),
            ];

            for value in bad_values {
                let ty = value.type_of();
                if to_object(&ctx, value).is_ok() {
                    panic!("Values of type {ty} should not be convertible to object")
                }
            }

            Ok(())
        })
        .await;
    }
}
