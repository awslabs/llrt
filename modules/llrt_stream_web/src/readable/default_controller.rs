use rquickjs::{
    class::{OwnedBorrow, OwnedBorrowMut, Trace},
    methods,
    prelude::{Opt, This},
    Class, Ctx, Error, Exception, JsLifetime, Object, Promise, Result, Value,
};

use super::{
    byte_controller::ReadableByteStreamControllerOwned,
    controller::{
        ReadableStreamController, ReadableStreamControllerClass, ReadableStreamControllerOwned,
    },
    default_reader::ReadableStreamDefaultReaderOrUndefined,
    objects::{
        ReadableStreamClassObjects, ReadableStreamDefaultControllerObjects,
        ReadableStreamDefaultReaderObjects, ReadableStreamObjects,
    },
    promise_resolved_with,
    reader::ReadableStreamReader,
    CancelAlgorithm, Null, PullAlgorithm, ReadableStream, ReadableStreamClass, ReadableStreamOwned,
    ReadableStreamReadRequest, ReadableStreamState, SizeAlgorithm, StartAlgorithm, Undefined,
    UnderlyingSource,
};
use crate::{
    class_from_owned_borrow_mut, queueing_strategy::SizeValue, upon_promise, Container,
    UnwrapOrUndefined,
};

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct ReadableStreamDefaultController<'js> {
    cancel_algorithm: Option<CancelAlgorithm<'js>>,
    close_requested: bool,
    pull_again: bool,
    pull_algorithm: Option<PullAlgorithm<'js>>,
    pulling: bool,
    container: Container<'js>,
    started: bool,
    strategy_hwm: f64,
    strategy_size_algorithm: Option<SizeAlgorithm<'js>>,
    pub(super) stream: ReadableStreamClass<'js>,
}

pub(super) type ReadableStreamDefaultControllerClass<'js> =
    Class<'js, ReadableStreamDefaultController<'js>>;
pub(super) type ReadableStreamDefaultControllerOwned<'js> =
    OwnedBorrowMut<'js, ReadableStreamDefaultController<'js>>;

impl<'js> ReadableStreamDefaultController<'js> {
    pub(super) fn set_up_readable_stream_default_controller_from_underlying_source(
        ctx: Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
        underlying_source_dict: UnderlyingSource<'js>,
        high_water_mark: f64,
        size_algorithm: SizeAlgorithm<'js>,
    ) -> Result<()> {
        let (start_algorithm, pull_algorithm, cancel_algorithm) = (
            // If underlyingSourceDict["start"] exists, then set startAlgorithm to an algorithm which returns the result of invoking underlyingSourceDict["start"] with argument list
            // « controller » and callback this value underlyingSource.
            underlying_source_dict
                .start
                .map(|f| StartAlgorithm::Function {
                    f,
                    underlying_source: underlying_source.clone(),
                })
                .unwrap_or(StartAlgorithm::ReturnUndefined),
            // If underlyingSourceDict["pull"] exists, then set pullAlgorithm to an algorithm which returns the result of invoking underlyingSourceDict["pull"] with argument list
            // « controller » and callback this value underlyingSource.
            underlying_source_dict
                .pull
                .map(|f| PullAlgorithm::Function {
                    f,
                    underlying_source: underlying_source.clone(),
                })
                .unwrap_or(PullAlgorithm::ReturnPromiseUndefined),
            // If underlyingSourceDict["cancel"] exists, then set cancelAlgorithm to an algorithm which takes an argument reason and returns the result of invoking underlyingSourceDict["cancel"] with argument list
            // « reason » and callback this value underlyingSource.
            underlying_source_dict
                .cancel
                .map(|f| CancelAlgorithm::Function {
                    f,
                    underlying_source,
                })
                .unwrap_or(CancelAlgorithm::ReturnPromiseUndefined),
        );

        // Perform ? SetUpReadableStreamDefaultController(stream, controller, startAlgorithm, pullAlgorithm, cancelAlgorithm, highWaterMark, sizeAlgorithm).
        Self::set_up_readable_stream_default_controller(
            ctx.clone(),
            stream,
            start_algorithm,
            pull_algorithm,
            cancel_algorithm,
            high_water_mark,
            size_algorithm,
        )?;

        Ok(())
    }

    pub(super) fn set_up_readable_stream_default_controller(
        ctx: Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
        start_algorithm: StartAlgorithm<'js>,
        pull_algorithm: PullAlgorithm<'js>,
        cancel_algorithm: CancelAlgorithm<'js>,
        high_water_mark: f64,
        size_algorithm: SizeAlgorithm<'js>,
    ) -> Result<Class<'js, Self>> {
        let (stream_class, mut stream) = class_from_owned_borrow_mut(stream);

        let controller = ReadableStreamDefaultController {
            // Set controller.[[stream]] to stream.
            stream: stream_class.clone(),

            // Perform ! ResetQueue(controller).
            container: Container::new(),

            // Set controller.[[started]], controller.[[closeRequested]], controller.[[pullAgain]], and controller.[[pulling]] to false.
            started: false,
            close_requested: false,
            pull_again: false,
            pulling: false,

            // Set controller.[[strategySizeAlgorithm]] to sizeAlgorithm and controller.[[strategyHWM]] to highWaterMark.
            strategy_size_algorithm: Some(size_algorithm),
            strategy_hwm: high_water_mark,

            // Set controller.[[pullAlgorithm]] to pullAlgorithm.
            pull_algorithm: Some(pull_algorithm),
            // Set controller.[[cancelAlgorithm]] to cancelAlgorithm.
            cancel_algorithm: Some(cancel_algorithm),
        };

        let controller_class = Class::instance(ctx.clone(), controller)?;

        // Set stream.[[controller]] to controller.
        stream.controller = ReadableStreamControllerClass::ReadableStreamDefaultController(
            controller_class.clone(),
        );

        let objects = ReadableStreamObjects::new_default(
            stream,
            OwnedBorrowMut::from_class(controller_class),
        );

        let promise_primordials = objects.stream.promise_primordials.clone();

        // Let startResult be the result of performing startAlgorithm. (This might throw an exception.)
        let (start_result, objects_class) =
            Self::start_algorithm(ctx.clone(), objects, start_algorithm)?;

        // Let startPromise be a promise resolved with startResult.
        let start_promise = promise_resolved_with(&ctx, &promise_primordials, Ok(start_result))?;

        let _ = upon_promise::<Value<'js>, _>(ctx.clone(), start_promise, {
            let objects_class = objects_class.clone();
            move |ctx, result| {
                let mut objects =
                    ReadableStreamObjects::from_class_no_reader(objects_class).refresh_reader();

                match result {
                    // Upon fulfillment of startPromise,
                    Ok(_) => {
                        // Set controller.[[started]] to true.
                        objects.controller.started = true;
                        // Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
                        Self::readable_stream_default_controller_call_pull_if_needed(ctx, objects)?;
                    },
                    // Upon rejection of startPromise with reason r,
                    Err(r) => {
                        // Perform ! ReadableByteStreamControllerError(controller, r).
                        Self::readable_stream_default_controller_error(objects, r)?;
                    },
                }
                Ok(())
            }
        })?;

        Ok(objects_class.controller)
    }

    fn reset_queue(&mut self) {
        // Set container.[[queue]] to a new empty list.
        self.container.queue.clear();
        // Set container.[[queueTotalSize]] to 0.
        self.container.queue_total_size = 0.0;
    }

    fn readable_stream_default_controller_call_pull_if_needed<
        R: ReadableStreamDefaultReaderOrUndefined<'js>,
    >(
        ctx: Ctx<'js>,
        objects: ReadableStreamDefaultControllerObjects<'js, R>,
    ) -> Result<ReadableStreamDefaultControllerObjects<'js, R>> {
        // Let shouldPull be ! ReadableStreamDefaultControllerShouldCallPull(controller).

        let (should_pull, mut objects) =
            ReadableStreamDefaultController::readable_stream_default_controller_should_call_pull(
                objects,
            );

        // If shouldPull is false, return.
        if !should_pull {
            return Ok(objects);
        }

        // If controller.[[pulling]] is true,
        if objects.controller.pulling {
            // Set controller.[[pullAgain]] to true.
            objects.controller.pull_again = true;

            // Return.
            return Ok(objects);
        }

        // Set controller.[[pulling]] to true.
        objects.controller.pulling = true;

        // Let pullPromise be the result of performing controller.[[pullAlgorithm]].
        let (pull_promise, objects_class) = Self::pull_algorithm(ctx.clone(), objects)?;

        upon_promise::<Value<'js>, _>(ctx.clone(), pull_promise, {
            let objects_class = objects_class.clone();
            move |ctx, result| {
                let mut objects =
                    ReadableStreamObjects::from_class_no_reader(objects_class).refresh_reader();
                match result {
                    // Upon fulfillment of pullPromise,
                    Ok(_) => {
                        // Set controller.[[pulling]] to false.
                        objects.controller.pulling = false;
                        // If controller.[[pullAgain]] is true,
                        if objects.controller.pull_again {
                            // Set controller.[[pullAgain]] to false.
                            objects.controller.pull_again = false;
                            // Perform ! ReadableStreamDefaultControllerCallPullIfNeeded(controller).
                            Self::readable_stream_default_controller_call_pull_if_needed(
                                ctx, objects,
                            )?;
                        };
                        Ok(())
                    },
                    // Upon rejection of pullPromise with reason e,
                    Err(e) => {
                        // Perform ! ReadableStreamDefaultControllerError(controller, e).
                        Self::readable_stream_default_controller_error(objects, e)?;
                        Ok(())
                    },
                }
            }
        })?;

        Ok(ReadableStreamObjects::from_class(objects_class))
    }

    pub(super) fn readable_stream_default_controller_error<R: ReadableStreamReader<'js>>(
        // Let stream be controller.[[stream]].
        mut objects: ReadableStreamDefaultControllerObjects<'js, R>,
        e: Value<'js>,
    ) -> Result<ReadableStreamDefaultControllerObjects<'js, R>> {
        // If stream.[[state]] is not "readable", return.
        if !matches!(objects.stream.state, ReadableStreamState::Readable) {
            return Ok(objects);
        };

        // Perform ! ResetQueue(controller).
        objects.controller.reset_queue();

        // Perform ! ReadableStreamDefaultControllerClearAlgorithms(controller).
        objects
            .controller
            .readable_stream_default_controller_clear_algorithms();

        // Perform ! ReadableStreamError(stream, e).
        ReadableStream::readable_stream_error(objects, e)
    }

    fn readable_stream_default_controller_should_call_pull<
        R: ReadableStreamDefaultReaderOrUndefined<'js>,
    >(
        mut objects: ReadableStreamDefaultControllerObjects<'js, R>,
    ) -> (bool, ReadableStreamDefaultControllerObjects<'js, R>) {
        // Let stream be controller.[[stream]].
        // If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(controller) is false, return false.
        if !objects
            .controller
            .readable_stream_default_controller_can_close_or_enqueue(&objects.stream)
        {
            return (false, objects);
        }

        // If controller.[[started]] is false, return false.
        if !objects.controller.started {
            return (false, objects);
        }

        {
            let mut ret = false;
            // If ! IsReadableStreamLocked(stream) is true and ! ReadableStreamGetNumReadRequests(stream) > 0, return true.
            objects = objects
                .with_some_reader(
                    |objects| {
                        if ReadableStream::readable_stream_get_num_read_requests(&objects.reader)
                            > 0
                        {
                            ret = true
                        }
                        Ok(objects)
                    },
                    Ok,
                )
                .unwrap();
            if ret {
                return (true, objects);
            }
        }

        // Let desiredSize be ! ReadableStreamDefaultControllerGetDesiredSize(controller).
        let desired_size = objects.controller
            .readable_stream_default_controller_get_desired_size(&objects.stream)
            .0
            .expect(
            "desiredSize should not be null during ReadableStreamDefaultControllerShouldCallPull",
        );
        // If desiredSize > 0, return true.
        if desired_size > 0.0 {
            return (true, objects);
        }

        // Return false.
        (false, objects)
    }

    fn readable_stream_default_controller_clear_algorithms(&mut self) {
        self.pull_algorithm = None;
        self.cancel_algorithm = None;
        self.strategy_size_algorithm = None;
    }

    fn readable_stream_default_controller_can_close_or_enqueue(
        &self,
        stream: &ReadableStream<'js>,
    ) -> bool {
        // Let state be controller.[[stream]].[[state]].
        match stream.state {
            // If controller.[[closeRequested]] is false and state is "readable", return true.
            ReadableStreamState::Readable if !self.close_requested => true,
            // Otherwise, return false.
            _ => false,
        }
    }

    fn readable_stream_default_controller_get_desired_size(
        &self,
        stream: &ReadableStream<'js>,
    ) -> Null<f64> {
        // Let state be controller.[[stream]].[[state]].
        match stream.state {
            // If state is "errored", return null.
            ReadableStreamState::Errored(_) => Null(None),
            // If state is "closed", return 0.
            ReadableStreamState::Closed => Null(Some(0.0)),
            // Return controller.[[strategyHWM]] − controller.[[queueTotalSize]].
            ReadableStreamState::Readable => {
                Null(Some(self.strategy_hwm - self.container.queue_total_size))
            },
        }
    }

    pub(super) fn readable_stream_default_controller_close<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: ReadableStreamDefaultControllerObjects<'js, R>,
    ) -> Result<ReadableStreamDefaultControllerObjects<'js, R>> {
        // If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(controller) is false, return.
        if !objects
            .controller
            .readable_stream_default_controller_can_close_or_enqueue(&objects.stream)
        {
            return Ok(objects);
        }

        // Set controller.[[closeRequested]] to true.
        objects.controller.close_requested = true;

        // If controller.[[queue]] is empty,
        if objects.controller.container.queue.is_empty() {
            // Perform ! ReadableStreamDefaultControllerClearAlgorithms(controller).
            objects
                .controller
                .readable_stream_default_controller_clear_algorithms();
            // Perform ! ReadableStreamClose(stream).
            objects = ReadableStream::readable_stream_close(ctx, objects)?;
        }

        Ok(objects)
    }

    pub(super) fn readable_stream_default_controller_enqueue<
        R: ReadableStreamDefaultReaderOrUndefined<'js>,
    >(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: ReadableStreamDefaultControllerObjects<'js, R>,
        chunk: Value<'js>,
    ) -> Result<ReadableStreamDefaultControllerObjects<'js, R>> {
        // If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(controller) is false, return.
        if !objects
            .controller
            .readable_stream_default_controller_can_close_or_enqueue(&objects.stream)
        {
            return Ok(objects);
        }

        let mut els = true;
        // If ! IsReadableStreamLocked(stream) is true and ! ReadableStreamGetNumReadRequests(stream) > 0, perform ! ReadableStreamFulfillReadRequest(stream, chunk, false).
        objects = objects.with_some_reader(
            |objects| {
                if ReadableStream::readable_stream_get_num_read_requests(&objects.reader) > 0 {
                    els = false;
                    ReadableStream::readable_stream_fulfill_read_request(
                        &ctx,
                        objects,
                        chunk.clone(),
                        false,
                    )
                } else {
                    Ok(objects)
                }
            },
            Ok,
        )?;

        if els {
            // Let result be the result of performing controller.[[strategySizeAlgorithm]], passing in chunk, and interpreting the result as a completion record.
            let (result, objects_class) =
                Self::strategy_size_algorithm(ctx.clone(), objects, chunk.clone());

            objects = ReadableStreamObjects::from_class(objects_class);

            match result {
                // If result is an abrupt completion,
                Err(Error::Exception) => {
                    let err = ctx.catch();
                    // Perform ! ReadableStreamDefaultControllerError(controller, result.[[Value]]).
                    Self::readable_stream_default_controller_error(objects, err.clone())?;

                    return Err(ctx.throw(err));
                },
                // Let chunkSize be result.[[Value]].
                Ok(chunk_size) => {
                    // Let enqueueResult be EnqueueValueWithSize(controller, chunk, chunkSize).
                    let enqueue_result = objects
                        .controller
                        .container
                        .enqueue_value_with_size(&ctx, chunk, chunk_size);

                    match enqueue_result {
                        // If enqueueResult is an abrupt completion,
                        Err(Error::Exception) => {
                            let err = ctx.catch();
                            // Perform ! ReadableStreamDefaultControllerError(controller, enqueueResult.[[Value]]).
                            Self::readable_stream_default_controller_error(objects, err.clone())?;
                            return Err(ctx.throw(err));
                        },
                        Err(err) => return Err(err),
                        Ok(()) => {},
                    }
                },
                Err(err) => return Err(err),
            }
        }

        // Perform ! ReadableStreamDefaultControllerCallPullIfNeeded(controller).
        Self::readable_stream_default_controller_call_pull_if_needed(ctx, objects)
    }

    fn start_algorithm<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableStreamDefaultControllerObjects<'js, R>,
        start_algorithm: StartAlgorithm<'js>,
    ) -> Result<(
        Value<'js>,
        ReadableStreamClassObjects<'js, OwnedBorrowMut<'js, Self>, R>,
    )> {
        let objects_class = objects.into_inner();

        Ok((
            start_algorithm.call(
                ctx,
                ReadableStreamControllerClass::ReadableStreamDefaultController(
                    objects_class.controller.clone(),
                ),
            )?,
            objects_class,
        ))
    }

    fn pull_algorithm<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableStreamDefaultControllerObjects<'js, R>,
    ) -> Result<(
        Promise<'js>,
        ReadableStreamClassObjects<'js, OwnedBorrowMut<'js, Self>, R>,
    )> {
        let pull_algorithm = objects
            .controller
            .pull_algorithm
            .clone()
            .expect("pull algorithm used after ReadableStreamDefaultControllerClearAlgorithms");
        let promise_primordials = objects.stream.promise_primordials.clone();
        let objects_class = objects.into_inner();

        Ok((
            pull_algorithm.call(
                ctx,
                &promise_primordials,
                ReadableStreamControllerClass::ReadableStreamDefaultController(
                    objects_class.controller.clone(),
                ),
            )?,
            objects_class,
        ))
    }

    fn strategy_size_algorithm<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableStreamDefaultControllerObjects<'js, R>,
        chunk: Value<'js>,
    ) -> (
        Result<SizeValue<'js>>,
        ReadableStreamClassObjects<'js, OwnedBorrowMut<'js, Self>, R>,
    ) {
        let strategy_size_algorithm = objects
            .controller
            .strategy_size_algorithm
            .clone()
            .expect("size algorithm used after ReadableStreamDefaultControllerClearAlgorithms");
        let objects_class = objects.into_inner();

        (strategy_size_algorithm.call(ctx, chunk), objects_class)
    }

    pub(super) fn cancel_algorithm<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableStreamDefaultControllerObjects<'js, R>,
        reason: Value<'js>,
    ) -> Result<(
        Promise<'js>,
        ReadableStreamClassObjects<'js, OwnedBorrowMut<'js, Self>, R>,
    )> {
        let cancel_algorithm =
            objects.controller.cancel_algorithm.clone().expect(
                "cancel algorithm used after ReadableStreamDefaultControllerClearAlgorithms",
            );
        let promise_primordials = objects.stream.promise_primordials.clone();
        let objects_class = objects.into_inner();

        Ok((
            cancel_algorithm.call(ctx, &promise_primordials, reason)?,
            objects_class,
        ))
    }
}

#[methods(rename_all = "camelCase")]
impl<'js> ReadableStreamDefaultController<'js> {
    // this is required by web platform tests for unclear reasons
    fn constructor() -> Self {
        unimplemented!()
    }

    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    // readonly attribute unrestricted double? desiredSize;
    #[qjs(get)]
    fn desired_size(&self) -> Null<f64> {
        let stream = OwnedBorrow::from_class(self.stream.clone());
        self.readable_stream_default_controller_get_desired_size(&stream)
    }

    // undefined close();
    fn close(ctx: Ctx<'js>, controller: This<OwnedBorrowMut<'js, Self>>) -> Result<()> {
        let objects = ReadableStreamObjects::from_default_controller(controller.0);

        // If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(this) is false, throw a TypeError exception.
        if !objects
            .controller
            .readable_stream_default_controller_can_close_or_enqueue(&objects.stream)
        {
            return Err(Exception::throw_type(
                &ctx,
                "The stream is not in a state that permits close",
            ));
        }

        // Perform ! ReadableStreamDefaultControllerClose(this).
        Self::readable_stream_default_controller_close(ctx, objects)?;
        Ok(())
    }

    // undefined enqueue(optional any chunk);
    fn enqueue(
        ctx: Ctx<'js>,
        controller: This<OwnedBorrowMut<'js, Self>>,
        chunk: Opt<Value<'js>>,
    ) -> Result<()> {
        let objects = ReadableStreamObjects::from_default_controller(controller.0);

        // If ! ReadableStreamDefaultControllerCanCloseOrEnqueue(this) is false, throw a TypeError exception.
        if !objects
            .controller
            .readable_stream_default_controller_can_close_or_enqueue(&objects.stream)
        {
            return Err(Exception::throw_type(
                &ctx,
                "The stream is not in a state that permits enqueue",
            ));
        }

        objects.with_reader(
            |objects| {
                // Perform ? ReadableStreamDefaultControllerEnqueue(this, chunk).
                Self::readable_stream_default_controller_enqueue(
                    ctx.clone(),
                    objects,
                    chunk.0.clone().unwrap_or_undefined(&ctx),
                )
            },
            |_| panic!("Default controller must not have byob reader"),
            |objects| {
                // Perform ? ReadableStreamDefaultControllerEnqueue(this, chunk).
                Self::readable_stream_default_controller_enqueue(
                    ctx.clone(),
                    objects,
                    chunk.0.clone().unwrap_or_undefined(&ctx),
                )
            },
        )?;

        Ok(())
    }

    // undefined error(optional any e);
    fn error(
        ctx: Ctx<'js>,
        controller: This<OwnedBorrowMut<'js, Self>>,
        e: Opt<Value<'js>>,
    ) -> Result<()> {
        let objects = ReadableStreamObjects::from_default_controller(controller.0);

        // Perform ! ReadableStreamDefaultControllerError(this, e).
        Self::readable_stream_default_controller_error(objects, e.0.unwrap_or_undefined(&ctx))?;
        Ok(())
    }
}

impl<'js> ReadableStreamController<'js> for ReadableStreamDefaultControllerOwned<'js> {
    type Class = ReadableStreamDefaultControllerClass<'js>;

    fn with_controller<C, O>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            ReadableStreamDefaultControllerOwned<'js>,
        ) -> Result<(O, ReadableStreamDefaultControllerOwned<'js>)>,
        _: impl FnOnce(
            C,
            ReadableByteStreamControllerOwned<'js>,
        ) -> Result<(O, ReadableByteStreamControllerOwned<'js>)>,
    ) -> Result<(O, Self)> {
        let (ctx, reader) = default(ctx, self)?;
        Ok((ctx, reader))
    }

    fn into_inner(self) -> Self::Class {
        OwnedBorrowMut::into_inner(self)
    }

    fn from_class(class: Self::Class) -> Self {
        OwnedBorrowMut::from_class(class)
    }

    fn into_erased(self) -> ReadableStreamControllerOwned<'js> {
        ReadableStreamControllerOwned::ReadableStreamDefaultController(self)
    }

    fn try_from_erased(erased: ReadableStreamControllerOwned<'js>) -> Option<Self> {
        match erased {
            ReadableStreamControllerOwned::ReadableStreamDefaultController(r) => Some(r),
            ReadableStreamControllerOwned::ReadableStreamByteController(_) => None,
        }
    }

    fn pull_steps(
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamDefaultReaderObjects<'js, Self>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, Self>> {
        // If this.[[queue]] is not empty,
        if !objects.controller.container.queue.is_empty() {
            // Let chunk be ! DequeueValue(this).
            let chunk = objects.controller.container.dequeue_value();
            // If this.[[closeRequested]] is true and this.[[queue]] is empty,
            if objects.controller.close_requested && objects.controller.container.queue.is_empty() {
                // Perform ! ReadableStreamDefaultControllerClearAlgorithms(this).
                objects
                    .controller
                    .readable_stream_default_controller_clear_algorithms();
                // Perform ! ReadableStreamClose(stream).
                objects = ReadableStream::readable_stream_close(ctx.clone(), objects)?;
            } else {
                // Otherwise, perform ! ReadableStreamDefaultControllerCallPullIfNeeded(this).
                objects =
                    ReadableStreamDefaultController::readable_stream_default_controller_call_pull_if_needed(
                        ctx.clone(),
                        objects,
                    )?;
            }

            // Perform readRequest’s chunk steps, given chunk.
            read_request.chunk_steps_typed(objects, chunk)
        } else {
            // Otherwise,
            // Perform ! ReadableStreamAddReadRequest(stream, readRequest).
            objects
                .stream
                .readable_stream_add_read_request(&mut objects.reader, read_request);
            // Perform ! ReadableStreamDefaultControllerCallPullIfNeeded(this).

            ReadableStreamDefaultController::readable_stream_default_controller_call_pull_if_needed(
                ctx.clone(),
                objects,
            )
        }
    }

    fn cancel_steps<R: ReadableStreamReader<'js>>(
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamObjects<'js, Self, R>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, ReadableStreamObjects<'js, Self, R>)> {
        // Perform ! ResetQueue(this).
        objects.controller.reset_queue();

        // Let result be the result of performing this.[[cancelAlgorithm]], passing reason.
        let (result, objects_class) =
            ReadableStreamDefaultController::cancel_algorithm(ctx.clone(), objects, reason)?;

        objects = ReadableStreamObjects::from_class(objects_class);
        // Perform ! ReadableStreamDefaultControllerClearAlgorithms(this).
        objects
            .controller
            .readable_stream_default_controller_clear_algorithms();

        // Return result.
        Ok((result, objects))
    }

    fn release_steps(&mut self) {}
}
