use llrt_abort::{AbortController, AbortSignal};
use llrt_utils::{
    object::CreateSymbol,
    option::{Null, Undefined},
    primordials::Primordial,
};
use rquickjs::{
    class::{JsClass, OwnedBorrowMut, Trace},
    function::Constructor,
    methods,
    prelude::{Opt, This},
    Class, Ctx, Error, Exception, Function, JsLifetime, Object, Promise, Result, Symbol, Value,
};

use super::{
    default_writer::WritableStreamDefaultWriterOwned,
    objects::{WritableStreamClassObjects, WritableStreamObjects},
    writer::{UndefinedWriter, WritableStreamWriter},
    UnderlyingSink, WritableStream, WritableStreamClass, WritableStreamOwned, WritableStreamState,
};
use crate::{
    class_from_owned_borrow_mut, promise_resolved_with,
    queuing_strategy::{SizeAlgorithm, SizeValue},
    upon_promise, Container, PromisePrimordials, UnwrapOrUndefined,
};

#[rquickjs::class]
#[derive(JsLifetime, Trace)]
pub(crate) struct WritableStreamDefaultController<'js> {
    abort_algorithm: Option<AbortAlgorithm<'js>>,
    close_algorithm: Option<CloseAlgorithm<'js>>,
    container: Container<'js>,
    pub(super) started: bool,
    strategy_hwm: f64,
    strategy_size_algorithm: Option<SizeAlgorithm<'js>>,
    pub(super) abort_controller: Class<'js, AbortController<'js>>,
    pub(super) stream: WritableStreamClass<'js>,
    write_algorithm: Option<WriteAlgorithm<'js>>,

    primordials: WritableStreamDefaultControllerPrimordials<'js>,
}

pub(crate) type WritableStreamDefaultControllerClass<'js> =
    Class<'js, WritableStreamDefaultController<'js>>;
pub(crate) type WritableStreamDefaultControllerOwned<'js> =
    OwnedBorrowMut<'js, WritableStreamDefaultController<'js>>;

impl<'js> WritableStreamDefaultController<'js> {
    pub(super) fn set_up_writable_stream_default_controller_from_underlying_sink(
        ctx: Ctx<'js>,
        stream: WritableStreamOwned<'js>,
        underlying_sink: Null<Undefined<Object<'js>>>,
        underlying_sink_dict: UnderlyingSink<'js>,
        high_water_mark: f64,
        size_algorithm: SizeAlgorithm<'js>,
    ) -> Result<()> {
        let (start_algorithm, write_algorithm, close_algorithm, abort_algorithm) = (
            // If underlyingSinkDict["start"] exists, then set startAlgorithm to an algorithm which returns the result of invoking underlyingSinkDict["start"] with argument list
            // « controller », exception behavior "rethrow", and callback this value underlyingSink.
            underlying_sink_dict
                .start
                .map(|f| StartAlgorithm::Function {
                    f,
                    underlying_sink: underlying_sink.clone(),
                })
                .unwrap_or(StartAlgorithm::ReturnUndefined),
            // If underlyingSinkDict["write"] exists, then set writeAlgorithm to an algorithm which takes an argument chunk and returns the result of invoking underlyingSinkDict["write"] with argument list
            // « chunk, controller » and callback this value underlyingSink.
            underlying_sink_dict
                .write
                .map(|f| WriteAlgorithm::Function {
                    f,
                    underlying_sink: underlying_sink.clone(),
                })
                .unwrap_or(WriteAlgorithm::ReturnPromiseUndefined),
            // If underlyingSinkDict["close"] exists, then set closeAlgorithm to an algorithm which returns the result of invoking underlyingSinkDict["close"] with argument list
            // «» and callback this value underlyingSink.
            underlying_sink_dict
                .close
                .map(|f| CloseAlgorithm::Function {
                    f,
                    underlying_sink: underlying_sink.clone(),
                })
                .unwrap_or(CloseAlgorithm::ReturnPromiseUndefined),
            // If underlyingSinkDict["abort"] exists, then set abortAlgorithm to an algorithm which takes an argument reason and returns the result of invoking underlyingSinkDict["abort"] with argument list
            // « reason » and callback this value underlyingSink.
            underlying_sink_dict
                .abort
                .map(|f| AbortAlgorithm::Function {
                    f,
                    underlying_sink: underlying_sink.clone(),
                })
                .unwrap_or(AbortAlgorithm::ReturnPromiseUndefined),
        );

        // Perform ? SetUpWritableStreamDefaultController(stream, controller, startAlgorithm, writeAlgorithm, closeAlgorithm, abortAlgorithm, highWaterMark, sizeAlgorithm).
        Self::set_up_writable_stream_default_controller(
            ctx,
            stream,
            start_algorithm,
            write_algorithm,
            close_algorithm,
            abort_algorithm,
            high_water_mark,
            size_algorithm,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn set_up_writable_stream_default_controller(
        ctx: Ctx<'js>,
        stream: WritableStreamOwned<'js>,
        start_algorithm: StartAlgorithm<'js>,
        write_algorithm: WriteAlgorithm<'js>,
        close_algorithm: CloseAlgorithm<'js>,
        abort_algorithm: AbortAlgorithm<'js>,
        high_water_mark: f64,
        size_algorithm: SizeAlgorithm<'js>,
    ) -> Result<()> {
        // TODO: needed?
        let (stream_class, mut stream) = class_from_owned_borrow_mut(stream);

        let controller = Self {
            // Set controller.[[stream]] to stream.
            stream: stream_class,

            // Perform ! ResetQueue(controller).
            container: Container::new(),

            // Set controller.[[abortController]] to a new AbortController.
            abort_controller: Class::instance(ctx.clone(), AbortController::new(ctx.clone())?)?,

            // Set controller.[[started]] to false.
            started: false,

            // Set controller.[[strategySizeAlgorithm]] to sizeAlgorithm.
            strategy_size_algorithm: Some(size_algorithm),
            // Set controller.[[strategyHWM]] to highWaterMark.
            strategy_hwm: high_water_mark,

            // Set controller.[[writeAlgorithm]] to writeAlgorithm.
            write_algorithm: Some(write_algorithm),
            // Set controller.[[closeAlgorithm]] to closeAlgorithm.
            close_algorithm: Some(close_algorithm),
            // Set controller.[[abortAlgorithm]] to abortAlgorithm.
            abort_algorithm: Some(abort_algorithm),

            primordials: WritableStreamDefaultControllerPrimordials::get(&ctx)?.clone(),
        };

        let controller_class = Class::instance(ctx.clone(), controller)?;

        // Set stream.[[controller]] to controller.
        stream.controller = Some(controller_class.clone());

        let objects = WritableStreamObjects::from_stream(stream);

        // Let backpressure be ! WritableStreamDefaultControllerGetBackpressure(controller).
        let backpressure = objects
            .controller
            .writable_stream_default_controller_get_backpressure();
        // Perform ! WritableStreamUpdateBackpressure(stream, backpressure).
        let objects = WritableStream::writable_stream_update_backpressure(
            ctx.clone(),
            objects,
            backpressure,
        )?;
        let promise_primordials = objects.stream.promise_primordials.clone();

        // Let startResult be the result of performing startAlgorithm. (This may throw an exception.)
        let (start_result, objects_class) =
            Self::start_algorithm(ctx.clone(), objects, start_algorithm)?;

        // Let startPromise be a promise resolved with startResult.
        let start_promise = promise_resolved_with(&ctx, &promise_primordials, Ok(start_result))?;

        let _ = upon_promise::<Value<'js>, _>(ctx.clone(), start_promise, {
            move |ctx, result| {
                let mut objects =
                    WritableStreamObjects::from_class_no_writer(objects_class).refresh_writer();
                match result {
                    // Upon fulfillment of startPromise,
                    Ok(_) => {
                        // Set controller.[[started]] to true.
                        objects.controller.started = true;
                        // Perform ! WritableStreamDefaultControllerAdvanceQueueIfNeeded(controller).
                        Self::writable_stream_default_controller_advance_queue_if_needed(
                            ctx, objects,
                        )?;
                    },
                    // Upon rejection of startPromise with reason r,
                    Err(r) => {
                        // Set controller.[[started]] to true.
                        objects.controller.started = true;

                        // Perform ! WritableStreamDealWithRejection(stream, r).
                        WritableStream::writable_stream_deal_with_rejection(ctx, objects, r)?;
                    },
                }
                Ok(())
            }
        })?;

        Ok(())
    }

    pub(super) fn writable_stream_default_controller_close<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        mut objects: WritableStreamObjects<'js, W>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        let close_sentinel = objects
            .controller
            .primordials
            .close_sentinel
            .as_value()
            .clone();

        // Perform ! EnqueueValueWithSize(controller, close sentinel, 0).
        objects.controller.container.enqueue_value_with_size(
            &ctx,
            close_sentinel,
            SizeValue::Native(0.0),
        )?;

        // Perform ! WritableStreamDefaultControllerAdvanceQueueIfNeeded(controller).
        objects = Self::writable_stream_default_controller_advance_queue_if_needed(ctx, objects)?;

        Ok(objects)
    }

    pub(super) fn writable_stream_default_controller_get_desired_size(&self) -> f64 {
        self.strategy_hwm - self.container.queue_total_size
    }

    pub fn writable_stream_default_controller_get_backpressure(&self) -> bool {
        // Let desiredSize be ! WritableStreamDefaultControllerGetDesiredSize(controller).
        let desired_size = self.writable_stream_default_controller_get_desired_size();
        // Return true if desiredSize ≤ 0, or false otherwise.
        desired_size <= 0.0
    }

    pub(super) fn writable_stream_default_controller_get_chunk_size(
        ctx: Ctx<'js>,
        mut objects: WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
        chunk: Value<'js>,
    ) -> Result<(
        SizeValue<'js>,
        WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
    )> {
        let (return_value, objects_class) =
            Self::strategy_size_algorithm(ctx.clone(), objects, chunk);

        // Let returnValue be the result of performing controller.[[strategySizeAlgorithm]], passing in chunk, and interpreting the result as a completion record.
        match return_value {
            Ok(chunk_size) => {
                objects = WritableStreamObjects::from_class(objects_class);
                Ok((chunk_size, objects))
            },
            // If returnValue is an abrupt completion,
            Err(Error::Exception) => {
                let reason = ctx.catch();

                objects = WritableStreamObjects::from_class(objects_class);

                // Perform ! WritableStreamDefaultControllerErrorIfNeeded(controller, returnValue.[[Value]]).
                objects = Self::writable_stream_default_controller_error_if_needed(
                    ctx.clone(),
                    objects,
                    reason,
                )?;

                // Return 1.
                Ok((SizeValue::Native(1.0), objects))
            },
            Err(err) => Err(err),
        }
    }

    fn writable_stream_default_controller_error_if_needed(
        ctx: Ctx<'js>,
        objects: WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
        error: Value<'js>,
    ) -> Result<WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>> {
        // If controller.[[stream]].[[state]] is "writable", perform ! WritableStreamDefaultControllerError(controller, error).
        if let WritableStreamState::Writable = objects.stream.state {
            Self::writable_stream_default_controller_error(ctx, objects, error)
        } else {
            Ok(objects)
        }
    }

    fn writable_stream_default_controller_error<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: WritableStreamObjects<'js, W>,
        reason: Value<'js>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // Perform ! WritableStreamDefaultControllerClearAlgorithms(controller).
        objects
            .controller
            .writable_stream_default_controller_clear_algorithms();

        // Perform ! WritableStreamStartErroring(stream, error).
        objects = WritableStream::writable_stream_start_erroring(ctx, objects, reason)?;

        Ok(objects)
    }

    fn writable_stream_default_controller_clear_algorithms(&mut self) {
        // Set controller.[[writeAlgorithm]] to undefined.
        self.write_algorithm = None;

        // Set controller.[[closeAlgorithm]] to undefined.
        self.close_algorithm = None;

        // Set controller.[[abortAlgorithm]] to undefined.
        self.abort_algorithm = None;

        // Set controller.[[strategySizeAlgorithm]] to undefined.
        self.strategy_size_algorithm = None;
    }

    pub(super) fn writable_stream_default_controller_write(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
        chunk: Value<'js>,
        chunk_size: SizeValue<'js>,
    ) -> Result<WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>> {
        // Let enqueueResult be EnqueueValueWithSize(controller, chunk, chunkSize).
        let enqueue_result = objects
            .controller
            .container
            .enqueue_value_with_size(&ctx, chunk, chunk_size);

        match enqueue_result {
            // If enqueueResult is an abrupt completion,
            Err(Error::Exception) => {
                let reason = ctx.catch();
                // Perform ! WritableStreamDefaultControllerErrorIfNeeded(controller, enqueueResult.[[Value]]).
                objects =
                    Self::writable_stream_default_controller_error_if_needed(ctx, objects, reason)?;

                return Ok(objects);
            },
            Err(err) => return Err(err),
            Ok(()) => {},
        }

        // If ! WritableStreamCloseQueuedOrInFlight(stream) is false and stream.[[state]] is "writable",
        if !objects.stream.writable_stream_close_queued_or_in_flight()
            && matches!(objects.stream.state, WritableStreamState::Writable)
        {
            // Let backpressure be ! WritableStreamDefaultControllerGetBackpressure(controller).
            let backpressure = objects
                .controller
                .writable_stream_default_controller_get_backpressure();

            // Perform ! WritableStreamUpdateBackpressure(stream, backpressure).
            objects = WritableStream::writable_stream_update_backpressure(
                ctx.clone(),
                objects,
                backpressure,
            )?;
        }

        // Perform ! WritableStreamDefaultControllerAdvanceQueueIfNeeded(controller).
        let objects =
            Self::writable_stream_default_controller_advance_queue_if_needed(ctx, objects)?;

        Ok(objects)
    }

    fn writable_stream_default_controller_advance_queue_if_needed<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        objects: WritableStreamObjects<'js, W>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // If controller.[[started]] is false, return.
        // If stream.[[inFlightWriteRequest]] is not undefined, return.
        if !objects.controller.started || objects.stream.in_flight_write_request.is_some() {
            return Ok(objects);
        }

        // Let state be stream.[[state]].

        // If state is "erroring",
        if let WritableStreamState::Erroring(ref stored_error) = objects.stream.state {
            let stored_error = stored_error.clone();
            // Perform ! WritableStreamFinishErroring(stream).
            // Return.
            return WritableStream::writable_stream_finish_erroring(ctx, objects, stored_error);
        }

        let value = match objects.controller.container.queue.front() {
            // If controller.[[queue]] is empty, return.
            None => {
                return Ok(objects);
            },
            // Let value be ! PeekQueueValue(controller).
            Some(value) => value.clone(),
        };

        if value.value.as_symbol() == Some(&objects.controller.primordials.close_sentinel) {
            // If value is the close sentinel, perform ! WritableStreamDefaultControllerProcessClose(controller).
            Self::writable_stream_default_controller_process_close(ctx, objects)
        } else {
            // Otherwise, perform ! WritableStreamDefaultControllerProcessWrite(controller, value).
            Self::writable_stream_default_controller_process_write(ctx, objects, value.value)
        }
    }

    fn writable_stream_default_controller_process_close<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: WritableStreamObjects<'js, W>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // Perform ! WritableStreamMarkCloseRequestInFlight(stream).
        objects
            .stream
            .writable_stream_mark_close_request_in_flight();

        // Perform ! DequeueValue(controller).
        objects.controller.container.dequeue_value();

        // Assert: controller.[[queue]] is empty.

        // Let sinkClosePromise be the result of performing controller.[[closeAlgorithm]].
        let (sink_close_promise, objects_class) = Self::close_algorithm(&ctx, objects)?;

        objects = WritableStreamObjects::from_class(objects_class.clone());

        // Perform ! WritableStreamDefaultControllerClearAlgorithms(controller).
        objects
            .controller
            .writable_stream_default_controller_clear_algorithms();

        upon_promise::<Value<'js>, ()>(ctx, sink_close_promise, |ctx, result| {
            let objects = WritableStreamObjects::from_class(objects_class);
            match result {
                // Upon fulfillment of sinkClosePromise,
                Ok(_) => {
                    // Perform ! WritableStreamFinishInFlightClose(stream).
                    WritableStream::writable_stream_finish_in_flight_close(objects)?;
                },
                // Upon rejection of sinkClosePromise with reason reason,
                Err(reason) => {
                    // Perform ! WritableStreamFinishInFlightCloseWithError(stream, reason).
                    WritableStream::writable_stream_finish_in_flight_close_with_error(
                        ctx, objects, reason,
                    )?;
                },
            }

            Ok(())
        })?;

        Ok(objects)
    }

    fn writable_stream_default_controller_process_write<W: WritableStreamWriter<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: WritableStreamObjects<'js, W>,
        chunk: Value<'js>,
    ) -> Result<WritableStreamObjects<'js, W>> {
        // Perform ! WritableStreamMarkFirstWriteRequestInFlight(stream).
        objects
            .stream
            .writable_stream_mark_first_write_request_in_flight();

        // Let sinkWritePromise be the result of performing controller.[[writeAlgorithm]], passing in chunk.
        let (sink_write_promise, objects_class) = Self::write_algorithm(&ctx, objects, chunk)?;

        // Upon fulfillment of sinkWritePromise,
        upon_promise::<Value<'js>, ()>(ctx, sink_write_promise, {
            let objects_class = objects_class.clone();
            |ctx, result| {
                let mut objects = WritableStreamObjects::from_class(objects_class).refresh_writer();
                match result {
                    Ok(_) => {
                        // Upon fulfillment of sinkWritePromise,
                        // Perform ! WritableStreamFinishInFlightWrite(stream).
                        objects.stream.writable_stream_finish_in_flight_write()?;

                        // Let state be stream.[[state]].
                        let state = &objects.stream.state;

                        // Perform ! DequeueValue(controller).
                        objects.controller.container.dequeue_value();

                        // If ! WritableStreamCloseQueuedOrInFlight(stream) is false and state is "writable",
                        if !objects.stream.writable_stream_close_queued_or_in_flight()
                            && matches!(state, WritableStreamState::Writable)
                        {
                            // Let backpressure be ! WritableStreamDefaultControllerGetBackpressure(controller).
                            let backpressure = objects
                                .controller
                                .writable_stream_default_controller_get_backpressure();

                            // Perform ! WritableStreamUpdateBackpressure(stream, backpressure).
                            objects = WritableStream::writable_stream_update_backpressure(
                                ctx.clone(),
                                objects,
                                backpressure,
                            )?;
                        }

                        // Perform ! WritableStreamDefaultControllerAdvanceQueueIfNeeded(controller).
                        WritableStreamDefaultController::writable_stream_default_controller_advance_queue_if_needed(ctx, objects)?;
                    },
                    Err(reason) => {
                        // Upon rejection of sinkWritePromise with reason,
                        if let WritableStreamState::Writable = objects.stream.state {
                            // If stream.[[state]] is "writable", perform ! WritableStreamDefaultControllerClearAlgorithms(controller).
                            objects
                                .controller
                                .writable_stream_default_controller_clear_algorithms();
                        }
                        // Perform ! WritableStreamFinishInFlightWriteWithError(stream, reason).
                        WritableStream::writable_stream_finish_in_flight_write_with_error(
                            ctx, objects, reason,
                        )?;
                    },
                }

                Ok(())
            }
        })?;

        Ok(WritableStreamObjects::from_class(objects_class))
    }

    pub(super) fn error_steps(&mut self) {
        // Perform ! ResetQueue(this).
        self.reset_queue()
    }

    fn reset_queue(&mut self) {
        // Set container.[[queue]] to a new empty list.
        self.container.queue.clear();
        // Set container.[[queueTotalSize]] to 0.
        self.container.queue_total_size = 0.0;
    }

    pub(super) fn abort_steps<W: WritableStreamWriter<'js>>(
        ctx: &Ctx<'js>,
        mut objects: WritableStreamObjects<'js, W>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, WritableStreamObjects<'js, W>)> {
        // Let result be the result of performing this.[[abortAlgorithm]], passing reason.
        let (result, objects_class) = Self::abort_algorithm(ctx, objects, reason)?;

        objects = WritableStreamObjects::from_class(objects_class);

        // Perform ! WritableStreamDefaultControllerClearAlgorithms(this).
        objects
            .controller
            .writable_stream_default_controller_clear_algorithms();

        // Return result.
        Ok((result, objects))
    }

    fn strategy_size_algorithm(
        ctx: Ctx<'js>,
        objects: WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
        chunk: Value<'js>,
    ) -> (
        Result<SizeValue<'js>>,
        WritableStreamClassObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
    ) {
        let strategy_size_algorithm = objects
            .controller
            .strategy_size_algorithm
            .clone()
            .unwrap_or(SizeAlgorithm::AlwaysOne);

        let objects_class = objects.into_inner();

        (strategy_size_algorithm.call(ctx, chunk), objects_class)
    }

    fn start_algorithm(
        ctx: Ctx<'js>,
        objects: WritableStreamObjects<'js, UndefinedWriter>,
        start_algorithm: StartAlgorithm<'js>,
    ) -> Result<(Value<'js>, WritableStreamClassObjects<'js, UndefinedWriter>)> {
        let objects_class = objects.into_inner();

        Ok((
            start_algorithm.call(ctx, objects_class.controller.clone())?,
            objects_class,
        ))
    }

    fn write_algorithm<W: WritableStreamWriter<'js>>(
        ctx: &Ctx<'js>,
        objects: WritableStreamObjects<'js, W>,
        chunk: Value<'js>,
    ) -> Result<(Promise<'js>, WritableStreamClassObjects<'js, W>)> {
        let write_algorithm =
            objects.controller.write_algorithm.clone().expect(
                "write algorithm used after WritableStreamDefaultControllerClearAlgorithms",
            );
        let promise_primordials = objects.stream.promise_primordials.clone();
        let objects_class = objects.into_inner();

        Ok((
            write_algorithm.call(
                ctx,
                &promise_primordials,
                objects_class.controller.clone().clone(),
                chunk,
            )?,
            objects_class,
        ))
    }

    fn close_algorithm<W: WritableStreamWriter<'js>>(
        ctx: &Ctx<'js>,
        objects: WritableStreamObjects<'js, W>,
    ) -> Result<(Promise<'js>, WritableStreamClassObjects<'js, W>)> {
        let close_algorithm =
            objects.controller.close_algorithm.clone().expect(
                "close algorithm used after WritableStreamDefaultControllerClearAlgorithms",
            );
        let promise_primordials = objects.stream.promise_primordials.clone();
        let objects_class = objects.into_inner();

        Ok((
            close_algorithm.call(ctx, &promise_primordials)?,
            objects_class,
        ))
    }

    fn abort_algorithm<W: WritableStreamWriter<'js>>(
        ctx: &Ctx<'js>,
        objects: WritableStreamObjects<'js, W>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, WritableStreamClassObjects<'js, W>)> {
        let abort_algorithm =
            objects.controller.abort_algorithm.clone().expect(
                "abort algorithm used after WritableStreamDefaultControllerClearAlgorithms",
            );
        let promise_primordials = objects.stream.promise_primordials.clone();
        let objects_class = objects.into_inner();

        Ok((
            abort_algorithm.call(ctx, &promise_primordials, reason)?,
            objects_class,
        ))
    }
}

#[methods(rename_all = "camelCase")]
impl<'js> WritableStreamDefaultController<'js> {
    // this is required by web platform tests
    #[qjs(get)]
    pub fn constructor(ctx: Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        <WritableStreamDefaultController as JsClass>::constructor(&ctx)
    }

    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    // readonly attribute AbortSignal signal;
    #[qjs(get)]
    fn signal(&self) -> Class<'js, AbortSignal<'js>> {
        // Return this.[[abortController]]'s signal.
        self.abort_controller.borrow().signal()
    }

    // undefined error(optional any e);
    fn error(
        ctx: Ctx<'js>,
        controller: This<OwnedBorrowMut<'js, Self>>,
        e: Opt<Value<'js>>,
    ) -> Result<()> {
        let objects = WritableStreamObjects::from_controller(controller.0);

        // Let state be this.[[stream]].[[state]].
        // If state is not "writable", return.
        if !matches!(objects.stream.state, WritableStreamState::Writable) {
            return Ok(());
        }

        // Perform ! WritableStreamDefaultControllerError(this, e).
        Self::writable_stream_default_controller_error(
            ctx.clone(),
            objects.refresh_writer(),
            e.0.unwrap_or_undefined(&ctx),
        )?;

        Ok(())
    }
}

#[derive(Clone)]
enum StartAlgorithm<'js> {
    ReturnUndefined,
    Function {
        f: Function<'js>,
        underlying_sink: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> StartAlgorithm<'js> {
    fn call(
        &self,
        ctx: Ctx<'js>,
        controller: WritableStreamDefaultControllerClass<'js>,
    ) -> Result<Value<'js>> {
        match self {
            StartAlgorithm::ReturnUndefined => Ok(Value::new_undefined(ctx.clone())),
            StartAlgorithm::Function { f, underlying_sink } => {
                f.call::<_, Value>((This(underlying_sink.clone()), controller))
            },
        }
    }
}

#[derive(JsLifetime, Trace, Clone)]
enum WriteAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_sink: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> WriteAlgorithm<'js> {
    fn call(
        &self,
        ctx: &Ctx<'js>,
        promise_primordials: &PromisePrimordials<'js>,
        controller: WritableStreamDefaultControllerClass<'js>,
        chunk: Value<'js>,
    ) -> Result<Promise<'js>> {
        match self {
            WriteAlgorithm::ReturnPromiseUndefined => {
                Ok(promise_primordials.promise_resolved_with_undefined.clone())
            },
            WriteAlgorithm::Function { f, underlying_sink } => promise_resolved_with(
                ctx,
                promise_primordials,
                f.call::<_, Value>((This(underlying_sink.clone()), chunk, controller)),
            ),
        }
    }
}

#[derive(JsLifetime, Trace, Clone)]
enum CloseAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_sink: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> CloseAlgorithm<'js> {
    fn call(
        &self,
        ctx: &Ctx<'js>,
        promise_primordials: &PromisePrimordials<'js>,
    ) -> Result<Promise<'js>> {
        match self {
            CloseAlgorithm::ReturnPromiseUndefined => {
                Ok(promise_primordials.promise_resolved_with_undefined.clone())
            },
            CloseAlgorithm::Function { f, underlying_sink } => promise_resolved_with(
                ctx,
                promise_primordials,
                f.call::<_, Value>((This(underlying_sink.clone()),)),
            ),
        }
    }
}

#[derive(JsLifetime, Trace, Clone)]
enum AbortAlgorithm<'js> {
    ReturnPromiseUndefined,
    Function {
        f: Function<'js>,
        underlying_sink: Null<Undefined<Object<'js>>>,
    },
}

impl<'js> AbortAlgorithm<'js> {
    fn call(
        &self,
        ctx: &Ctx<'js>,
        promise_primordials: &PromisePrimordials<'js>,
        reason: Value<'js>,
    ) -> Result<Promise<'js>> {
        match self {
            AbortAlgorithm::ReturnPromiseUndefined => {
                Ok(promise_primordials.promise_resolved_with_undefined.clone())
            },
            AbortAlgorithm::Function { f, underlying_sink } => promise_resolved_with(
                ctx,
                promise_primordials,
                f.call::<_, Value>((This(underlying_sink.clone()), reason)),
            ),
        }
    }
}

#[derive(Trace, Clone, JsLifetime)]
struct WritableStreamDefaultControllerPrimordials<'js> {
    close_sentinel: Symbol<'js>,
}

impl<'js> Primordial<'js> for WritableStreamDefaultControllerPrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            close_sentinel: Symbol::for_description(ctx, "close sentinel")?,
        })
    }
}
