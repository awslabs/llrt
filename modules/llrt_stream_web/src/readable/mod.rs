use std::{cell::OnceCell, rc::Rc};

use byob_reader::{ReadableStreamReadIntoRequest, ViewBytes};
use controller::{
    ReadableStreamController, ReadableStreamControllerClass, ReadableStreamControllerOwned,
};
use iterator::{IteratorKind, IteratorRecord, ReadableStreamAsyncIterator};
use llrt_abort::AbortSignal;
use llrt_utils::{error_messages::ERROR_MSG_ARRAY_BUFFER_DETACHED, result::ResultExt};
use objects::{ReadableStreamClassObjects, ReadableStreamObjects};
use reader::{
    ReadableStreamReader, ReadableStreamReaderClass, ReadableStreamReaderOwned, UndefinedReader,
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{OwnedBorrowMut, Trace, Tracer},
    function::Constructor,
    prelude::{List, MutFn, OnceFn, Opt, This},
    ArrayBuffer, Class, Ctx, Error, Exception, FromJs, Function, IntoJs, JsLifetime, Object,
    Promise, Result, Type, Value,
};

use super::{
    promise_rejected_with, promise_resolved_with,
    queueing_strategy::{QueuingStrategy, SizeAlgorithm},
    writable::WritableStream,
    writable::WritableStreamDefaultWriter,
    Null, ObjectExt, ReadableWritablePair, Undefined,
};

mod byob_reader;
mod byte_controller;
mod controller;
mod default_controller;
mod default_reader;
mod iterator;
mod objects;
mod pipe;
mod reader;
mod tee;

pub(crate) use byob_reader::ReadableStreamBYOBReader;
pub(crate) use byte_controller::{ReadableByteStreamController, ReadableStreamBYOBRequest};
pub(crate) use default_controller::ReadableStreamDefaultController;
pub(crate) use default_reader::ReadableStreamDefaultReader;

use crate::readable::byob_reader::ReadableStreamBYOBReaderOwned;
use crate::readable::byte_controller::{
    ReadableByteStreamControllerClass, ReadableByteStreamControllerOwned,
};
use crate::readable::default_controller::{
    ReadableStreamDefaultControllerClass, ReadableStreamDefaultControllerOwned,
};
use crate::readable::default_reader::ReadableStreamDefaultReaderOwned;

#[rquickjs::class]
#[derive(JsLifetime, Trace)]
pub(crate) struct ReadableStream<'js> {
    controller: ReadableStreamControllerClass<'js>,
    disturbed: bool,
    state: ReadableStreamState,
    reader: Option<ReadableStreamReaderClass<'js>>,
    stored_error: Option<Value<'js>>,
}

pub(crate) type ReadableStreamClass<'js> = Class<'js, ReadableStream<'js>>;
pub(crate) type ReadableStreamOwned<'js> = OwnedBorrowMut<'js, ReadableStream<'js>>;

#[derive(Debug, Trace, Clone, Copy, PartialEq, Eq)]
enum ReadableStreamState {
    Readable,
    Closed,
    Errored,
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

        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                // Set stream.[[state]] to "readable".
                state: ReadableStreamState::Readable,
                // Set stream.[[reader]] and stream.[[storedError]] to undefined.
                reader: None,
                stored_error: None,
                // Set stream.[[disturbed]] to false.
                disturbed: false,
                controller: ReadableStreamControllerClass::Uninitialised,
            },
        )?;
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

    // Promise<undefined> cancel(optional any reason);
    fn cancel(
        ctx: Ctx<'js>,
        mut stream: This<OwnedBorrowMut<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        // If ! IsReadableStreamLocked(this) is true, return a promise rejected with a TypeError exception.
        if stream.is_readable_stream_locked() {
            let e: Value =
                ctx.eval(r#"new TypeError("Cannot cancel a stream that already has a reader")"#)?;
            return promise_rejected_with(&ctx, e);
        }
        let controller = ReadableStreamControllerOwned::from_class(stream.controller.clone());
        let reader = stream.reader_mut();

        let objects = ReadableStreamObjects {
            stream: stream.0,
            controller,
            reader,
        };

        let (promise, _) = Self::readable_stream_cancel(
            ctx.clone(),
            objects,
            reason.0.unwrap_or(Value::new_undefined(ctx)),
        )?;
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
        options: Opt<Value<'js>>,
    ) -> Result<ReadableStreamClass<'js>> {
        // If ! IsReadableStreamLocked(this) is true, throw a TypeError exception.
        if stream.is_readable_stream_locked() {
            return Err(Exception::throw_type(
                &ctx,
                "ReadableStream.prototype.pipeThrough cannot be used on a locked ReadableStream",
            ));
        }

        let options = match options.0 {
            Some(options) if !options.is_null() => Some(StreamPipeOptions::from_js(&ctx, options)?),
            Some(_null) => None,
            None => None,
        };

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
        let options = options.unwrap_or_default();

        let signal = match options.signal {
            Some(signal) => match Class::<'js, AbortSignal>::from_js(&ctx, signal) {
                Ok(signal) => Some(signal),
                Err(_) => {
                    return Err(Exception::throw_type(&ctx, "Invalid signal argument"));
                },
            },
            None => None,
        };

        // Let promise be ! ReadableStreamPipeTo(this, transform["writable"], options["preventClose"], options["preventAbort"], options["preventCancel"], signal).
        let promise = ReadableStream::readable_stream_pipe_to(
            ctx.clone(),
            stream.0,
            writable,
            options.prevent_close,
            options.prevent_abort,
            options.prevent_cancel,
            signal,
        )?;

        // Set promise.[[PromiseIsHandled]] to true.
        let () = promise.then()?.call((
            This(promise.clone()),
            Value::new_undefined(ctx.clone()),
            Function::new(ctx, || {}),
        ))?;

        // Return transform["readable"].
        Ok(readable_class)
    }

    // Promise<undefined> pipeTo(WritableStream destination, optional StreamPipeOptions options = {});
    fn pipe_to(
        ctx: Ctx<'js>,
        stream: This<Value<'js>>,
        destination: Value<'js>,
        options: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        let Ok(stream) = ReadableStreamClass::<'js>::from_value(&stream.0) else {
            let e: Value =
                ctx.eval(r#"new TypeError("'pipeTo' called on an object that is not a valid instance of ReadableStream.")"#)?;
            return promise_rejected_with(&ctx, e);
        };

        let Ok(destination) = Class::<WritableStream<'js>>::from_value(&destination) else {
            let e: Value = ctx.eval(
                r#"new TypeError("Failed to execute 'pipeTo' on 'ReadableStream': parameter 1")"#,
            )?;
            return promise_rejected_with(&ctx, e);
        };

        let options = match options.0 {
            Some(options) if !options.is_null() => {
                Some(match StreamPipeOptions::from_js(&ctx, options) {
                    Ok(options) => options,
                    Err(Error::Exception) => {
                        return promise_rejected_with(&ctx, ctx.catch());
                    },
                    Err(err) => return Err(err),
                })
            },
            Some(_null) => None,
            None => None,
        };

        let stream = OwnedBorrowMut::from_class(stream);
        let destination = OwnedBorrowMut::from_class(destination);

        // If ! IsReadableStreamLocked(this) is true, return a promise rejected with a TypeError exception.
        if stream.is_readable_stream_locked() {
            let e: Value =
                ctx.eval(r#"new TypeError("ReadableStream.prototype.pipeTo cannot be used on a locked ReadableStream")"#)?;
            return promise_rejected_with(&ctx, e);
        }

        // If ! IsWritableStreamLocked(destination) is true, return a promise rejected with a TypeError exception.
        if destination.is_writable_stream_locked() {
            let e: Value =
                ctx.eval(r#"new TypeError("ReadableStream.prototype.pipeTo cannot be used on a locked WritableStream")"#)?;
            return promise_rejected_with(&ctx, e);
        }

        // Let signal be options["signal"] if it exists, or undefined otherwise.
        let options = options.unwrap_or_default();

        let signal = match options.signal {
            Some(signal) => match Class::<'js, AbortSignal>::from_js(&ctx, signal) {
                Ok(signal) => Some(signal),
                Err(_) => {
                    let e: Value = ctx.eval(r#"new TypeError("Invalid signal argument")"#)?;
                    return promise_rejected_with(&ctx, e);
                },
            },
            None => None,
        };

        // Return ! ReadableStreamPipeTo(this, destination, options["preventClose"], options["preventAbort"], options["preventCancel"], signal).
        Self::readable_stream_pipe_to(
            ctx,
            stream,
            destination,
            options.prevent_close,
            options.prevent_abort,
            options.prevent_cancel,
            signal,
        )
    }

    // sequence<ReadableStream> tee();
    fn tee(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
    ) -> Result<List<(Class<'js, Self>, Class<'js, Self>)>> {
        let controller = ReadableStreamControllerOwned::from_class(stream.controller.clone());
        // Return ? ReadableStreamTee(this, false).
        Ok(List(Self::readable_stream_tee(
            ctx,
            ReadableStreamObjects {
                stream: stream.0,
                controller,
                reader: UndefinedReader,
            },
            false,
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
            Some(arg) => matches!(arg.get_optional("preventCancel")?, Some(true)),
        };

        let controller = stream.controller.clone();
        let stream = stream.into_inner();

        ReadableStreamAsyncIterator::new(ctx, stream, controller, reader, prevent_cancel)
    }
}

impl<'js> ReadableStream<'js> {
    fn readable_stream_error<C: ReadableStreamController<'js>, R: ReadableStreamReader<'js>>(
        // Let reader be stream.[[reader]].
        mut objects: ReadableStreamObjects<'js, C, R>,
        e: Value<'js>,
    ) -> Result<ReadableStreamObjects<'js, C, R>> {
        // Set stream.[[state]] to "errored".
        objects.stream.state = ReadableStreamState::Errored;
        // Set stream.[[storedError]] to e.
        objects.stream.stored_error = Some(e.clone());

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

    fn readable_stream_get_num_read_requests(reader: &ReadableStreamDefaultReader) -> usize {
        reader.read_requests.len()
    }

    fn readable_stream_get_num_read_into_requests(reader: &ReadableStreamBYOBReader) -> usize {
        reader.read_into_requests.len()
    }

    fn readable_stream_fulfill_read_request<C: ReadableStreamController<'js>>(
        ctx: &Ctx<'js>,
        // Let reader be stream.[[reader]].
        mut objects: ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>,
        chunk: Value<'js>,
        done: bool,
    ) -> Result<ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>> {
        // Let readRequest be reader.[[readRequests]][0].
        let read_request = {
            let read_requests = &mut objects.reader.read_requests;
            // Remove readRequest from reader.[[readRequests]].
            read_requests
                .pop_front()
                .expect("ReadableStreamFulfillReadRequest called with empty readRequests")
        };

        if done {
            // If done is true, perform readRequest’s close steps.
            read_request.close_steps_typed(ctx, objects)
        } else {
            // Otherwise, perform readRequest’s chunk steps, given chunk.
            read_request.chunk_steps_typed(objects, chunk)
        }
    }

    fn readable_stream_fulfill_read_into_request(
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
        chunk: ViewBytes<'js>,
        done: bool,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
    > {
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

    fn readable_stream_close<C: ReadableStreamController<'js>, R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        // Let reader be stream.[[reader]].
        mut objects: ReadableStreamObjects<'js, C, R>,
    ) -> Result<ReadableStreamObjects<'js, C, R>> {
        // Set stream.[[state]] to "closed".
        objects.stream.state = ReadableStreamState::Closed;

        objects.with_reader(
            |mut objects| {
                // Resolve reader.[[closedPromise]] with undefined.
                objects
                    .reader
                    .generic
                    .closed_promise
                    .resolve(Value::new_undefined(ctx.clone()))?;

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
                objects
                    .reader
                    .generic
                    .closed_promise
                    .resolve(Value::new_undefined(ctx.clone()))?;

                Ok(objects)
            },
            // If reader is undefined, return.
            Ok,
        )
    }

    fn is_readable_stream_locked(&self) -> bool {
        // If stream.[[reader]] is undefined, return false.
        if self.reader.is_none() {
            return false;
        }
        // Return true.
        true
    }

    fn readable_stream_add_read_request(
        &mut self,
        reader: &mut ReadableStreamDefaultReader<'js>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) {
        reader.read_requests.push_back(Box::new(read_request));
    }

    fn readable_stream_cancel<C: ReadableStreamController<'js>, R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<'js, C, R>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, ReadableStreamObjects<'js, C, R>)> {
        // Set stream.[[disturbed]] to true.
        objects.stream.disturbed = true;

        match objects.stream.state {
            // If stream.[[state]] is "closed", return a promise resolved with undefined.
            ReadableStreamState::Closed => Ok((
                promise_resolved_with(&ctx, Ok(Value::new_undefined(ctx.clone())))?,
                objects,
            )),
            // If stream.[[state]] is "errored", return a promise rejected with stream.[[storedError]].
            ReadableStreamState::Errored => Ok((
                promise_rejected_with(
                    &ctx,
                    objects
                        .stream
                        .stored_error
                        .clone()
                        .expect("ReadableStream in errored state without a stored error"),
                )?,
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
                let promise = source_cancel_promise.then()?.call((
                    This(source_cancel_promise.clone()),
                    Function::new(ctx.clone(), move || Value::new_undefined(ctx.clone()))?,
                ))?;

                Ok((promise, objects))
            },
        }
    }

    fn readable_stream_add_read_into_request(
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

        // Assert: ! IsNonNegativeNumber(highWaterMark) is true.

        // Let stream be a new ReadableStream.
        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                // Set stream.[[state]] to "readable".
                state: ReadableStreamState::Readable,
                // Set stream.[[reader]] and stream.[[storedError]] to undefined.
                reader: None,
                stored_error: None,
                // Set stream.[[disturbed]] to false.
                disturbed: false,
                controller: ReadableStreamControllerClass::Uninitialised,
            },
        )?;

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

    // CreateReadableByteStream(startAlgorithm, pullAlgorithm, cancelAlgorithm) performs the following steps:
    fn create_readable_byte_stream(
        ctx: Ctx<'js>,
        start_algorithm: StartAlgorithm<'js>,
        pull_algorithm: PullAlgorithm<'js>,
        cancel_algorithm: CancelAlgorithm<'js>,
    ) -> Result<(Class<'js, Self>, ReadableByteStreamControllerClass<'js>)> {
        // Let stream be a new ReadableStream.
        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                // Set stream.[[state]] to "readable".
                state: ReadableStreamState::Readable,
                // Set stream.[[reader]] and stream.[[storedError]] to undefined.
                reader: None,
                stored_error: None,
                // Set stream.[[disturbed]] to false.
                disturbed: false,
                controller: ReadableStreamControllerClass::Uninitialised,
            },
        )?;

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
        let mut iterator_record =
            IteratorRecord::get_iterator(ctx, async_iterable, IteratorKind::Async)?;
        let iterator = iterator_record.iterator.clone();

        // Let startAlgorithm be an algorithm that returns undefined.
        let start_algorithm = StartAlgorithm::ReturnUndefined;

        // Let pullAlgorithm be the following steps:
        let pull_algorithm = {
            let ctx = ctx.clone();
            let stream = stream.clone();
            move |controller: ReadableStreamDefaultControllerClass<'js>| {
                // Let nextResult be IteratorNext(iteratorRecord).
                let next_result: Result<Object<'js>> = iterator_record.iterator_next(&ctx, None);
                let next_promise = match next_result {
                    // If nextResult is an abrupt completion, return a promise rejected with nextResult.[[Value]].
                    Err(Error::Exception) => {
                        return promise_rejected_with(&ctx, ctx.catch());
                    },
                    Err(err) => return Err(err),
                    // Let nextPromise be a promise resolved with nextResult.[[Value]].
                    Ok(next_result) => promise_resolved_with(&ctx, Ok(next_result.into_inner()))?,
                };

                // Return the result of reacting to nextPromise with the following fulfillment steps, given iterResult:
                next_promise.then()?.call((
                This(next_promise.clone()),
                Function::new(ctx.clone(), OnceFn::new({
                    let ctx = ctx.clone();
                    let stream = stream.clone();
                    move |iter_result: Value<'js>| {

                    let iter_result = match iter_result.into_object() {
                        // If Type(iterResult) is not Object, throw a TypeError.
                        None => {
                            let e: Value = ctx
                                .eval(r#"new TypeError("The promise returned by the iterator.next() method must fulfill with an object")"#)?;
                            return Err(ctx.throw(e));
                        }
                        Some(iter_result) => iter_result,
                    };

                    // Let done be ? IteratorComplete(iterResult).
                    let done = IteratorRecord::iterator_complete(&iter_result)?;

                    let stream = OwnedBorrowMut::from_class(stream.get().cloned().expect("ReadableStreamFromIterable pull steps called with uninitialised stream"));
                    let controller = OwnedBorrowMut::from_class(controller);

                    let objects = ReadableStreamObjects {
                        stream,
                        controller,
                        reader: UndefinedReader
                    }.refresh_reader();

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
                }}))?,
            ))
            }
        };

        // Let cancelAlgorithm be the following steps, given reason:
        let cancel_algorithm = {
            let ctx = ctx.clone();
            move |reason: Value<'js>| {
                // Let iterator be iteratorRecord.[[Iterator]].

                // Let returnMethod be GetMethod(iterator, "return").
                let return_method: Function<'js> = match iterator.get(PredefinedAtom::Return) {
                    // If returnMethod is an abrupt completion, return a promise rejected with returnMethod.[[Value]].
                    Err(Error::Exception) => {
                        return promise_rejected_with(&ctx, ctx.catch());
                    },
                    Err(err) => return Err(err),
                    Ok(None) => {
                        // If returnMethod.[[Value]] is undefined, return a promise resolved with undefined.
                        return promise_resolved_with(&ctx, Ok(Value::new_undefined(ctx.clone())));
                    },
                    Ok(Some(return_method)) => return_method,
                };

                // Let returnResult be Call(returnMethod.[[Value]], iterator, « reason »).
                let return_result: Result<Value<'js>> =
                    return_method.call((This(iterator), reason));

                let return_result = match return_result {
                    // If returnResult is an abrupt completion, return a promise rejected with returnResult.[[Value]].
                    Err(Error::Exception) => {
                        return promise_rejected_with(&ctx, ctx.catch());
                    },
                    Err(err) => return Err(err),
                    Ok(return_result) => return_result,
                };

                // Let returnPromise be a promise resolved with returnResult.[[Value]].
                let return_promise = promise_resolved_with(&ctx, Ok(return_result))?;

                // Return the result of reacting to returnPromise with the following fulfillment steps, given iterResult:
                return_promise.then()?.call((
                This(return_promise.clone()),
                Function::new(
                    ctx.clone(),
                    OnceFn::new({
                        let ctx = ctx.clone();
                        move |iter_result: Value<'js>| {
                            // If Type(iterResult) is not Object, throw a TypeError.
                            if !iter_result.is_object() {
                                let e: Value = ctx
                                    .eval(r#"new TypeError("The promise returned by the iterator.next() method must fulfill with an object")"#)?;
                                return Err(ctx.throw(e));
                            }
                            // Return undefined.
                            Ok(Value::new_undefined(ctx))
                        }
                    }),
                ),
            ))
            }
        };

        let objects_class = ReadableStream::create_readable_stream(
            ctx.clone(),
            start_algorithm,
            PullAlgorithm::Function {
                f: Function::new(ctx.clone(), MutFn::new(pull_algorithm))?,
                underlying_source: Null(None),
            },
            CancelAlgorithm::Function {
                f: Function::new(ctx.clone(), OnceFn::new(cancel_algorithm))?,
                underlying_source: Null(None),
            },
            Some(0.0),
            None,
        )?;
        _ = stream.set(objects_class.stream.clone());
        Ok(objects_class.stream)
    }

    fn reader_mut(&mut self) -> Option<ReadableStreamReaderOwned<'js>> {
        self.reader
            .clone()
            .map(ReadableStreamReaderOwned::from_class)
    }
}

#[derive(Default)]
struct UnderlyingSource<'js> {
    // callback UnderlyingSourceStartCallback = any (ReadableStreamController controller);
    start: Option<Function<'js>>,
    // callback UnderlyingSourcePullCallback = Promise<undefined> (ReadableStreamController controller);
    pull: Option<Function<'js>>,
    // callback UnderlyingSourceCancelCallback = Promise<undefined> (optional any reason);
    cancel: Option<Function<'js>>,
    r#type: Option<ReadableStreamType>,
    // [EnforceRange] unsigned long long autoAllocateChunkSize;
    auto_allocate_chunk_size: Option<usize>,
}

impl<'js> UnderlyingSource<'js> {
    fn from_object(obj: Object<'js>) -> Result<Self> {
        let start = obj.get_optional::<_, _>("start")?;
        let pull = obj.get_optional::<_, _>("pull")?;
        let cancel = obj.get_optional::<_, _>("cancel")?;
        let r#type = obj.get_optional::<_, _>("type")?;
        let auto_allocate_chunk_size = obj.get_optional::<_, _>("autoAllocateChunkSize")?;

        Ok(Self {
            start,
            pull,
            cancel,
            r#type,
            auto_allocate_chunk_size,
        })
    }
}

// enum ReadableStreamType { "bytes" };
enum ReadableStreamType {
    Bytes,
}

impl<'js> FromJs<'js> for ReadableStreamType {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let typ = value.type_of();
        let str = match typ {
            Type::String => value.into_string().unwrap(),
            Type::Object => {
                if let Some(to_string) = value
                    .get_optional::<_, Value>("toString")?
                    .and_then(|s| s.into_function())
                {
                    to_string.call(())?
                } else if let Some(value_of) = value
                    .get_optional::<_, Value>("valueOf")?
                    .and_then(|s| s.into_function())
                {
                    value_of.call(())?
                } else {
                    return Err(Error::new_from_js("Object", "String"));
                }
            },
            typ => return Err(Error::new_from_js(typ.as_str(), "String")),
        };

        match str.to_string()?.as_str() {
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

        let mode = obj.get_optional::<_, ReadableStreamReaderMode>("mode")?;

        Ok(Self { mode })
    }
}

// enum ReadableStreamReaderMode { "byob" };
enum ReadableStreamReaderMode {
    Byob,
}

impl<'js> FromJs<'js> for ReadableStreamReaderMode {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let typ = value.type_of();
        let mode = match typ {
            Type::String => value.into_string().unwrap(),
            Type::Object => {
                if let Some(to_string) = value
                    .get_optional::<_, Value>("toString")?
                    .and_then(|s| s.into_function())
                {
                    to_string.call(())?
                } else if let Some(value_of) = value
                    .get_optional::<_, Value>("valueOf")?
                    .and_then(|s| s.into_function())
                {
                    value_of.call(())?
                } else {
                    return Err(Error::new_from_js("Object", "String"));
                }
            },
            typ => return Err(Error::new_from_js(typ.as_str(), "String")),
        };

        match mode.to_string()?.as_str() {
            "byob" => Ok(Self::Byob),
            _ => Err(Error::new_from_js(typ.as_str(), "ReadableStreamReaderMode")),
        }
    }
}

#[derive(Default)]
struct StreamPipeOptions<'js> {
    prevent_close: bool,
    prevent_abort: bool,
    prevent_cancel: bool,
    signal: Option<Value<'js>>,
}

impl<'js> FromJs<'js> for StreamPipeOptions<'js> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let get_bool = |key| {
            obj.get_optional::<_, Value<'js>>(key)?
                .filter(|value| !value.is_undefined() && !value.is_null())
                .map(|value| {
                    if let Some(bool) = value.as_bool() {
                        return Ok(bool);
                    }

                    // call the Boolean constructor to determine falsiness
                    let bool_object: Object<'js> = ctx
                        .globals()
                        .get::<_, Constructor<'js>>(PredefinedAtom::Boolean)?
                        .construct((value,))?;

                    bool_object
                        .get::<_, Function<'js>>("valueOf")?
                        .call((This(bool_object),))
                })
                .unwrap_or(Ok(false)) // undefined or null or missing all treated as false
        };

        let prevent_abort = get_bool("preventAbort")?;
        let prevent_close = get_bool("preventClose")?;
        let prevent_cancel = get_bool("preventCancel")?;

        let signal = obj.get_optional::<_, Value<'js>>("signal")?;

        Ok(Self {
            prevent_close,
            prevent_abort,
            prevent_cancel,
            signal,
        })
    }
}

fn transfer_array_buffer(buffer: ArrayBuffer<'_>) -> Result<ArrayBuffer<'_>> {
    buffer.get::<_, Function>("transfer")?.call((This(buffer),))
}

fn copy_data_block_bytes(
    ctx: &Ctx<'_>,
    to_block: &ArrayBuffer,
    to_index: usize,
    from_block: &ArrayBuffer,
    from_index: usize,
    count: usize,
) -> Result<()> {
    let to_raw = to_block
        .as_raw()
        .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
        .or_throw(ctx)?;
    let to_slice = unsafe { std::slice::from_raw_parts_mut(to_raw.ptr.as_ptr(), to_raw.len) };
    let from_raw = from_block
        .as_raw()
        .ok_or(ERROR_MSG_ARRAY_BUFFER_DETACHED)
        .or_throw(ctx)?;
    let from_slice = unsafe { std::slice::from_raw_parts(from_raw.ptr.as_ptr(), from_raw.len) };

    to_slice[to_index..to_index + count]
        .copy_from_slice(&from_slice[from_index..from_index + count]);
    Ok(())
}

#[derive(Clone)]
enum StartAlgorithm<'js> {
    ReturnUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> StartAlgorithm<'js> {
    fn call(
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

#[derive(JsLifetime, Trace, Clone)]
enum PullAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> PullAlgorithm<'js> {
    fn call(
        &self,
        ctx: Ctx<'js>,
        controller: ReadableStreamControllerClass<'js>,
    ) -> Result<Promise<'js>> {
        match self {
            PullAlgorithm::ReturnPromiseUndefined => Ok(promise_resolved_with(
                &ctx,
                Ok(Value::new_undefined(ctx.clone())),
            )?),
            PullAlgorithm::Function {
                f,
                underlying_source,
            } => promise_resolved_with(
                &ctx,
                f.call::<_, Value>((This(underlying_source.clone()), controller)),
            ),
        }
    }
}

#[derive(JsLifetime, Trace, Clone)]
enum CancelAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> CancelAlgorithm<'js> {
    fn call(&self, ctx: Ctx<'js>, reason: Value<'js>) -> Result<Promise<'js>> {
        match self {
            CancelAlgorithm::ReturnPromiseUndefined => Ok(promise_resolved_with(
                &ctx,
                Ok(Value::new_undefined(ctx.clone())),
            )?),
            CancelAlgorithm::Function {
                f,
                underlying_source,
            } => {
                let result: Result<Value> = f.call((This(underlying_source.clone()), reason));
                let promise = promise_resolved_with(&ctx, result);
                promise
            },
        }
    }
}

trait ReadableStreamReadRequest<'js>: Trace<'js> {
    fn chunk_steps_typed<C: ReadableStreamController<'js>>(
        &self,
        objects: ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>,
        chunk: Value<'js>,
    ) -> Result<ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>>
    where
        Self: Sized,
    {
        let mut erased = ReadableStreamObjects {
            stream: objects.stream,
            controller: objects.controller.into_erased(),
            reader: objects.reader,
        };

        erased = self.chunk_steps(erased, chunk)?;

        Ok(ReadableStreamObjects {
            stream: erased.stream,
            controller: C::try_from_erased(erased.controller)
                .expect("chunk steps must not change type of controller"),
            reader: erased.reader,
        })
    }

    fn chunk_steps(
        &self,
        objects: ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
        chunk: Value<'js>,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    >;

    fn close_steps_typed<C: ReadableStreamController<'js>>(
        &self,
        ctx: &Ctx<'js>,
        objects: ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>,
    ) -> Result<ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>>
    where
        Self: Sized,
    {
        let mut erased = ReadableStreamObjects {
            stream: objects.stream,
            controller: objects.controller.into_erased(),
            reader: objects.reader,
        };

        erased = self.close_steps(ctx, erased)?;

        Ok(ReadableStreamObjects {
            stream: erased.stream,
            controller: C::try_from_erased(erased.controller)
                .expect("close steps must not change type of controller"),
            reader: erased.reader,
        })
    }

    fn close_steps(
        &self,
        ctx: &Ctx<'js>,
        objects: ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    >;

    fn error_steps_typed<C: ReadableStreamController<'js>>(
        &self,
        objects: ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>,
        reason: Value<'js>,
    ) -> Result<ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>>
    where
        Self: Sized,
    {
        let mut erased = ReadableStreamObjects {
            stream: objects.stream,
            controller: objects.controller.into_erased(),
            reader: objects.reader,
        };

        erased = self.error_steps(erased, reason)?;

        Ok(ReadableStreamObjects {
            stream: erased.stream,
            controller: C::try_from_erased(erased.controller)
                .expect("error steps must not change type of controller"),
            reader: erased.reader,
        })
    }

    fn error_steps(
        &self,
        objects: ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
        reason: Value<'js>,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    >;
}

impl<'js> Trace<'js> for Box<dyn ReadableStreamReadRequest<'js> + 'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.as_ref().trace(tracer);
    }
}

impl<'js> ReadableStreamReadRequest<'js> for Box<dyn ReadableStreamReadRequest<'js> + 'js> {
    fn chunk_steps(
        &self,
        objects: ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
        chunk: Value<'js>,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    > {
        self.as_ref().chunk_steps(objects, chunk)
    }

    fn close_steps(
        &self,
        ctx: &Ctx<'js>,
        objects: ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    > {
        self.as_ref().close_steps(ctx, objects)
    }

    fn error_steps(
        &self,
        objects: ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
        reason: Value<'js>,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableStreamControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    > {
        self.as_ref().error_steps(objects, reason)
    }
}

struct ReadableStreamReadResult<'js> {
    value: Option<Value<'js>>,
    done: bool,
}

impl<'js> IntoJs<'js> for ReadableStreamReadResult<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let obj = Object::new(ctx.clone())?;
        obj.set("value", self.value)?;
        obj.set("done", self.done)?;
        Ok(obj.into_value())
    }
}
