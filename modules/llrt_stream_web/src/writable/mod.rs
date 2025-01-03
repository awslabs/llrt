use std::collections::VecDeque;

pub(crate) use default_controller::WritableStreamDefaultController;
use default_controller::WritableStreamDefaultControllerClass;
use default_writer::WritableStreamDefaultWriterClass;
pub(crate) use default_writer::{WritableStreamDefaultWriter, WritableStreamDefaultWriterOwned};
use llrt_abort::AbortController;
pub(crate) use objects::{WritableStreamClassObjects, WritableStreamObjects};
use rquickjs::{
    class::{OwnedBorrowMut, Trace},
    prelude::{Opt, This},
    Class, Ctx, Exception, Function, JsLifetime, Object, Promise, Result, Value,
};
use writer::{UndefinedWriter, WritableStreamWriter};

use super::{
    promise_rejected_with, promise_resolved_with, upon_promise, Null, ObjectExt, Undefined,
};
use crate::{new_type_error, queueing_strategy::QueuingStrategy, ResolveablePromise};

mod default_controller;
mod default_writer;
mod objects;
mod writer;

#[rquickjs::class]
#[derive(JsLifetime, Trace)]
pub struct WritableStream<'js> {
    backpressure: bool,
    close_request: Option<ResolveablePromise<'js>>,
    pub(crate) controller: Option<WritableStreamDefaultControllerClass<'js>>,
    in_flight_write_request: Option<ResolveablePromise<'js>>,
    in_flight_close_request: Option<ResolveablePromise<'js>>,
    pending_abort_request: Option<PendingAbortRequest<'js>>,
    pub(crate) state: WritableStreamState<'js>,
    writer: Option<WritableStreamDefaultWriterClass<'js>>,
    write_requests: VecDeque<ResolveablePromise<'js>>,
}

pub(crate) type WritableStreamClass<'js> = Class<'js, WritableStream<'js>>;
pub(crate) type WritableStreamOwned<'js> = OwnedBorrowMut<'js, WritableStream<'js>>;

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> WritableStream<'js> {
    // constructor(optional object underlyingSink, optional QueuingStrategy strategy = {});
    #[qjs(constructor)]
    fn new(
        ctx: Ctx<'js>,
        underlying_sink: Opt<Undefined<Object<'js>>>,
        queuing_strategy: Opt<Undefined<QueuingStrategy<'js>>>,
    ) -> Result<Class<'js, Self>> {
        // If underlyingSink is missing, set it to null.
        let underlying_sink = Null(underlying_sink.0);

        // Let underlyingSinkDict be underlyingSink, converted to an IDL value of type UnderlyingSink.
        let underlying_sink_dict = match underlying_sink {
            Null(None) | Null(Some(Undefined(None))) => UnderlyingSink::default(),
            Null(Some(Undefined(Some(ref obj)))) => UnderlyingSink::from_object(obj.clone())?,
        };

        // If underlyingSinkDict["type"] exists, throw a RangeError exception.
        if underlying_sink_dict.r#type.is_some() {
            return Err(Exception::throw_range(&ctx, "Invalid type is specified"));
        }

        // Perform ! InitializeWritableStream(this).
        let stream_class = Class::instance(
            ctx.clone(),
            Self {
                // Set stream.[[state]] to "writable".
                state: WritableStreamState::Writable,
                // Set stream.[[storedError]], stream.[[writer]], stream.[[controller]], stream.[[inFlightWriteRequest]], stream.[[closeRequest]], stream.[[inFlightCloseRequest]], and stream.[[pendingAbortRequest]] to undefined.
                writer: None,
                controller: None,
                in_flight_write_request: None,
                close_request: None,
                in_flight_close_request: None,
                pending_abort_request: None,
                // Set stream.[[writeRequests]] to a new empty list.
                write_requests: VecDeque::new(),
                // Set stream.[[backpressure]] to false.
                backpressure: false,
            },
        )?;
        let stream = OwnedBorrowMut::from_class(stream_class.clone());
        let queuing_strategy = queuing_strategy.0.and_then(|qs| qs.0);

        // Let sizeAlgorithm be ! ExtractSizeAlgorithm(strategy).
        let size_algorithm = QueuingStrategy::extract_size_algorithm(queuing_strategy.as_ref());

        // Let highWaterMark be ? ExtractHighWaterMark(strategy, 1).
        let high_water_mark =
            QueuingStrategy::extract_high_water_mark(&ctx, queuing_strategy, 1.0)?;

        // Perform ? SetUpWritableStreamDefaultControllerFromUnderlyingSink(this, underlyingSink, underlyingSinkDict, highWaterMark, sizeAlgorithm).
        WritableStreamDefaultController::set_up_writable_stream_default_controller_from_underlying_sink(ctx, stream, underlying_sink, underlying_sink_dict, high_water_mark, size_algorithm)?;

        Ok(stream_class)
    }

    // readonly attribute boolean locked;
    #[qjs(get)]
    fn locked(&self) -> bool {
        // Return ! IsWritableStreamLocked(this).
        self.is_writable_stream_locked()
    }

    fn abort(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        if stream.is_writable_stream_locked() {
            // If ! IsWritableStreamLocked(this) is true, return a promise rejected with a TypeError exception.
            let e: Value = new_type_error(&ctx, "Cannot abort a stream that already has a writer")?;
            return promise_rejected_with(&ctx, e);
        }

        let controller = OwnedBorrowMut::from_class(
            stream
                .controller
                .clone()
                .expect("Abort called on stream without a controller"),
        );

        let objects = WritableStreamObjects {
            stream: stream.0,
            controller,
            writer: UndefinedWriter,
        };

        // Return ! WritableStreamAbort(this, reason).
        let (promise, _) = Self::writable_stream_abort(ctx.clone(), objects, reason.0)?;

        Ok(promise)
    }

    fn close(ctx: Ctx<'js>, stream: This<OwnedBorrowMut<'js, Self>>) -> Result<Promise<'js>> {
        if stream.is_writable_stream_locked() {
            // If ! IsWritableStreamLocked(this) is true, return a promise rejected with a TypeError exception.
            let e: Value = new_type_error(&ctx, "Cannot close a stream that already has a writer")?;
            return promise_rejected_with(&ctx, e);
        }

        if Self::writable_stream_close_queued_or_in_flight(&stream.0) {
            // If ! WritableStreamCloseQueuedOrInFlight(this) is true, return a promise rejected with a TypeError exception.
            let e: Value = new_type_error(&ctx, "Cannot close an already-closing stream")?;
            return promise_rejected_with(&ctx, e);
        }

        let controller = OwnedBorrowMut::from_class(
            stream
                .controller
                .clone()
                .expect("Abort called on stream without a controller"),
        );

        let objects = WritableStreamObjects {
            stream: stream.0,
            controller,
            writer: UndefinedWriter,
        };

        // Return ! WritableStreamClose(this).
        let (promise, _) = Self::writable_stream_close(ctx.clone(), objects)?;

        Ok(promise)
    }

    fn get_writer(
        ctx: Ctx<'js>,
        stream: This<OwnedBorrowMut<'js, Self>>,
    ) -> Result<WritableStreamDefaultWriterClass<'js>> {
        // Return ? AcquireWritableStreamDefaultWriter(this).
        let (_, writer) =
            WritableStreamDefaultWriter::acquire_writable_stream_default_writer(&ctx, stream.0)?;

        Ok(writer)
    }
}

impl<'js> WritableStream<'js> {
    pub(super) fn is_writable_stream_locked(&self) -> bool {
        if self.writer.is_none() {
            // If stream.[[writer]] is undefined, return false.
            false
        } else {
            // Return true.
            true
        }
    }

    pub(super) fn writable_stream_abort<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        mut objects: WritableStreamObjects<'js, W>,
        mut reason: Option<Value<'js>>,
    ) -> Result<(Promise<'js>, WritableStreamObjects<'js, W>)> {
        // If stream.[[state]] is "closed" or "errored", return a promise resolved with undefined.
        if matches!(
            objects.stream.state,
            WritableStreamState::Closed | WritableStreamState::Errored(_)
        ) {
            return Ok((
                promise_resolved_with(&ctx, Ok(Value::new_undefined(ctx.clone())))?,
                objects,
            ));
        }

        // Signal abort on stream.[[controller]].[[abortController]] with reason.
        {
            // this executes user code, so we should ensure we hold no locks
            let abort_controller = objects.controller.abort_controller.clone();
            let objects_class = objects.into_inner();
            AbortController::abort(ctx.clone(), This(abort_controller), Opt(reason.clone()))?;
            objects = WritableStreamObjects::from_class(objects_class);
        }

        // Let state be stream.[[state]].
        // If state is "closed" or "errored", return a promise resolved with undefined.
        if matches!(
            objects.stream.state,
            WritableStreamState::Closed | WritableStreamState::Errored(_)
        ) {
            return Ok((
                promise_resolved_with(&ctx, Ok(Value::new_undefined(ctx.clone())))?,
                objects,
            ));
        }

        // If stream.[[pendingAbortRequest]] is not undefined, return stream.[[pendingAbortRequest]]'s promise.
        match objects.stream.pending_abort_request {
            None => {},
            Some(ref pending_abort_request) => {
                return Ok((pending_abort_request.promise.promise.clone(), objects))
            },
        }

        let was_already_erroring = match objects.stream.state {
            // If state is "erroring",
            // Set wasAlreadyErroring to true.
            // Set reason to undefined.
            WritableStreamState::Erroring(_) => {
                reason = None;
                true
            },
            // Let wasAlreadyErroring be false.
            _ => false,
        };

        // Let promise be a new promise.
        let promise = ResolveablePromise::new(&ctx)?;

        let reason = reason.unwrap_or(Value::new_undefined(ctx.clone()));

        // Set stream.[[pendingAbortRequest]] to a new pending abort request whose promise is promise, reason is reason, and was already erroring is wasAlreadyErroring.
        objects.stream.pending_abort_request = Some(PendingAbortRequest {
            promise: promise.clone(),
            reason: reason.clone(),
            was_already_erroring,
        });

        // If wasAlreadyErroring is false, perform ! WritableStreamStartErroring(stream, reason).
        if !was_already_erroring {
            objects = Self::writable_stream_start_erroring(ctx, objects, reason)?;
        }

        Ok((promise.promise.clone(), objects))
    }

    fn writable_stream_close<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        mut objects: WritableStreamObjects<'js, W>,
    ) -> Result<(Promise<'js>, WritableStreamObjects<'js, W>)> {
        // Let state be stream.[[state]].
        // If state is "closed" or "errored", return a promise rejected with a TypeError exception.
        if matches!(
            objects.stream.state,
            WritableStreamState::Closed | WritableStreamState::Errored(_)
        ) {
            let e: Value = new_type_error(
                &ctx,
                "The stream is not in the writable state and cannot be closed",
            )?;
            return Ok((promise_rejected_with(&ctx, e)?, objects));
        }

        // Let promise be a new promise.
        let promise = ResolveablePromise::new(&ctx)?;
        // Set stream.[[closeRequest]] to promise.
        objects.stream.close_request = Some(promise.clone());

        // Let writer be stream.[[writer]].
        // If writer is not undefined, and stream.[[backpressure]] is true, and state is "writable", resolve writer.[[readyPromise]] with undefined.
        objects = objects.with_writer(
            |objects| {
                if objects.stream.backpressure
                    && matches!(objects.stream.state, WritableStreamState::Writable)
                {
                    let () = objects
                        .writer
                        .ready_promise
                        .resolve(Value::new_undefined(ctx.clone()))?;
                }
                Ok(objects)
            },
            Ok,
        )?;

        // Perform ! WritableStreamDefaultControllerClose(stream.[[controller]]).
        objects = WritableStreamDefaultController::writable_stream_default_controller_close(
            ctx, objects,
        )?;

        // Return promise.
        Ok((promise.promise.clone(), objects))
    }

    fn writable_stream_start_erroring<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        // Let controller be stream.[[controller]].
        // Let writer be stream.[[writer]].
        mut objects: WritableStreamObjects<'js, W>,
        reason: Value<'js>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // Set stream.[[state]] to "erroring".
        // Set stream.[[storedError]] to reason.
        objects.stream.state = WritableStreamState::Erroring(reason.clone());

        // If writer is not undefined, perform ! WritableStreamDefaultWriterEnsureReadyPromiseRejected(writer, reason).
        objects = objects.with_writer(
            |mut objects| {
                objects
                    .writer
                    .writable_stream_default_writer_ensure_ready_promise_rejected(
                        &ctx,
                        reason.clone(),
                    )?;
                Ok(objects)
            },
            Ok,
        )?;

        // If ! WritableStreamHasOperationMarkedInFlight(stream) is false and controller.[[started]] is true, perform ! WritableStreamFinishErroring(stream).
        if !objects
            .stream
            .writable_stream_has_operation_marked_in_flight()
            && objects.controller.started
        {
            objects = Self::writable_stream_finish_erroring(ctx, objects, reason)?;
        }

        Ok(objects)
    }

    fn writable_stream_finish_erroring<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        mut objects: WritableStreamObjects<'js, W>,
        // Let storedError be stream.[[storedError]].
        stored_error: Value<'js>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // Set stream.[[state]] to "errored".
        objects.stream.state = WritableStreamState::Errored(stored_error.clone());

        // Perform ! stream.[[controller]].[[ErrorSteps]]().
        objects.controller.error_steps();

        // For each writeRequest of stream.[[writeRequests]]:
        for write_request in &mut objects.stream.write_requests {
            let () = write_request.reject(stored_error.clone())?;
        }

        // Set stream.[[writeRequests]] to an empty list.
        objects.stream.write_requests.clear();

        // Let abortRequest be stream.[[pendingAbortRequest]].
        // Set stream.[[pendingAbortRequest]] to undefined.
        let abort_request = if let Some(pending_abort_request) =
            objects.stream.pending_abort_request.take()
        {
            pending_abort_request
        } else {
            // If stream.[[pendingAbortRequest]] is undefined,
            // Perform ! WritableStreamRejectCloseAndClosedPromiseIfNeeded(stream).
            objects =
                WritableStream::writable_stream_reject_close_and_closed_promise_if_needed(objects)?;
            // Return.
            return Ok(objects);
        };

        // If abortRequest’s was already erroring is true,
        if abort_request.was_already_erroring {
            // Reject abortRequest’s promise with storedError.
            let () = abort_request.promise.reject(stored_error.clone())?;

            // Perform ! WritableStreamRejectCloseAndClosedPromiseIfNeeded(stream).
            objects =
                WritableStream::writable_stream_reject_close_and_closed_promise_if_needed(objects)?;

            // Return.
            return Ok(objects);
        }

        // Let promise be ! stream.[[controller]].[[AbortSteps]](abortRequest’s reason).
        let (promise, objects) =
            WritableStreamDefaultController::abort_steps(&ctx, objects, abort_request.reason)?;

        let objects_class = objects.into_inner();

        // Upon fulfillment of promise,
        let _ = upon_promise::<Value<'js>, _>(ctx.clone(), promise, {
            let objects_class = objects_class.clone();
            move |ctx, result| {
                let objects =
                    WritableStreamObjects::from_class_no_writer(objects_class).refresh_writer();
                match result {
                    // Upon fulfillment of promise,
                    Ok(_) => {
                        // Resolve abortRequest’s promise with undefined.
                        let () = abort_request
                            .promise
                            .resolve(Value::new_undefined(ctx.clone()))?;
                        // Perform ! WritableStreamRejectCloseAndClosedPromiseIfNeeded(stream).
                        WritableStream::writable_stream_reject_close_and_closed_promise_if_needed(
                            objects,
                        )?;
                        Ok(())
                    },
                    // Upon rejection of promise with reason reason,
                    Err(reason) => {
                        // Reject abortRequest’s promise with reason.
                        let () = abort_request.promise.reject(reason)?;
                        // Perform ! WritableStreamRejectCloseAndClosedPromiseIfNeeded(stream).
                        WritableStream::writable_stream_reject_close_and_closed_promise_if_needed(
                            objects,
                        )?;
                        Ok(())
                    },
                }
            }
        })?;

        Ok(WritableStreamObjects::from_class(objects_class))
    }

    fn writable_stream_reject_close_and_closed_promise_if_needed<W: WritableStreamWriter<'js>>(
        // Let writer be stream.[[writer]].
        mut objects: WritableStreamObjects<'js, W>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // If stream.[[closeRequest]] is not undefined,
        if let Some(ref close_request) = objects.stream.close_request {
            // Reject stream.[[closeRequest]] with stream.[[storedError]].
            let () = close_request.reject(objects.stream.stored_error())?;
            // Set stream.[[closeRequest]] to undefined.
            objects.stream.close_request = None;
        }

        // If writer is not undefined,
        objects.with_writer(
            |objects| {
                // Reject writer.[[closedPromise]] with stream.[[storedError]].
                let () = objects
                    .writer
                    .closed_promise
                    .reject(objects.stream.stored_error())?;

                // Set writer.[[closedPromise]].[[PromiseIsHandled]] to true.
                objects.writer.closed_promise.set_is_handled()?;

                Ok(objects)
            },
            Ok,
        )
    }

    fn writable_stream_mark_first_write_request_in_flight(&mut self) {
        // Let writeRequest be stream.[[writeRequests]][0].
        // Remove writeRequest from stream.[[writeRequests]].
        let write_request = self.write_requests.pop_front().expect("writable_stream_mark_first_write_request_in_flight must be called with non-empty write requests");
        // Set stream.[[inFlightWriteRequest]] to writeRequest.
        self.in_flight_write_request = Some(write_request);
    }

    fn writable_stream_mark_close_request_in_flight(&mut self) {
        // Set stream.[[inFlightCloseRequest]] to stream.[[closeRequest]].
        // Set stream.[[closeRequest]] to undefined.
        self.in_flight_close_request =
            Some(self.close_request.take().expect(
                "writable_stream_mark_close_request_in_flight called without close request",
            ))
    }

    fn writable_stream_has_operation_marked_in_flight(&self) -> bool {
        if self.in_flight_write_request.is_none() && self.in_flight_close_request.is_none() {
            // If stream.[[inFlightWriteRequest]] is undefined and stream.[[inFlightCloseRequest]] is undefined, return false.
            false
        } else {
            // Return true.
            true
        }
    }

    pub(crate) fn writable_stream_close_queued_or_in_flight(&self) -> bool {
        if self.close_request.is_none() && self.in_flight_close_request.is_none() {
            // If stream.[[closeRequest]] is undefined and stream.[[inFlightCloseRequest]] is undefined, return false.
            false
        } else {
            // Return true.
            true
        }
    }

    fn writable_stream_add_write_request(&mut self, ctx: &Ctx<'js>) -> Result<Promise<'js>> {
        // Let promise be a new promise.
        let promise = ResolveablePromise::new(ctx)?;
        // Append promise to stream.[[writeRequests]].
        self.write_requests.push_back(promise.clone());
        Ok(promise.promise.clone())
    }

    fn writable_stream_finish_in_flight_write_with_error<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        mut objects: WritableStreamObjects<'js, W>,
        error: Value<'js>,
    ) -> Result<()> {
        // Reject stream.[[inFlightWriteRequest]] with error.
        // Set stream.[[inFlightWriteRequest]] to undefined.
        objects.stream.in_flight_write_request.take().expect("writable_stream_finish_in_flight_write_with_error called without in flight write request").reject(error.clone())?;

        // Perform ! WritableStreamDealWithRejection(stream, error).
        Self::writable_stream_deal_with_rejection(ctx, objects, error)?;

        Ok(())
    }

    fn writable_stream_finish_in_flight_close_with_error<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        mut objects: WritableStreamObjects<'js, W>,
        error: Value<'js>,
    ) -> Result<()> {
        // Reject stream.[[inFlightCloseRequest]] with error.
        // Set stream.[[inFlightCloseRequest]] to undefined.
        objects.stream.in_flight_close_request.take().expect("writable_stream_finish_in_flight_close_with_error called without in flight close request").reject(error.clone())?;

        // Assert: stream.[[state]] is "writable" or "erroring".

        // If stream.[[pendingAbortRequest]] is not undefined,
        if let Some(pending_abort_request) = objects.stream.pending_abort_request.take() {
            // Reject stream.[[pendingAbortRequest]]'s promise with error.
            // Set stream.[[pendingAbortRequest]] to undefined.
            pending_abort_request.promise.reject(error.clone())?
        }

        // Perform ! WritableStreamDealWithRejection(stream, error).
        Self::writable_stream_deal_with_rejection(ctx, objects, error)?;

        Ok(())
    }

    fn writable_stream_deal_with_rejection<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        objects: WritableStreamObjects<'js, W>,
        error: Value<'js>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // Let state be stream.[[state]].
        match &objects.stream.state {
            // If state is "writable",
            WritableStreamState::Writable => {
                // Perform ! WritableStreamStartErroring(stream, error).
                Self::writable_stream_start_erroring(ctx, objects, error)
            },
            WritableStreamState::Erroring(ref stored_error) => {
                let stored_error = stored_error.clone();
                // Perform ! WritableStreamFinishErroring(stream).
                Self::writable_stream_finish_erroring(ctx, objects, stored_error)
            },
            other => panic!("WritableStreamDealWithRejection must be called in state 'writable' or 'erroring', found {other:?}"),
        }
    }

    fn writable_stream_finish_in_flight_write(&mut self, ctx: Ctx<'js>) -> Result<()> {
        // Resolve stream.[[inFlightWriteRequest]] with undefined.
        // Set stream.[[inFlightWriteRequest]] to undefined.
        self.in_flight_write_request
            .take()
            .expect("writable_stream_finish_in_flight_write called without in flight write request")
            .resolve(Value::new_undefined(ctx))
    }

    fn writable_stream_finish_in_flight_close<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        // Let writer be stream.[[writer]].
        mut objects: WritableStreamObjects<'js, W>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // Assert: stream.[[inFlightCloseRequest]] is not undefined.

        // Resolve stream.[[inFlightCloseRequest]] with undefined.
        // Set stream.[[inFlightCloseRequest]] to undefined.
        objects
            .stream
            .in_flight_close_request
            .take()
            .expect("writable_stream_finish_in_flight_close called without in flight close request")
            .resolve(Value::new_undefined(ctx.clone()))?;

        // Let state be stream.[[state]].
        // If state is "erroring",
        if let WritableStreamState::Erroring(_) = objects.stream.state {
            // Set stream.[[storedError]] to undefined.
            // (implicitly covered by change to Closed below)

            // If stream.[[pendingAbortRequest]] is not undefined,
            if let Some(pending_abort_request) = objects.stream.pending_abort_request.take() {
                // Resolve stream.[[pendingAbortRequest]]'s promise with undefined.
                // Set stream.[[pendingAbortRequest]] to undefined.
                pending_abort_request
                    .promise
                    .resolve(Value::new_undefined(ctx.clone()))?;
            }
        }

        // Set stream.[[state]] to "closed".
        objects.stream.state = WritableStreamState::Closed;

        // If writer is not undefined, resolve writer.[[closedPromise]] with undefined.
        objects.with_writer(
            |objects| {
                objects
                    .writer
                    .closed_promise
                    .resolve(Value::new_undefined(ctx.clone()))?;

                Ok(objects)
            },
            Ok,
        )
    }

    fn writable_stream_update_backpressure<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        // Let writer be stream.[[writer]].
        mut objects: WritableStreamObjects<'js, W>,
        backpressure: bool,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // If writer is not undefined and backpressure is not stream.[[backpressure]],
        objects = objects.with_writer(
            |mut objects| {
                if backpressure != objects.stream.backpressure {
                    if backpressure {
                        // If backpressure is true, set writer.[[readyPromise]] to a new promise.
                        objects.writer.ready_promise = ResolveablePromise::new(&ctx)?;
                    } else {
                        // Otherwise,
                        // Resolve writer.[[readyPromise]] with undefined.
                        objects
                            .writer
                            .ready_promise
                            .resolve(Value::new_undefined(ctx))?;
                    }
                }

                Ok(objects)
            },
            Ok,
        )?;

        // Set stream.[[backpressure]] to backpressure.
        objects.stream.backpressure = backpressure;

        Ok(objects)
    }

    fn writer_mut(&mut self) -> Option<WritableStreamDefaultWriterOwned<'js>> {
        self.writer.clone().map(OwnedBorrowMut::from_class)
    }

    pub(crate) fn stored_error(&self) -> Option<Value<'js>> {
        match self.state {
            WritableStreamState::Erroring(ref stored_error)
            | WritableStreamState::Errored(ref stored_error) => Some(stored_error.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Trace, Clone, JsLifetime)]
pub(crate) enum WritableStreamState<'js> {
    Writable,
    Closed,
    Erroring(Value<'js>),
    Errored(Value<'js>),
}

#[derive(JsLifetime, Trace)]
struct PendingAbortRequest<'js> {
    promise: ResolveablePromise<'js>,
    reason: Value<'js>,
    was_already_erroring: bool,
}

#[derive(Default)]
struct UnderlyingSink<'js> {
    // callback UnderlyingSinkStartCallback = any (WritableStreamDefaultController controller);
    start: Option<Function<'js>>,
    // callback UnderlyingSinkWriteCallback = Promise<undefined> (any chunk, WritableStreamDefaultController controller);
    write: Option<Function<'js>>,
    // callback UnderlyingSinkCloseCallback = Promise<undefined> ();
    close: Option<Function<'js>>,
    // callback UnderlyingSinkAbortCallback = Promise<undefined> (optional any reason);
    abort: Option<Function<'js>>,
    r#type: Option<Value<'js>>,
}

impl<'js> UnderlyingSink<'js> {
    fn from_object(obj: Object<'js>) -> Result<Self> {
        let start = obj.get_optional::<_, _>("start")?;
        let write = obj.get_optional::<_, _>("write")?;
        let close = obj.get_optional::<_, _>("close")?;
        let abort = obj.get_optional::<_, _>("abort")?;
        let r#type = obj.get_optional::<_, _>("type")?;

        Ok(Self {
            start,
            write,
            close,
            abort,
            r#type,
        })
    }
}
