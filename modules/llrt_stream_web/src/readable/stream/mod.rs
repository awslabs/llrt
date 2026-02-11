use std::{cell::OnceCell, panic, rc::Rc};

use crate::{
    queuing_strategy::{QueuingStrategy, SizeAlgorithm},
    readable::{
        byob_reader::{ReadableStreamBYOBReader, ReadableStreamReadIntoRequest, ViewBytes},
        byte_controller::{ReadableByteStreamController, ReadableByteStreamControllerClass},
        controller::{ReadableStreamController, ReadableStreamControllerClass},
        default_controller::{
            ReadableStreamDefaultController, ReadableStreamDefaultControllerOwned,
        },
        default_reader::{ReadableStreamDefaultReader, ReadableStreamReadRequest},
        iterator::{IteratorKind, IteratorRecord, ReadableStreamAsyncIterator},
        objects::{
            ReadableStreamBYOBObjects, ReadableStreamClassObjects,
            ReadableStreamDefaultReaderObjects, ReadableStreamObjects,
        },
        reader::{
            ReadableStreamReader, ReadableStreamReaderClass, ReadableStreamReaderOwned,
            UndefinedReader,
        },
    },
    readable_writable_pair::ReadableWritablePair,
    utils::{
        promise::{
            promise_rejected_catch, promise_rejected_with, promise_rejected_with_constructor,
            promise_resolved_with, upon_promise_fulfilment, with_promise_result,
            PromisePrimordials,
        },
        UnwrapOrUndefined, ValueOrUndefined,
    },
    writable::WritableStreamOwned,
};

use pipe::StreamPipeOptions;
use source::UnderlyingSource;

pub use algorithms::{CancelAlgorithm, PullAlgorithm, StartAlgorithm};
use llrt_utils::{
    option::{Null, NullableOpt, Undefined},
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{OwnedBorrowMut, Trace},
    function::Constructor,
    prelude::{List, Opt, This},
    Class, Coerced, Ctx, Error, Exception, FromJs, Function, IntoJs, JsLifetime, Object, Promise,
    Result, Value,
};

pub mod algorithms;
mod pipe;
pub(super) mod source;
mod tee;

/// Tee a ReadableStream into two branches. The stream must not be locked.
pub fn tee_readable_stream<'js>(
    ctx: Ctx<'js>,
    stream: Class<'js, ReadableStream<'js>>,
) -> Result<(
    Class<'js, ReadableStream<'js>>,
    Class<'js, ReadableStream<'js>>,
)> {
    let owned = OwnedBorrowMut::from_class(stream);
    let objects = ReadableStreamObjects::from_stream(owned);
    ReadableStream::readable_stream_tee(ctx, objects)
}

#[rquickjs::class]
#[derive(JsLifetime)]
pub struct ReadableStream<'js> {
    pub controller: ReadableStreamControllerClass<'js>,
    pub disturbed: bool,
    pub state: ReadableStreamState<'js>,
    pub(crate) reader: Option<ReadableStreamReaderClass<'js>>,
    pub(crate) promise_primordials: PromisePrimordials<'js>,
    pub(crate) constructor_type_error: Constructor<'js>,
    pub(crate) constructor_range_error: Constructor<'js>,
    pub(crate) function_array_buffer_is_view: Function<'js>,
}

impl<'js> Trace<'js> for ReadableStream<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        self.controller.trace(tracer);
        self.state.trace(tracer);
        self.reader.trace(tracer);

        self.promise_primordials.trace(tracer);
        self.constructor_type_error.trace(tracer);
        self.constructor_range_error.trace(tracer);
        self.function_array_buffer_is_view.trace(tracer);
    }
}

pub(crate) type ReadableStreamClass<'js> = Class<'js, ReadableStream<'js>>;
pub(crate) type ReadableStreamOwned<'js> = OwnedBorrowMut<'js, ReadableStream<'js>>;

#[derive(Debug, Trace, Clone, JsLifetime)]
pub enum ReadableStreamState<'js> {
    Readable,
    Closed,
    Errored(Value<'js>),
}

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> ReadableStream<'js> {
    // Streams Spec: 4.2.4: https://streams.spec.whatwg.org/#rs-prototype
    // constructor(optional object underlyingSource, optional QueuingStrategy strategy = {});
    #[qjs(constructor)]
    fn new(
        ctx: Ctx<'js>,
        underlying_source: Opt<Undefined<Object<'js>>>,
        queuing_strategy: Opt<Undefined<QueuingStrategy<'js>>>,
    ) -> Result<Class<'js, Self>> {
        // If underlyingSource is missing, set it to null.
        let underlying_source = Null(underlying_source.0);

        // Let underlyingSourceDict be underlyingSource, converted to an IDL value of type UnderlyingSource.
        let underlying_source_dict = match underlying_source {
            Null(None) | Null(Some(Undefined(None))) => UnderlyingSource::default(),
            Null(Some(Undefined(Some(ref obj)))) => UnderlyingSource::from_object(obj.clone())?,
        };

        let promise_primordials = PromisePrimordials::get(&ctx)?.clone();
        let base_primordials = BasePrimordials::get(&ctx)?;

        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                // Set stream.[[state]] to "readable".
                state: ReadableStreamState::Readable,
                // Set stream.[[reader]] and stream.[[storedError]] to undefined.
                reader: None,
                // Set stream.[[disturbed]] to false.
                disturbed: false,
                controller: ReadableStreamControllerClass::Uninitialised,
                constructor_type_error: base_primordials.constructor_type_error.clone(),
                constructor_range_error: base_primordials.constructor_range_error.clone(),
                function_array_buffer_is_view: base_primordials
                    .function_array_buffer_is_view
                    .clone(),
                promise_primordials,
            },
        )?;
        drop(base_primordials);
        let stream = OwnedBorrowMut::from_class(stream_class.clone());
        let queuing_strategy = queuing_strategy.0.and_then(|qs| qs.0);

        match underlying_source_dict.r#type {
            // If underlyingSourceDict["type"] is "bytes":
            Some(ReadableStreamType::Bytes) => {
                // If strategy["size"] exists, throw a RangeError exception.
                if queuing_strategy
                    .as_ref()
                    .and_then(|qs| qs.size.as_ref())
                    .is_some()
                {
                    return Err(Exception::throw_range(
                        &ctx,
                        "The strategy for a byte stream cannot have a size function",
                    ));
                }
                // Let highWaterMark be ? ExtractHighWaterMark(strategy, 0).
                let high_water_mark =
                    QueuingStrategy::extract_high_water_mark(&ctx, queuing_strategy, 0.0)?;

                // Perform ? SetUpReadableByteStreamControllerFromUnderlyingSource(this, underlyingSource, underlyingSourceDict, highWaterMark).
                ReadableByteStreamController::set_up_readable_byte_stream_controller_from_underlying_source(
                    &ctx,
                    stream,
                    underlying_source,
                    underlying_source_dict,
                    high_water_mark,
                )?;
            },
            // Otherwise,
            None => {
                // Let sizeAlgorithm be ! ExtractSizeAlgorithm(strategy).
                let size_algorithm =
                    QueuingStrategy::extract_size_algorithm(queuing_strategy.as_ref());

                // Let highWaterMark be ? ExtractHighWaterMark(strategy, 1).
                let high_water_mark =
                    QueuingStrategy::extract_high_water_mark(&ctx, queuing_strategy, 1.0)?;

                // Perform ? SetUpReadableStreamDefaultControllerFromUnderlyingSource(this, underlyingSource, underlyingSourceDict, highWaterMark, sizeAlgorithm).
                ReadableStreamDefaultController::set_up_readable_stream_default_controller_from_underlying_source(
                    ctx,
                    stream,
                    underlying_source,
                    underlying_source_dict,
                    high_water_mark,
                    size_algorithm,
                )?;
            },
        }

        Ok(stream_class)
    }

    // static ReadableStream from(any asyncIterable);
    #[qjs(static)]
    fn from(ctx: Ctx<'js>, async_iterable: Value<'js>) -> Result<Class<'js, Self>> {
        // Return ? ReadableStreamFromIterable(asyncIterable).
        Self::readable_stream_from_iterable(&ctx, async_iterable)
    }

    // readonly attribute boolean locked;
    #[qjs(get)]
    fn locked(&self) -> bool {
        // Return ! IsReadableStreamLocked(this).
        self.is_readable_stream_locked()
    }

    // Internal property for checking if stream has been read from
    #[qjs(get)]
    fn disturbed(&self) -> bool {
        self.disturbed
    }

    // Promise<undefined> cancel(optional any reason);
    fn cancel(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        // If ! IsReadableStreamLocked(this) is true, return a promise rejected with a TypeError exception.
        if stream.is_readable_stream_locked() {
            return promise_rejected_with_constructor(
                &stream.constructor_type_error,
                &stream.promise_primordials,
                "Cannot cancel a stream that already has a reader",
            );
        }

        let objects = ReadableStreamObjects::from_stream(stream.0).refresh_reader();

        let (promise, _) =
            Self::readable_stream_cancel(ctx.clone(), objects, reason.0.unwrap_or_undefined(&ctx))?;
        Ok(promise)
    }

    // ReadableStreamReader getReader(optional ReadableStreamGetReaderOptions options = {});
    fn get_reader(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
        options: Opt<Option<ReadableStreamGetReaderOptions>>,
    ) -> Result<ReadableStreamReaderClass<'js>> {
        // If options["mode"] does not exist, return ? AcquireReadableStreamDefaultReader(this).
        let reader = match options.0 {
            None | Some(None | Some(ReadableStreamGetReaderOptions { mode: None })) => {
                let (_, reader) =
                    ReadableStreamReaderClass::acquire_readable_stream_default_reader(
                        ctx.clone(),
                        stream.0,
                    )?;
                reader.into()
            },
            // Return ? AcquireReadableStreamBYOBReader(this).
            Some(Some(ReadableStreamGetReaderOptions {
                mode: Some(ReadableStreamReaderMode::Byob),
            })) => {
                let (_, reader) = ReadableStreamReaderClass::acquire_readable_stream_byob_reader(
                    ctx.clone(),
                    stream.0,
                )?;
                reader.into()
            },
        };

        Ok(reader)
    }

    // ReadableStream pipeThrough(ReadableWritablePair transform, optional StreamPipeOptions options = {});
    fn pipe_through(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
        transform: ReadableWritablePair<'js>,
        options: NullableOpt<StreamPipeOptions<'js>>,
    ) -> Result<ReadableStreamClass<'js>> {
        // If ! IsReadableStreamLocked(this) is true, throw a TypeError exception.
        if stream.is_readable_stream_locked() {
            return Err(Exception::throw_type(
                &ctx,
                "ReadableStream.prototype.pipeThrough cannot be used on a locked ReadableStream",
            ));
        }

        let readable_class = transform.readable.clone();
        let writable = OwnedBorrowMut::from_class(transform.writable);

        // If ! IsWritableStreamLocked(transform["writable"]) is true, throw a TypeError exception.
        if writable.is_writable_stream_locked() {
            return Err(Exception::throw_type(
                &ctx,
                "ReadableStream.prototype.pipeThrough cannot be used on a locked WritableStream",
            ));
        }

        // Let signal be options["signal"] if it exists, or undefined otherwise.
        let options = options.0.unwrap_or_default();

        // Let promise be ! ReadableStreamPipeTo(this, transform["writable"], options["preventClose"], options["preventAbort"], options["preventCancel"], signal).
        let promise = ReadableStream::readable_stream_pipe_to(
            ctx.clone(),
            stream.0,
            writable,
            options.prevent_close,
            options.prevent_abort,
            options.prevent_cancel,
            options.signal,
        )?;

        // Set promise.[[PromiseIsHandled]] to true.
        let () = promise
            .catch()?
            .call((This(promise.clone()), Function::new(ctx, || {})))?;

        // Return transform["readable"].
        Ok(readable_class)
    }

    // Promise<undefined> pipeTo(WritableStream destination, optional StreamPipeOptions options = {});
    fn pipe_to(
        ctx: Ctx<'js>,
        stream: This<Value<'js>>,
        destination: Value<'js>,
        options: NullableOpt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        with_promise_result(&ctx, || {
            let stream =
                ReadableStreamOwned::from_class(Class::from_value(&stream.0).or_throw_type(
                    &ctx,
                    "'pipeTo' called on an object that is not a valid instance of ReadableStream.",
                )?);

            let options = match options.0 {
                Some(options) => Some(StreamPipeOptions::from_js(&ctx, options)?),
                None => None,
            };

            // If ! IsReadableStreamLocked(this) is true, return a promise rejected with a TypeError exception.
            if stream.is_readable_stream_locked() {
                return promise_rejected_with_constructor(
                    &stream.constructor_type_error,
                    &stream.promise_primordials,
                    "ReadableStream.prototype.pipeTo cannot be used on a locked ReadableStream",
                );
            }

            let destination = WritableStreamOwned::from_class(
                Class::from_value(&destination).or_throw_type(&ctx,"'pipeTo' instructed to pipe to an object that is not a valid instance of WritableStream.")?,
            );

            // If ! IsWritableStreamLocked(destination) is true, return a promise rejected with a TypeError exception.
            if destination.is_writable_stream_locked() {
                return promise_rejected_with_constructor(
                    &stream.constructor_type_error,
                    &stream.promise_primordials,
                    "ReadableStream.prototype.pipeTo cannot be used on a locked WritableStream",
                );
            }

            // Let signal be options["signal"] if it exists, or undefined otherwise.
            let options = options.unwrap_or_default();

            // Return ! ReadableStreamPipeTo(this, destination, options["preventClose"], options["preventAbort"], options["preventCancel"], signal).
            Self::readable_stream_pipe_to(
                ctx.clone(),
                stream,
                destination,
                options.prevent_close,
                options.prevent_abort,
                options.prevent_cancel,
                options.signal,
            )
        })
    }

    // sequence<ReadableStream> tee();
    fn tee(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
    ) -> Result<List<(Class<'js, Self>, Class<'js, Self>)>> {
        Ok(List(Self::readable_stream_tee(
            ctx,
            ReadableStreamObjects::from_stream(stream.0),
        )?))
    }

    #[qjs(rename = PredefinedAtom::SymbolAsyncIterator)]
    fn async_iterate(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
    ) -> Result<Class<'js, ReadableStreamAsyncIterator<'js>>> {
        Self::values(ctx, stream, Opt(None))
    }

    fn values(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
        arg: Opt<Object<'js>>,
    ) -> Result<Class<'js, ReadableStreamAsyncIterator<'js>>> {
        // Let reader be ? AcquireReadableStreamDefaultReader(stream).
        let (stream, reader) = ReadableStreamReaderClass::acquire_readable_stream_default_reader(
            ctx.clone(),
            stream.0,
        )?;

        // Let preventCancel be args[0]["preventCancel"].
        let prevent_cancel = match arg.0 {
            None => false,
            Some(arg) => matches!(arg.get_value_or_undefined("preventCancel")?, Some(true)),
        };

        let promise_primordials = stream.promise_primordials.clone();
        let controller = stream.controller.clone();

        ReadableStreamAsyncIterator::new(
            ctx,
            ReadableStreamClassObjects {
                stream: stream.into_inner(),
                controller,
                reader,
            },
            promise_primordials,
            prevent_cancel,
        )
    }
}

impl<'js> ReadableStream<'js> {
    pub(super) fn readable_stream_error<
        C: ReadableStreamController<'js>,
        R: ReadableStreamReader<'js>,
    >(
        // Let reader be stream.[[reader]].
        mut objects: ReadableStreamObjects<'js, C, R>,
        e: Value<'js>,
    ) -> Result<ReadableStreamObjects<'js, C, R>> {
        // Set stream.[[state]] to "errored".
        // Set stream.[[storedError]] to e.
        objects.stream.state = ReadableStreamState::Errored(e.clone());

        objects = objects.with_reader(
            // If reader implements ReadableStreamDefaultReader,
            |mut objects| {
                // Reject reader.[[closedPromise]] with e.
                objects.reader
                    .generic
                    .closed_promise
                    .reject(e.clone())?;

                // Set reader.[[closedPromise]].[[PromiseIsHandled]] to true.
                objects.reader.generic.closed_promise.set_is_handled()?;

                // Perform ! ReadableStreamDefaultReaderErrorReadRequests(reader, e).
                objects = ReadableStreamDefaultReader::readable_stream_default_reader_error_read_requests(
                        objects, e.clone(),
                )?;
                Ok(objects)
        },
            // Otherwise,
            |mut objects| {
                // Reject reader.[[closedPromise]] with e.
                objects.reader
                    .generic
                    .closed_promise
                    .reject(e.clone())?;

                // Set reader.[[closedPromise]].[[PromiseIsHandled]] to true.
                objects.reader.generic.closed_promise.set_is_handled()?;

                // Perform ! ReadableStreamBYOBReaderErrorReadIntoRequests(reader, e).
                objects = ReadableStreamBYOBReader::readable_stream_byob_reader_error_read_into_requests(
                    objects, e.clone(),
                )?;

                Ok(objects)
            },
        // If reader is undefined, return.
        Ok)?;

        Ok(objects)
    }

    pub(super) fn readable_stream_get_num_read_requests(
        reader: &ReadableStreamDefaultReader,
    ) -> usize {
        reader.read_requests.len()
    }

    pub(super) fn readable_stream_get_num_read_into_requests(
        reader: &ReadableStreamBYOBReader,
    ) -> usize {
        reader.read_into_requests.len()
    }

    pub(super) fn readable_stream_fulfill_read_request<C: ReadableStreamController<'js>>(
        ctx: &Ctx<'js>,
        // Let reader be stream.[[reader]].
        mut objects: ReadableStreamDefaultReaderObjects<'js, C>,
        chunk: Value<'js>,
        done: bool,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>> {
        // Let readRequest be reader.[[readRequests]][0].
        // Remove readRequest from reader.[[readRequests]].
        let read_request = objects
            .reader
            .read_requests
            .pop_front()
            .expect("ReadableStreamFulfillReadRequest called with empty readRequests");

        if done {
            // If done is true, perform readRequest’s close steps.
            read_request.close_steps_typed(ctx, objects)
        } else {
            // Otherwise, perform readRequest’s chunk steps, given chunk.
            read_request.chunk_steps_typed(objects, chunk)
        }
    }

    pub(super) fn readable_stream_fulfill_read_into_request(
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamBYOBObjects<'js>,
        chunk: ViewBytes<'js>,
        done: bool,
    ) -> Result<ReadableStreamBYOBObjects<'js>> {
        // Let readIntoRequest be reader.[[readIntoRequests]][0].
        // Remove readIntoRequest from reader.[[readIntoRequests]].
        let read_into_request = objects
            .reader
            .read_into_requests
            .pop_front()
            .expect("ReadableStreamFulfillReadIntoRequest called with empty readIntoRequests");

        if done {
            // If done is true, perform readIntoRequest’s close steps, given chunk.
            read_into_request.close_steps(objects, chunk.into_js(ctx)?)
        } else {
            // Otherwise, perform readIntoRequest’s chunk steps, given chunk.
            read_into_request.chunk_steps(objects, chunk.into_js(ctx)?)
        }
    }

    pub(super) fn readable_stream_close<
        C: ReadableStreamController<'js>,
        R: ReadableStreamReader<'js>,
    >(
        ctx: Ctx<'js>,
        // Let reader be stream.[[reader]].
        mut objects: ReadableStreamObjects<'js, C, R>,
    ) -> Result<ReadableStreamObjects<'js, C, R>> {
        // Set stream.[[state]] to "closed".
        objects.stream.state = ReadableStreamState::Closed;

        objects.with_reader(
            |mut objects| {
                // Resolve reader.[[closedPromise]] with undefined.
                objects.reader.generic.closed_promise.resolve_undefined()?;

                // If reader implements ReadableStreamDefaultReader,
                // Let readRequests be reader.[[readRequests]].
                // Set reader.[[readRequests]] to an empty list.
                let read_requests = objects.reader.read_requests.split_off(0);

                // For each readRequest of readRequests,
                for read_request in read_requests {
                    // Perform readRequest’s close steps.
                    objects = read_request.close_steps_typed(&ctx, objects)?;
                }

                Ok(objects)
            },
            |objects| {
                objects.reader.generic.closed_promise.resolve_undefined()?;

                Ok(objects)
            },
            // If reader is undefined, return.
            Ok,
        )
    }

    pub fn is_readable_stream_locked(&self) -> bool {
        // If stream.[[reader]] is undefined, return false.
        if self.reader.is_none() {
            return false;
        }
        // Return true.
        true
    }

    pub(super) fn readable_stream_add_read_request(
        &mut self,
        reader: &mut ReadableStreamDefaultReader<'js>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) {
        reader.read_requests.push_back(Box::new(read_request));
    }

    pub(super) fn readable_stream_cancel<
        C: ReadableStreamController<'js>,
        R: ReadableStreamReader<'js>,
    >(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<'js, C, R>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, ReadableStreamObjects<'js, C, R>)> {
        // Set stream.[[disturbed]] to true.
        objects.stream.disturbed = true;

        match objects.stream.state {
            // If stream.[[state]] is "closed", return a promise resolved with undefined.
            ReadableStreamState::Closed => Ok((
                // wpt tests expect that this is a new promise every time so we can't duplicate the primordial promise_resolved_with_undefined
                promise_resolved_with(
                    &ctx,
                    &objects.stream.promise_primordials,
                    Ok(Value::new_undefined(ctx.clone())),
                )?,
                objects,
            )),
            // If stream.[[state]] is "errored", return a promise rejected with stream.[[storedError]].
            ReadableStreamState::Errored(ref stored_error) => Ok((
                promise_rejected_with(&objects.stream.promise_primordials, stored_error.clone())?,
                objects,
            )),
            ReadableStreamState::Readable => {
                // Perform ! ReadableStreamClose(stream).
                objects = ReadableStream::readable_stream_close(ctx.clone(), objects)?;
                // Let reader be stream.[[reader]].
                // If reader is not undefined and reader implements ReadableStreamBYOBReader,

                objects = objects.with_reader(
                    Ok,
                    |mut objects| {
                        // Let readIntoRequests be reader.[[readIntoRequests]].
                        // Set reader.[[readIntoRequests]] to an empty list.
                        let read_into_requests = objects.reader.read_into_requests.split_off(0);
                        // For each readIntoRequest of readIntoRequests,
                        for read_into_request in read_into_requests {
                            // Perform readIntoRequest’s close steps, given undefined.
                            objects = read_into_request
                                .close_steps(objects, Value::new_undefined(ctx.clone()))?;
                        }

                        Ok(objects)
                    },
                    Ok,
                )?;

                // Let sourceCancelPromise be ! stream.[[controller]].[[CancelSteps]](reason).
                let (source_cancel_promise, objects) = C::cancel_steps(&ctx, objects, reason)?;

                // Return the result of reacting to sourceCancelPromise with a fulfillment step that returns undefined.
                let promise = upon_promise_fulfilment(ctx, source_cancel_promise, |_, ()| {
                    Ok(rquickjs::Undefined)
                })?;

                Ok((promise, objects))
            },
        }
    }

    pub(super) fn readable_stream_add_read_into_request(
        reader: &mut ReadableStreamBYOBReader<'js>,
        read_request: impl ReadableStreamReadIntoRequest<'js> + 'js,
    ) {
        // Append readRequest to stream.[[reader]].[[readIntoRequests]].
        reader.read_into_requests.push_back(Box::new(read_request))
    }

    // CreateReadableStream(startAlgorithm, pullAlgorithm, cancelAlgorithm[, highWaterMark, [, sizeAlgorithm]]) performs the following steps:
    fn create_readable_stream(
        ctx: Ctx<'js>,
        start_algorithm: StartAlgorithm<'js>,
        pull_algorithm: PullAlgorithm<'js>,
        cancel_algorithm: CancelAlgorithm<'js>,
        high_water_mark: Option<f64>,
        size_algorithm: Option<SizeAlgorithm<'js>>,
    ) -> Result<
        ReadableStreamClassObjects<'js, ReadableStreamDefaultControllerOwned<'js>, UndefinedReader>,
    > {
        // If highWaterMark was not passed, set it to 1.
        let high_water_mark = high_water_mark.unwrap_or(1.0);

        // If sizeAlgorithm was not passed, set it to an algorithm that returns 1.
        let size_algorithm = size_algorithm.unwrap_or(SizeAlgorithm::AlwaysOne);

        let base_primordials = BasePrimordials::get(&ctx)?;

        // Let stream be a new ReadableStream.
        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                // Set stream.[[state]] to "readable".
                state: ReadableStreamState::Readable,
                // Set stream.[[reader]] and stream.[[storedError]] to undefined.
                reader: None,
                // Set stream.[[disturbed]] to false.
                disturbed: false,
                controller: ReadableStreamControllerClass::Uninitialised,
                promise_primordials: PromisePrimordials::get(&ctx)?.clone(),
                constructor_range_error: base_primordials.constructor_range_error.clone(),
                constructor_type_error: base_primordials.constructor_type_error.clone(),
                function_array_buffer_is_view: base_primordials
                    .function_array_buffer_is_view
                    .clone(),
            },
        )?;
        drop(base_primordials);

        // Perform ? SetUpReadableStreamDefaultController(stream, controller, startAlgorithm, pullAlgorithm, cancelAlgorithm, highWaterMark, sizeAlgorithm).
        let controller_class =
            ReadableStreamDefaultController::set_up_readable_stream_default_controller(
                ctx,
                OwnedBorrowMut::from_class(stream_class.clone()),
                start_algorithm,
                pull_algorithm,
                cancel_algorithm,
                high_water_mark,
                size_algorithm,
            )?;

        // Return stream.
        Ok(ReadableStreamClassObjects {
            stream: stream_class,
            controller: controller_class,
            reader: UndefinedReader,
        })
    }

    /// Create a ReadableStream from Rust pull/cancel algorithms
    pub fn from_pull_algorithm(
        ctx: Ctx<'js>,
        pull_algorithm: PullAlgorithm<'js>,
        cancel_algorithm: CancelAlgorithm<'js>,
    ) -> Result<Class<'js, Self>> {
        Ok(Self::create_readable_stream(
            ctx,
            StartAlgorithm::ReturnUndefined,
            pull_algorithm,
            cancel_algorithm,
            None,
            None,
        )?
        .stream)
    }

    // CreateReadableByteStream(startAlgorithm, pullAlgorithm, cancelAlgorithm) performs the following steps:
    pub fn create_readable_byte_stream(
        ctx: Ctx<'js>,
        start_algorithm: StartAlgorithm<'js>,
        pull_algorithm: PullAlgorithm<'js>,
        cancel_algorithm: CancelAlgorithm<'js>,
    ) -> Result<(Class<'js, Self>, ReadableByteStreamControllerClass<'js>)> {
        let base_primordials = BasePrimordials::get(&ctx)?;

        // Let stream be a new ReadableStream.
        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                // Set stream.[[state]] to "readable".
                state: ReadableStreamState::Readable,
                // Set stream.[[reader]] and stream.[[storedError]] to undefined.
                reader: None,
                // Set stream.[[disturbed]] to false.
                disturbed: false,
                controller: ReadableStreamControllerClass::Uninitialised,
                promise_primordials: PromisePrimordials::get(&ctx)?.clone(),
                constructor_type_error: base_primordials.constructor_type_error.clone(),
                constructor_range_error: base_primordials.constructor_range_error.clone(),
                function_array_buffer_is_view: base_primordials
                    .function_array_buffer_is_view
                    .clone(),
            },
        )?;
        drop(base_primordials);

        // Perform ? SetUpReadableStreamDefaultController(stream, controller, startAlgorithm, pullAlgorithm, cancelAlgorithm, highWaterMark, sizeAlgorithm).
        let controller_class =
            ReadableByteStreamController::set_up_readable_byte_stream_controller(
                ctx,
                OwnedBorrowMut::from_class(stream_class.clone()),
                start_algorithm,
                pull_algorithm,
                cancel_algorithm,
                0.0,
                None,
            )?;

        // Return stream.
        Ok((stream_class, controller_class))
    }

    fn readable_stream_from_iterable(
        ctx: &Ctx<'js>,
        async_iterable: Value<'js>,
    ) -> Result<Class<'js, Self>> {
        let stream: Rc<OnceCell<Class<'js, Self>>> = Rc::new(OnceCell::new());

        // Let iteratorRecord be ? GetIterator(asyncIterable, async).
        let iterator_record =
            IteratorRecord::get_iterator(ctx, async_iterable, IteratorKind::Async)?;
        let iterator = iterator_record.iterator.clone();

        // Let startAlgorithm be an algorithm that returns undefined.
        let start_algorithm = StartAlgorithm::ReturnUndefined;

        let promise_primordials = PromisePrimordials::get(ctx)?.clone();

        // Let pullAlgorithm be the following steps:
        let pull_algorithm = {
            let stream = stream.clone();
            let promise_primordials = promise_primordials.clone();
            move |ctx: Ctx<'js>, controller: ReadableStreamControllerClass<'js>| {
                // Let nextResult be IteratorNext(iteratorRecord).
                let next_result: Result<Object<'js>> = iterator_record.iterator_next(&ctx, None);
                let next_promise = match next_result {
                    // If nextResult is an abrupt completion, return a promise rejected with nextResult.[[Value]].
                    Err(Error::Exception) => {
                        return promise_rejected_catch(&ctx, &promise_primordials);
                    },
                    Err(err) => return Err(err),
                    // Let nextPromise be a promise resolved with nextResult.[[Value]].
                    Ok(next_result) => promise_resolved_with(
                        &ctx,
                        &promise_primordials,
                        Ok(next_result.into_inner()),
                    )?,
                };

                // Return the result of reacting to nextPromise with the following fulfillment steps, given iterResult:
                upon_promise_fulfilment(ctx, next_promise, {
                    let stream = stream.clone();
                    move |ctx, iter_result: Value<'js>| {
                        let iter_result = match iter_result.into_object() {
                            // If Type(iterResult) is not Object, throw a TypeError.
                            None => {
                                return Err(Exception::throw_type(&ctx, "The promise returned by the iterator.next() method must fulfill with an object"));
                            },
                            Some(iter_result) => iter_result,
                        };

                        // Let done be ? IteratorComplete(iterResult).
                        let done = IteratorRecord::iterator_complete(&iter_result)?;

                        let stream = OwnedBorrowMut::from_class(stream.get().cloned().expect("ReadableStreamFromIterable pull steps called with uninitialised stream"));
                        let controller = match controller {
                        ReadableStreamControllerClass::ReadableStreamDefaultController(c) => OwnedBorrowMut::from_class(c),
                        _ => panic!("ReadableStreamFromIterable pull steps called without default controller")
                    };

                        let objects = ReadableStreamObjects::new_default(stream, controller);

                        // If done is true:
                        if done {
                            // Perform ! ReadableStreamDefaultControllerClose(stream.[[controller]]).
                            ReadableStreamDefaultController::readable_stream_default_controller_close(ctx.clone(), objects)?;
                        } else {
                            // Let value be ? IteratorValue(iterResult).
                            let value = IteratorRecord::iterator_value(&iter_result)?;

                            // Perform ! ReadableStreamDefaultControllerEnqueue(stream.[[controller]], value).
                            ReadableStreamDefaultController::readable_stream_default_controller_enqueue(ctx.clone(), objects, value)?;
                        }

                        Ok(())
                    }
                })
            }
        };

        // Let cancelAlgorithm be the following steps, given reason:
        let cancel_algorithm = {
            let ctx = ctx.clone();
            let promise_primordials = promise_primordials.clone();
            move |reason: Value<'js>| {
                // Let iterator be iteratorRecord.[[Iterator]].

                // Let returnMethod be GetMethod(iterator, "return").
                let return_method: Function<'js> = match iterator.get(PredefinedAtom::Return) {
                    // If returnMethod is an abrupt completion, return a promise rejected with returnMethod.[[Value]].
                    Err(Error::Exception) => {
                        return promise_rejected_catch(&ctx, &promise_primordials);
                    },
                    Err(err) => return Err(err),
                    Ok(None) => {
                        // If returnMethod.[[Value]] is undefined, return a promise resolved with undefined.
                        return Ok(promise_primordials.promise_resolved_with_undefined.clone());
                    },
                    Ok(Some(return_method)) => return_method,
                };

                // Let returnResult be Call(returnMethod.[[Value]], iterator, « reason »).
                let return_result: Result<Value<'js>> =
                    return_method.call((This(iterator), reason));

                let return_result = match return_result {
                    // If returnResult is an abrupt completion, return a promise rejected with returnResult.[[Value]].
                    Err(Error::Exception) => {
                        return promise_rejected_catch(&ctx, &promise_primordials);
                    },
                    Err(err) => return Err(err),
                    Ok(return_result) => return_result,
                };

                // Let returnPromise be a promise resolved with returnResult.[[Value]].
                let return_promise =
                    promise_resolved_with(&ctx, &promise_primordials, Ok(return_result))?;

                // Return the result of reacting to returnPromise with the following fulfillment steps, given iterResult:
                upon_promise_fulfilment(
                    ctx,
                    return_promise,
                    move |ctx: Ctx<'js>, iter_result: Value<'js>| {
                        // If Type(iterResult) is not Object, throw a TypeError.
                        if !iter_result.is_object() {
                            return Err(Exception::throw_type(&ctx, "The promise returned by the iterator.next() method must fulfill with an object"));
                        }
                        // Return undefined.
                        Ok(rquickjs::Undefined)
                    },
                )
            }
        };

        let objects_class = ReadableStream::create_readable_stream(
            ctx.clone(),
            start_algorithm,
            PullAlgorithm::from_fn(pull_algorithm),
            CancelAlgorithm::from_fn(cancel_algorithm),
            Some(0.0),
            None,
        )?;
        _ = stream.set(objects_class.stream.clone());
        Ok(objects_class.stream)
    }

    pub(super) fn reader_mut(&mut self) -> Option<ReadableStreamReaderOwned<'js>> {
        self.reader
            .clone()
            .map(ReadableStreamReaderOwned::from_class)
    }
}

// enum ReadableStreamType { "bytes" };
enum ReadableStreamType {
    Bytes,
}

impl<'js> FromJs<'js> for ReadableStreamType {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let typ = value.type_of();

        match Coerced::<String>::from_js(ctx, value)?.as_str() {
            "bytes" => Ok(Self::Bytes),
            _ => Err(Error::new_from_js(typ.as_str(), "ReadableStreamType")),
        }
    }
}

struct ReadableStreamGetReaderOptions {
    mode: Option<ReadableStreamReaderMode>,
}

impl<'js> FromJs<'js> for ReadableStreamGetReaderOptions {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let mode = obj.get_value_or_undefined::<_, ReadableStreamReaderMode>("mode")?;

        Ok(Self { mode })
    }
}

// enum ReadableStreamReaderMode { "byob" };
enum ReadableStreamReaderMode {
    Byob,
}

impl<'js> FromJs<'js> for ReadableStreamReaderMode {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let typ = value.type_of();

        match Coerced::<String>::from_js(ctx, value)?.as_str() {
            "byob" => Ok(Self::Byob),
            _ => Err(Error::new_from_js(typ.as_str(), "ReadableStreamReaderMode")),
        }
    }
}
