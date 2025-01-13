use std::collections::VecDeque;

use llrt_utils::{
    error_messages::ERROR_MSG_ARRAY_BUFFER_DETACHED,
    option::{Null, Undefined},
    primordials::{BasePrimordials, Primordial},
    result::ResultExt,
};
use rquickjs::{
    class::{OwnedBorrow, OwnedBorrowMut, Trace},
    function::Constructor,
    methods,
    prelude::{Opt, This},
    ArrayBuffer, Class, Ctx, Error, Exception, Function, IntoJs, JsLifetime, Object, Promise,
    Result, TypedArray, Value,
};

use crate::{
    readable::{
        byob_reader::{ArrayConstructorPrimordials, ReadableStreamReadIntoRequest, ViewBytes},
        controller::{
            ReadableStreamController, ReadableStreamControllerClass, ReadableStreamControllerOwned,
        },
        default_controller::ReadableStreamDefaultControllerOwned,
        default_reader::ReadableStreamReadRequest,
        objects::{
            ReadableByteStreamObjects, ReadableStreamBYOBObjects, ReadableStreamClassObjects,
            ReadableStreamDefaultReaderObjects, ReadableStreamObjects,
        },
        reader::ReadableStreamReader,
        stream::{
            algorithms::{CancelAlgorithm, PullAlgorithm, StartAlgorithm},
            source::UnderlyingSource,
            ReadableStream, ReadableStreamClass, ReadableStreamOwned, ReadableStreamState,
        },
    },
    utils::{
        class_from_owned_borrow_mut,
        promise::{promise_resolved_with, upon_promise},
        UnwrapOrUndefined,
    },
};

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct ReadableByteStreamController<'js> {
    auto_allocate_chunk_size: Option<usize>,
    #[qjs(get)]
    byob_request: Option<Class<'js, ReadableStreamBYOBRequest<'js>>>,
    cancel_algorithm: Option<CancelAlgorithm<'js>>,
    close_requested: bool,
    pull_again: bool,
    pull_algorithm: Option<PullAlgorithm<'js>>,
    pulling: bool,
    pub(super) pending_pull_intos: VecDeque<PullIntoDescriptor<'js>>,
    queue: VecDeque<ReadableByteStreamQueueEntry<'js>>,
    queue_total_size: usize,
    started: bool,
    strategy_hwm: f64,
    pub(super) stream: ReadableStreamClass<'js>,

    #[qjs(skip_trace)]
    pub(super) array_constructor_primordials: ArrayConstructorPrimordials<'js>,
    #[qjs(skip_trace)]
    constructor_array_buffer: Constructor<'js>,
    #[qjs(skip_trace)]
    pub(super) function_array_buffer_is_view: Function<'js>,
}

pub(crate) type ReadableByteStreamControllerClass<'js> =
    Class<'js, ReadableByteStreamController<'js>>;
pub(crate) type ReadableByteStreamControllerOwned<'js> =
    OwnedBorrowMut<'js, ReadableByteStreamController<'js>>;

impl<'js> ReadableByteStreamController<'js> {
    // SetUpReadableByteStreamControllerFromUnderlyingSource
    pub(super) fn set_up_readable_byte_stream_controller_from_underlying_source(
        ctx: &Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
        underlying_source: Null<Undefined<Object<'js>>>,
        underlying_source_dict: UnderlyingSource<'js>,
        high_water_mark: f64,
    ) -> Result<()> {
        let (start_algorithm, pull_algorithm, cancel_algorithm, auto_allocate_chunk_size) = (
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
            // Let autoAllocateChunkSize be underlyingSourceDict["autoAllocateChunkSize"], if it exists, or undefined otherwise.
            underlying_source_dict.auto_allocate_chunk_size,
        );

        // If autoAllocateChunkSize is 0, then throw a TypeError exception.
        if auto_allocate_chunk_size == Some(0) {
            return Err(Exception::throw_type(
                ctx,
                "autoAllocateChunkSize must be greater than 0",
            ));
        }

        Self::set_up_readable_byte_stream_controller(
            ctx.clone(),
            stream,
            start_algorithm,
            pull_algorithm,
            cancel_algorithm,
            high_water_mark,
            auto_allocate_chunk_size,
        )?;

        Ok(())
    }

    pub(super) fn set_up_readable_byte_stream_controller(
        ctx: Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
        start_algorithm: StartAlgorithm<'js>,
        pull_algorithm: PullAlgorithm<'js>,
        cancel_algorithm: CancelAlgorithm<'js>,
        high_water_mark: f64,
        auto_allocate_chunk_size: Option<usize>,
    ) -> Result<Class<'js, Self>> {
        let (stream_class, mut stream) = class_from_owned_borrow_mut(stream);

        let array_constructor_primordials = ArrayConstructorPrimordials::get(&ctx)?.clone();
        let BasePrimordials {
            constructor_array_buffer,
            function_array_buffer_is_view,
            ..
        } = &*BasePrimordials::get(&ctx)?;

        let controller = Self {
            // Set controller.[[stream]] to stream.
            stream: stream_class,

            // Set controller.[[pullAgain]] and controller.[[pulling]] to false.
            pull_again: false,
            pulling: false,

            // Set controller.[[byobRequest]] to null.
            byob_request: None,

            // Perform ! ResetQueue(controller).
            queue: VecDeque::new(),
            queue_total_size: 0,

            // Set controller.[[closeRequested]] and controller.[[started]] to false.
            close_requested: false,
            started: false,

            // Set controller.[[strategyHWM]] to highWaterMark.
            strategy_hwm: high_water_mark,

            // Set controller.[[pullAlgorithm]] to pullAlgorithm.
            pull_algorithm: Some(pull_algorithm),
            cancel_algorithm: Some(cancel_algorithm),

            // Set controller.[[autoAllocateChunkSize]] to autoAllocateChunkSize.
            auto_allocate_chunk_size,

            pending_pull_intos: VecDeque::new(),

            array_constructor_primordials,
            constructor_array_buffer: constructor_array_buffer.clone(),
            function_array_buffer_is_view: function_array_buffer_is_view.clone(),
        };

        let controller_class = Class::instance(ctx.clone(), controller)?;

        // Set stream.[[controller]] to controller.
        stream.controller =
            ReadableStreamControllerClass::ReadableStreamByteController(controller_class.clone());

        let objects =
            ReadableStreamObjects::new_byte(stream, OwnedBorrowMut::from_class(controller_class))
                .refresh_reader();

        let promise_primordials = objects.stream.promise_primordials.clone();

        // Let startResult be the result of performing startAlgorithm.
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
                        Self::readable_byte_stream_controller_call_pull_if_needed(ctx, objects)?;
                        Ok(())
                    },
                    // Upon rejection of startPromise with reason r,
                    Err(r) => {
                        // Perform ! ReadableByteStreamControllerError(controller, r).
                        Self::readable_byte_stream_controller_error(objects, r)?;
                        Ok(())
                    },
                }
            }
        })?;

        Ok(objects_class.controller)
    }

    fn readable_byte_stream_controller_call_pull_if_needed<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableByteStreamObjects<'js, R>,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // Let shouldPull be ! ReadableByteStreamControllerShouldCallPull(controller).
        let (should_pull, mut objects) =
            Self::readable_byte_stream_controller_should_call_pull(objects);

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

        upon_promise::<Value<'js>, ()>(ctx, pull_promise, {
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
                            // Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
                            Self::readable_byte_stream_controller_call_pull_if_needed(
                                ctx, objects,
                            )?;
                        };
                        Ok(())
                    },
                    // Upon rejection of pullPromise with reason e,
                    Err(e) => {
                        // Perform ! ReadableByteStreamControllerError(controller, e).
                        Self::readable_byte_stream_controller_error(objects, e)?;
                        Ok(())
                    },
                }
            }
        })?;

        Ok(ReadableStreamObjects::from_class(objects_class))
    }

    fn readable_byte_stream_controller_should_call_pull<R: ReadableStreamReader<'js>>(
        mut objects: ReadableByteStreamObjects<'js, R>,
    ) -> (bool, ReadableByteStreamObjects<'js, R>) {
        // Let stream be controller.[[stream]].
        match objects.stream.state {
            ReadableStreamState::Readable => {},
            // If stream.[[state]] is not "readable", return false.
            _ => return (false, objects),
        }

        // If controller.[[closeRequested]] is true, return false.
        if objects.controller.close_requested {
            return (false, objects);
        }

        // If controller.[[started]] is false, return false.
        if !objects.controller.started {
            return (false, objects);
        }

        let (mut has_read_requests, mut has_read_into_requests) = (false, false);
        objects = objects
            .with_reader(
                |objects| {
                    // If ! ReadableStreamHasDefaultReader(stream) is true and ! ReadableStreamGetNumReadRequests(stream) > 0, return true.
                    if ReadableStream::readable_stream_get_num_read_requests(&objects.reader) > 0 {
                        has_read_requests = true;
                    }
                    Ok(objects)
                },
                |objects| {
                    // If ! ReadableStreamHasBYOBReader(stream) is true and ! ReadableStreamGetNumReadIntoRequests(stream) > 0, return true.
                    if ReadableStream::readable_stream_get_num_read_into_requests(&objects.reader)
                        > 0
                    {
                        has_read_into_requests = true;
                    }
                    Ok(objects)
                },
                Ok,
            )
            .unwrap();

        if has_read_requests || has_read_into_requests {
            return (true, objects);
        }

        // Let desiredSize be ! ReadableByteStreamControllerGetDesiredSize(controller).
        let desired_size = objects
            .controller
            .readable_byte_stream_controller_get_desired_size(&objects.stream);

        // Assert: desiredSize is not null.
        if desired_size.0.expect("desired_size must not be null") > 0.0 {
            // If desiredSize > 0, return true.
            return (true, objects);
        }

        // Return false.
        (false, objects)
    }

    pub(super) fn readable_byte_stream_controller_error<R: ReadableStreamReader<'js>>(
        // Let stream be controller.[[stream]].
        mut objects: ReadableByteStreamObjects<'js, R>,
        e: Value<'js>,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // If stream.[[state]] is not "readable", return.
        if !matches!(objects.stream.state, ReadableStreamState::Readable) {
            return Ok(objects);
        };

        // Perform ! ReadableByteStreamControllerClearPendingPullIntos(controller).
        objects
            .controller
            .readable_byte_stream_controller_clear_pending_pull_intos();

        // Perform ! ResetQueue(controller).
        objects.controller.reset_queue();

        // Perform ! ReadableByteStreamControllerClearAlgorithms(controller).
        objects
            .controller
            .readable_byte_stream_controller_clear_algorithms();

        // Perform ! ReadableStreamError(stream, e).
        ReadableStream::readable_stream_error(objects, e)
    }

    fn readable_byte_stream_controller_clear_pending_pull_intos(&mut self) {
        // Perform ! ReadableByteStreamControllerInvalidateBYOBRequest(controller).
        self.readable_byte_stream_controller_invalidate_byob_request();

        // Set controller.[[pendingPullIntos]] to a new empty list.
        self.pending_pull_intos.clear();
    }

    fn readable_byte_stream_controller_invalidate_byob_request(&mut self) {
        let byob_request = match self.byob_request {
            // If controller.[[byobRequest]] is null, return.
            None => return,
            Some(ref byob_request) => byob_request.clone(),
        };
        let mut byob_request = OwnedBorrowMut::from_class(byob_request);
        byob_request.controller = None;
        byob_request.view = None;

        self.byob_request = None;
    }

    fn readable_byte_stream_controller_clear_algorithms(&mut self) {
        self.pull_algorithm = None;
        self.cancel_algorithm = None;
    }

    pub(super) fn readable_byte_stream_controller_get_byob_request(
        ctx: Ctx<'js>,
        controller: OwnedBorrowMut<'js, Self>,
    ) -> Result<(
        Null<Class<'js, ReadableStreamBYOBRequest<'js>>>,
        OwnedBorrowMut<'js, Self>,
    )> {
        // If controller.[[byobRequest]] is null and controller.[[pendingPullIntos]] is not empty,
        if controller.byob_request.is_none() && !controller.pending_pull_intos.is_empty() {
            // Let firstDescriptor be controller.[[pendingPullIntos]][0].
            let first_descriptor = &controller.pending_pull_intos[0];

            // Let view be ! Construct(%Uint8Array%, « firstDescriptor’s buffer, firstDescriptor’s byte offset + firstDescriptor’s bytes filled, firstDescriptor’s byte length − firstDescriptor’s bytes filled »).
            let view = ViewBytes::from_value(
                &ctx,
                &controller.function_array_buffer_is_view,
                Some(
                    &controller
                        .array_constructor_primordials
                        .constructor_uint8array
                        .construct((
                            first_descriptor.buffer.clone(),
                            first_descriptor.byte_offset + first_descriptor.bytes_filled,
                            first_descriptor.byte_length - first_descriptor.bytes_filled,
                        ))?,
                ),
            )?;

            let (controller_class, mut controller) = class_from_owned_borrow_mut(controller);

            // Let byobRequest be a new ReadableStreamBYOBRequest.
            let byob_request = ReadableStreamBYOBRequest {
                // Set byobRequest.[[controller]] to controller.
                controller: Some(controller_class),
                // Set byobRequest.[[view]] to view.
                view: Some(view),
            };

            // Set controller.[[byobRequest]] to byobRequest.
            controller.byob_request = Some(Class::instance(ctx, byob_request)?);

            Ok((Null(controller.byob_request.clone()), controller))
        } else {
            // Return controller.[[byobRequest]].
            Ok((Null(controller.byob_request.clone()), controller))
        }
    }

    fn readable_byte_stream_controller_get_desired_size(
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
            _ => Null(Some(self.strategy_hwm - self.queue_total_size as f64)),
        }
    }

    fn reset_queue(&mut self) {
        // Set container.[[queue]] to a new empty list.
        self.queue.clear();
        // Set container.[[queueTotalSize]] to 0.
        self.queue_total_size = 0;
    }

    pub(super) fn readable_byte_stream_controller_close<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: ReadableByteStreamObjects<'js, R>,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // If controller.[[closeRequested]] is true or stream.[[state]] is not "readable", return.
        if objects.controller.close_requested
            || !matches!(objects.stream.state, ReadableStreamState::Readable)
        {
            return Ok(objects);
        }

        // If controller.[[queueTotalSize]] > 0,
        if objects.controller.queue_total_size > 0 {
            // Set controller.[[closeRequested]] to true.
            objects.controller.close_requested = true;
            // Return.
            return Ok(objects);
        }

        // If controller.[[pendingPullIntos]] is not empty,
        // Let firstPendingPullInto be controller.[[pendingPullIntos]][0].
        if let Some(first_pending_pull_into) = objects.controller.pending_pull_intos.front() {
            // If the remainder after dividing firstPendingPullInto’s bytes filled by firstPendingPullInto’s element size is not 0,
            if first_pending_pull_into.bytes_filled % first_pending_pull_into.element_size != 0 {
                // Let e be a new TypeError exception.
                let e: Value = objects
                    .stream
                    .constructor_type_error
                    .call(("Insufficient bytes to fill elements in the given buffer",))?;
                Self::readable_byte_stream_controller_error(objects, e.clone())?;
                return Err(ctx.throw(e));
            }
        }

        // Perform ! ReadableByteStreamControllerClearAlgorithms(controller).
        objects
            .controller
            .readable_byte_stream_controller_clear_algorithms();

        // Perform ! ReadableStreamClose(stream).
        ReadableStream::readable_stream_close(ctx, objects)
    }

    pub(super) fn readable_byte_stream_controller_enqueue<R: ReadableStreamReader<'js>>(
        ctx: &Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: ReadableByteStreamObjects<'js, R>,
        chunk: ViewBytes<'js>,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // If controller.[[closeRequested]] is true or stream.[[state]] is not "readable", return.
        if objects.controller.close_requested
            || !matches!(objects.stream.state, ReadableStreamState::Readable)
        {
            return Ok(objects);
        };

        // Let buffer be chunk.[[ViewedArrayBuffer]].
        // Let byteOffset be chunk.[[ByteOffset]].
        // Let byteLength be chunk.[[ByteLength]].
        let (buffer, byte_length, byte_offset) = chunk.get_array_buffer()?;

        // If ! IsDetachedBuffer(buffer) is true, throw a TypeError exception.
        buffer.as_raw().ok_or(Exception::throw_type(
            ctx,
            "chunk's buffer is detached and so cannot be enqueued",
        ))?;

        // Let transferredBuffer be ? TransferArrayBuffer(buffer).
        let transferred_buffer = transfer_array_buffer(buffer)?;

        // If controller.[[pendingPullIntos]] is not empty,
        // Let firstPendingPullInto be controller.[[pendingPullIntos]][0].
        if !objects.controller.pending_pull_intos.is_empty() {
            // If ! IsDetachedBuffer(firstPendingPullInto’s buffer) is true, throw a TypeError exception.
            objects.controller.pending_pull_intos[0]
                    .buffer
                    .as_raw()
                    .or_throw_type(
                        ctx,
                        "The BYOB request's buffer has been detached and so cannot be filled with an enqueued chunk",
                    )?;

            // Perform ! ReadableByteStreamControllerInvalidateBYOBRequest(controller).
            objects
                .controller
                .readable_byte_stream_controller_invalidate_byob_request();

            // Set firstPendingPullInto’s buffer to ! TransferArrayBuffer(firstPendingPullInto’s buffer).
            objects.controller.pending_pull_intos[0].buffer =
                transfer_array_buffer(objects.controller.pending_pull_intos[0].buffer.clone())?;

            // If firstPendingPullInto’s reader type is "none", perform ? ReadableByteStreamControllerEnqueueDetachedPullIntoToQueue(controller, firstPendingPullInto).
            if let PullIntoDescriptorReaderType::None =
                objects.controller.pending_pull_intos[0].reader_type
            {
                objects = Self::readable_byte_stream_enqueue_detached_pull_into_to_queue(
                    ctx.clone(),
                    objects,
                    0,
                )?;
            }
        }

        objects = objects.with_reader(
            // If ! ReadableStreamHasDefaultReader(stream) is true,
            |mut objects| {
                // Perform ! ReadableByteStreamControllerProcessReadRequestsUsingQueue(controller).
                objects = Self::readable_byte_stream_controller_process_read_requests_using_queue(
                    objects, ctx,
                )?;

                // If ! ReadableStreamGetNumReadRequests(stream) is 0,
                if ReadableStream::readable_stream_get_num_read_requests(&objects.reader) == 0 {
                    // Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, transferredBuffer, byteOffset, byteLength).
                    objects
                        .controller
                        .readable_byte_stream_controller_enqueue_chunk_to_queue(
                            transferred_buffer.clone(),
                            byte_offset,
                            byte_length,
                        )
                } else {
                    // Otherwise,
                    // If controller.[[pendingPullIntos]] is not empty,
                    if !objects.controller.pending_pull_intos.is_empty() {
                        // Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
                        objects
                            .controller
                            .readable_byte_stream_controller_shift_pending_pull_into();
                    }

                    // Let transferredView be ! Construct(%Uint8Array%, « transferredBuffer, byteOffset, byteLength »).
                    let transferred_view = ViewBytes::from_value(
                        ctx,
                        &objects.controller.function_array_buffer_is_view,
                        Some(
                            &objects
                                .controller
                                .array_constructor_primordials
                                .constructor_uint8array
                                .construct((
                                    transferred_buffer.clone(),
                                    byte_offset,
                                    byte_length,
                                ))?,
                        ),
                    );

                    // Perform ! ReadableStreamFulfillReadRequest(stream, transferredView, false).
                    objects = ReadableStream::readable_stream_fulfill_read_request(
                        ctx,
                        objects,
                        transferred_view.into_js(ctx)?,
                        false,
                    )?;
                }

                Ok(objects)
            },
            |mut objects| {
                // Otherwise, if ! ReadableStreamHasBYOBReader(stream) is true,
                // Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, transferredBuffer, byteOffset, byteLength).
                objects
                    .controller
                    .readable_byte_stream_controller_enqueue_chunk_to_queue(
                        transferred_buffer.clone(),
                        byte_offset,
                        byte_length,
                    );
                // Perform ! ReadableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(controller).

                Self::readable_byte_stream_controller_process_pull_into_descriptors_using_queue(
                    ctx, objects,
                )
            },
            |mut objects| {
                // Otherwise,
                // Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, transferredBuffer, byteOffset, byteLength).
                objects
                    .controller
                    .readable_byte_stream_controller_enqueue_chunk_to_queue(
                        transferred_buffer.clone(),
                        byte_offset,
                        byte_length,
                    );

                Ok(objects)
            },
        )?;

        // Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
        Self::readable_byte_stream_controller_call_pull_if_needed(ctx.clone(), objects)
    }

    fn readable_byte_stream_enqueue_detached_pull_into_to_queue<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, R>,
        pull_into_descriptor_index: usize,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        let pull_into_descriptor =
            &objects.controller.pending_pull_intos[pull_into_descriptor_index];
        // If pullIntoDescriptor’s bytes filled > 0, perform ? ReadableByteStreamControllerEnqueueClonedChunkToQueue(controller, pullIntoDescriptor’s buffer, pullIntoDescriptor’s byte offset, pullIntoDescriptor’s bytes filled).
        if pull_into_descriptor.bytes_filled > 0 {
            let buffer = pull_into_descriptor.buffer.clone();
            let byte_offset = pull_into_descriptor.byte_offset;
            let bytes_filled = pull_into_descriptor.bytes_filled;
            objects = Self::readable_byte_stream_controller_enqueue_cloned_chunk_to_queue(
                ctx,
                objects,
                &buffer,
                byte_offset,
                bytes_filled,
            )?;
        }

        // Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
        objects
            .controller
            .readable_byte_stream_controller_shift_pending_pull_into();

        Ok(objects)
    }

    fn readable_byte_stream_controller_process_read_requests_using_queue(
        mut objects: ReadableStreamDefaultReaderObjects<'js, OwnedBorrowMut<'js, Self>>,
        ctx: &Ctx<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, OwnedBorrowMut<'js, Self>>> {
        // While reader.[[readRequests]] is not empty,
        while !objects.reader.read_requests.is_empty() {
            // If controller.[[queueTotalSize]] is 0, return.
            if objects.controller.queue_total_size == 0 {
                return Ok(objects);
            }

            // Let readRequest be reader.[[readRequests]][0].
            // Remove readRequest from reader.[[readRequests]].
            let read_request = objects.reader.read_requests.pop_front().unwrap();
            // Perform ! ReadableByteStreamControllerFillReadRequestFromQueue(controller, readRequest).
            objects = Self::readable_byte_stream_controller_fill_read_request_from_queue(
                ctx,
                objects,
                read_request,
            )?;
        }

        Ok(objects)
    }

    fn readable_byte_stream_controller_shift_pending_pull_into(
        &mut self,
    ) -> PullIntoDescriptor<'js> {
        // Let descriptor be controller.[[pendingPullIntos]][0].
        // Remove descriptor from controller.[[pendingPullIntos]].
        // Return descriptor.
        self.pending_pull_intos.pop_front().expect(
            "ReadableByteStreamControllerShiftPendingPullInto called on empty pendingPullIntos",
        )
    }

    fn readable_byte_stream_controller_enqueue_chunk_to_queue(
        &mut self,
        buffer: ArrayBuffer<'js>,
        byte_offset: usize,
        byte_length: usize,
    ) {
        let len = buffer.len();
        // Append a new readable byte stream queue entry with buffer buffer, byte offset byteOffset, and byte length byteLength to controller.[[queue]].
        self.queue.push_back(ReadableByteStreamQueueEntry {
            buffer,
            byte_offset,
            byte_length,
        });

        // Set controller.[[queueTotalSize]] to controller.[[queueTotalSize]] + byteLength.
        self.queue_total_size += len;
    }

    fn readable_byte_stream_controller_process_pull_into_descriptors_using_queue<
        R: ReadableStreamReader<'js>,
    >(
        ctx: &Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, R>,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // While controller.[[pendingPullIntos]] is not empty,
        while !objects.controller.pending_pull_intos.is_empty() {
            // If controller.[[queueTotalSize]] is 0, return.
            if objects.controller.queue_total_size == 0 {
                return Ok(objects);
            }

            // Let pullIntoDescriptor be controller.[[pendingPullIntos]][0].
            let mut pull_into_descriptor_ref = PullIntoDescriptorRefMut::Index(0);

            // If ! ReadableByteStreamControllerFillPullIntoDescriptorFromQueue(controller, pullIntoDescriptor) is true,
            if objects
                .controller
                .readable_byte_stream_controller_fill_pull_into_descriptor_from_queue(
                    ctx,
                    &mut pull_into_descriptor_ref,
                )?
            {
                // Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
                let pull_into_descriptor = objects
                    .controller
                    .readable_byte_stream_controller_shift_pending_pull_into();

                // Perform ! ReadableByteStreamControllerCommitPullIntoDescriptor(controller.[[stream]], pullIntoDescriptor).
                objects = Self::readable_byte_stream_controller_commit_pull_into_descriptor(
                    ctx.clone(),
                    objects,
                    pull_into_descriptor,
                )?;
            }
        }
        Ok(objects)
    }

    fn readable_byte_stream_controller_enqueue_cloned_chunk_to_queue<
        R: ReadableStreamReader<'js>,
    >(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, R>,
        buffer: &ArrayBuffer<'js>,
        byte_offset: usize,
        byte_length: usize,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // Let cloneResult be CloneArrayBuffer(buffer, byteOffset, byteLength, %ArrayBuffer%).
        let clone_result = match ArrayBuffer::new_copy(
            ctx.clone(),
            &buffer.as_bytes().expect(
                "ReadableByteStreamControllerEnqueueClonedChunkToQueue called on detached buffer",
            )[byte_offset..byte_offset + byte_length],
        ) {
            Ok(clone_result) => clone_result,
            Err(Error::Exception) => {
                let err = ctx.catch();
                Self::readable_byte_stream_controller_error(objects, err.clone())?;
                return Err(ctx.throw(err));
            },
            Err(err) => return Err(err),
        };

        // Perform ! ReadableByteStreamControllerEnqueueChunkToQueue(controller, cloneResult.[[Value]], 0, byteLength).
        objects
            .controller
            .readable_byte_stream_controller_enqueue_chunk_to_queue(clone_result, 0, byte_length);

        Ok(objects)
    }

    fn readable_byte_stream_controller_fill_read_request_from_queue(
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamDefaultReaderObjects<'js, OwnedBorrowMut<'js, Self>>,
        read_request: impl ReadableStreamReadRequest<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, OwnedBorrowMut<'js, Self>>> {
        let entry = {
            // Assert: controller.[[queueTotalSize]] > 0.
            // Let entry be controller.[[queue]][0].
            // Remove entry from controller.[[queue]].
            let entry = objects.controller.queue.pop_front().expect(
                "ReadableByteStreamControllerFillReadRequestFromQueue called with empty queue",
            );

            // Set controller.[[queueTotalSize]] to controller.[[queueTotalSize]] − entry’s byte length.
            objects.controller.queue_total_size -= entry.byte_length;

            entry
        };

        // Perform ! ReadableByteStreamControllerHandleQueueDrain(controller).
        objects = Self::readable_byte_stream_controller_handle_queue_drain(ctx.clone(), objects)?;

        // Let view be ! Construct(%Uint8Array%, « entry’s buffer, entry’s byte offset, entry’s byte length »).
        let view: TypedArray<u8> = objects
            .controller
            .array_constructor_primordials
            .constructor_uint8array
            .construct((entry.buffer, entry.byte_offset, entry.byte_length))?;

        // Perform readRequest’s chunk steps, given view.
        read_request.chunk_steps_typed(objects, view.into_value())
    }

    fn readable_byte_stream_controller_fill_pull_into_descriptor_from_queue<'a>(
        &'a mut self,
        ctx: &Ctx<'js>,
        pull_into_descriptor_ref: &mut PullIntoDescriptorRefMut<'js, 'a>,
    ) -> Result<bool> {
        let (mut total_bytes_to_copy_remaining, ready) = {
            let pull_into_descriptor = match pull_into_descriptor_ref {
                PullIntoDescriptorRefMut::Index(i) => &mut self.pending_pull_intos[*i],
                PullIntoDescriptorRefMut::Owned(r) => r,
            };
            // Let maxBytesToCopy be min(controller.[[queueTotalSize]], pullIntoDescriptor’s byte length − pullIntoDescriptor’s bytes filled).
            let max_bytes_to_copy: usize = std::cmp::min(
                self.queue_total_size,
                pull_into_descriptor.byte_length - pull_into_descriptor.bytes_filled,
            );

            // Let maxBytesFilled be pullIntoDescriptor’s bytes filled + maxBytesToCopy.
            let max_bytes_filled = pull_into_descriptor.bytes_filled + max_bytes_to_copy;

            // Let totalBytesToCopyRemaining be maxBytesToCopy.
            let mut total_bytes_to_copy_remaining = max_bytes_to_copy;

            // Let ready be false.
            let mut ready = false;

            // Let remainderBytes be the remainder after dividing maxBytesFilled by pullIntoDescriptor’s element size.
            let remainder_bytes = max_bytes_filled % pull_into_descriptor.element_size;

            // Let maxAlignedBytes be maxBytesFilled − remainderBytes.
            let max_aligned_bytes = max_bytes_filled - remainder_bytes;

            // If maxAlignedBytes ≥ pullIntoDescriptor’s minimum fill,
            if max_aligned_bytes >= pull_into_descriptor.minimum_fill {
                // Set totalBytesToCopyRemaining to maxAlignedBytes − pullIntoDescriptor’s bytes filled.
                total_bytes_to_copy_remaining =
                    max_aligned_bytes - pull_into_descriptor.bytes_filled;
                // Set ready to true.
                ready = true
            }

            (total_bytes_to_copy_remaining, ready)
        };

        // Let queue be controller.[[queue]].
        // While totalBytesToCopyRemaining > 0,
        while total_bytes_to_copy_remaining > 0 {
            let bytes_to_copy = {
                let pull_into_descriptor = match pull_into_descriptor_ref {
                    PullIntoDescriptorRefMut::Index(i) => &mut self.pending_pull_intos[*i],
                    PullIntoDescriptorRefMut::Owned(r) => r,
                };

                // Let headOfQueue be queue[0].
                let head_of_queue = self
                    .queue
                    .front_mut()
                    .expect("empty queue with bytes to copy");
                // Let bytesToCopy be min(totalBytesToCopyRemaining, headOfQueue’s byte length).
                let bytes_to_copy: usize =
                    std::cmp::min(total_bytes_to_copy_remaining, head_of_queue.byte_length);
                // Let destStart be pullIntoDescriptor’s byte offset + pullIntoDescriptor’s bytes filled.
                let dest_start: usize =
                    pull_into_descriptor.byte_offset + pull_into_descriptor.bytes_filled;
                // Perform ! CopyDataBlockBytes(pullIntoDescriptor’s buffer.[[ArrayBufferData]], destStart, headOfQueue’s buffer.[[ArrayBufferData]], headOfQueue’s byte offset, bytesToCopy).
                copy_data_block_bytes(
                    ctx,
                    &pull_into_descriptor.buffer,
                    dest_start,
                    &head_of_queue.buffer,
                    head_of_queue.byte_offset,
                    bytes_to_copy,
                )?;
                if head_of_queue.byte_length == bytes_to_copy {
                    // If headOfQueue’s byte length is bytesToCopy,
                    // Remove queue[0].
                    self.queue.pop_front();
                } else {
                    // Otherwise,
                    // Set headOfQueue’s byte offset to headOfQueue’s byte offset + bytesToCopy.
                    head_of_queue.byte_offset += bytes_to_copy;
                    // Set headOfQueue’s byte length to headOfQueue’s byte length − bytesToCopy.
                    head_of_queue.byte_length -= bytes_to_copy
                }

                // Set controller.[[queueTotalSize]] to controller.[[queueTotalSize]] − bytesToCopy.
                self.queue_total_size -= bytes_to_copy;

                bytes_to_copy
            };

            // Perform ! ReadableByteStreamControllerFillHeadPullIntoDescriptor(controller, bytesToCopy, pullIntoDescriptor).
            self.readable_byte_stream_controller_fill_head_pull_into_descriptor(
                bytes_to_copy,
                pull_into_descriptor_ref,
            );

            // Set totalBytesToCopyRemaining to totalBytesToCopyRemaining − bytesToCopy.
            total_bytes_to_copy_remaining -= bytes_to_copy
        }

        Ok(ready)
    }

    fn readable_byte_stream_controller_commit_pull_into_descriptor<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableByteStreamObjects<'js, R>,
        pull_into_descriptor: PullIntoDescriptor<'js>,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // Let done be false.
        let mut done = false;
        // If stream.[[state]] is "closed",
        if matches!(objects.stream.state, ReadableStreamState::Closed) {
            // Set done to true.
            done = true
        }

        let reader_type = pull_into_descriptor.reader_type;

        // Let filledView be ! ReadableByteStreamControllerConvertPullIntoDescriptor(pullIntoDescriptor).
        let filled_view = Self::readable_byte_stream_controller_convert_pull_into_descriptor(
            ctx.clone(),
            &objects.stream.function_array_buffer_is_view,
            pull_into_descriptor,
        )?;

        if let PullIntoDescriptorReaderType::Default = reader_type {
            // If pullIntoDescriptor’s reader type is "default",
            objects.with_assert_default_reader(|objects| {
                // Perform ! ReadableStreamFulfillReadRequest(stream, filledView, done).
                ReadableStream::readable_stream_fulfill_read_request(
                    &ctx,
                    objects,
                    filled_view.into_js(&ctx)?,
                    done,
                )
            })
        } else {
            // Otherwise,
            objects.with_assert_byob_reader(|objects| {
                // Perform ! ReadableStreamFulfillReadIntoRequest(stream, filledView, done).
                ReadableStream::readable_stream_fulfill_read_into_request(
                    &ctx,
                    objects,
                    filled_view,
                    done,
                )
            })
        }
    }

    fn readable_byte_stream_controller_handle_queue_drain<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, R>,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // If controller.[[queueTotalSize]] is 0 and controller.[[closeRequested]] is true,
        if objects.controller.queue_total_size == 0 && objects.controller.close_requested {
            // Perform ! ReadableByteStreamControllerClearAlgorithms(controller).
            objects
                .controller
                .readable_byte_stream_controller_clear_algorithms();
            // Perform ! ReadableStreamClose(controller.[[stream]]).
            ReadableStream::readable_stream_close(ctx, objects)
        } else {
            // Otherwise,
            // Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
            Self::readable_byte_stream_controller_call_pull_if_needed(ctx.clone(), objects)
        }
    }

    fn readable_byte_stream_controller_convert_pull_into_descriptor(
        ctx: Ctx<'js>,
        function_array_buffer_is_view: &Function<'js>,
        pull_into_descriptor: PullIntoDescriptor<'js>,
    ) -> Result<ViewBytes<'js>> {
        let PullIntoDescriptor {
            // Let bytesFilled be pullIntoDescriptor’s bytes filled.
            bytes_filled,
            // Let elementSize be pullIntoDescriptor’s element size.
            element_size,
            byte_offset,
            buffer,
            ..
        } = pull_into_descriptor;
        // Let buffer be ! TransferArrayBuffer(pullIntoDescriptor’s buffer).
        let buffer = transfer_array_buffer(buffer);
        // Return ! Construct(pullIntoDescriptor’s view constructor, « buffer, pullIntoDescriptor’s byte offset, bytesFilled ÷ elementSize »).
        let view: Object = pull_into_descriptor.view_constructor.construct((
            buffer,
            byte_offset,
            bytes_filled / element_size,
        ))?;
        ViewBytes::from_object(&ctx, function_array_buffer_is_view, &view)
    }

    pub(super) fn readable_byte_stream_controller_pull_into(
        ctx: &Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: ReadableStreamBYOBObjects<'js>,
        view: ViewBytes<'js>,
        min: u64,
        read_into_request: impl ReadableStreamReadIntoRequest<'js> + 'js,
    ) -> Result<ReadableStreamBYOBObjects<'js>> {
        // Set elementSize to the element size specified in the typed array constructors table for view.[[TypedArrayName]].
        // Set ctor to the constructor specified in the typed array constructors table for view.[[TypedArrayName]].
        let (element_size, ctor) = (
            view.element_size(),
            objects
                .controller
                .array_constructor_primordials
                .for_view_bytes(&view),
        );

        // Let minimumFill be min × elementSize.
        let minimum_fill: usize = (min as usize) * element_size;

        // Let byteOffset be view.[[ByteOffset]].
        // Let byteLength be view.[[ByteLength]].
        let (buffer, byte_length, byte_offset) = view.get_array_buffer()?;

        // Let bufferResult be TransferArrayBuffer(view.[[ViewedArrayBuffer]]).
        let buffer_result = transfer_array_buffer(buffer);
        let buffer = match buffer_result {
            // If bufferResult is an abrupt completion,
            Err(Error::Exception) => {
                // Perform readIntoRequest’s error steps, given bufferResult.[[Value]].
                objects = read_into_request.error_steps(objects, ctx.catch())?;
                // Return.
                return Ok(objects);
            },
            Err(err) => return Err(err),
            // Let buffer be bufferResult.[[Value]].
            Ok(buffer) => buffer,
        };

        let buffer_byte_length = buffer.len();
        // Let pullIntoDescriptor be a new pull-into descriptor with
        let mut pull_into_descriptor = PullIntoDescriptor {
            buffer,
            buffer_byte_length,
            byte_offset,
            byte_length,
            bytes_filled: 0,
            minimum_fill,
            element_size,
            view_constructor: ctor.clone(),
            reader_type: PullIntoDescriptorReaderType::Byob,
        };

        // If controller.[[pendingPullIntos]] is not empty,
        if !objects.controller.pending_pull_intos.is_empty() {
            // Append pullIntoDescriptor to controller.[[pendingPullIntos]].
            objects
                .controller
                .pending_pull_intos
                .push_back(pull_into_descriptor);

            // Perform ! ReadableStreamAddReadIntoRequest(stream, readIntoRequest).
            ReadableStream::readable_stream_add_read_into_request(
                &mut objects.reader,
                read_into_request,
            );

            // Return.
            return Ok(objects);
        }

        // If stream.[[state]] is "closed",
        if matches!(objects.stream.state, ReadableStreamState::Closed) {
            // Let emptyView be ! Construct(ctor, « pullIntoDescriptor’s buffer, pullIntoDescriptor’s byte offset, 0 »).
            let empty_view: Value<'js> = ctor.construct((
                pull_into_descriptor.buffer,
                pull_into_descriptor.byte_offset,
                0,
            ))?;

            // Perform readIntoRequest’s close steps, given emptyView.
            objects = read_into_request.close_steps(objects, empty_view)?;

            // Return.
            return Ok(objects);
        }

        // If controller.[[queueTotalSize]] > 0,
        if objects.controller.queue_total_size > 0 {
            // If ! ReadableByteStreamControllerFillPullIntoDescriptorFromQueue(controller, pullIntoDescriptor) is true,
            if objects
                .controller
                .readable_byte_stream_controller_fill_pull_into_descriptor_from_queue(
                    ctx,
                    &mut PullIntoDescriptorRefMut::Owned(&mut pull_into_descriptor),
                )?
            {
                // Let filledView be ! ReadableByteStreamControllerConvertPullIntoDescriptor(pullIntoDescriptor).
                let filled_view = objects
                    .controller
                    .readable_byte_steam_controller_convert_pull_into_descriptor(
                        pull_into_descriptor,
                    )?;

                // Perform ! ReadableByteStreamControllerHandleQueueDrain(controller).
                objects =
                    Self::readable_byte_stream_controller_handle_queue_drain(ctx.clone(), objects)?;

                // Perform readIntoRequest’s chunk steps, given filledView.
                // Return.
                return read_into_request.chunk_steps(objects, filled_view);
            }

            // If controller.[[closeRequested]] is true,
            if objects.controller.close_requested {
                // Let e be a TypeError exception.
                let e: Value = objects
                    .stream
                    .constructor_type_error
                    .call(("Insufficient bytes to fill elements in the given buffer",))?;

                // Perform ! ReadableByteStreamControllerError(controller, e).
                objects = Self::readable_byte_stream_controller_error(objects, e.clone())?;

                // Perform readIntoRequest’s error steps, given e.
                // Return.
                return read_into_request.error_steps(objects, e);
            }
        }

        // Append pullIntoDescriptor to controller.[[pendingPullIntos]].
        objects
            .controller
            .pending_pull_intos
            .push_back(pull_into_descriptor);

        // Perform ! ReadableStreamAddReadIntoRequest(stream, readIntoRequest).
        ReadableStream::readable_stream_add_read_into_request(
            &mut objects.reader,
            read_into_request,
        );

        // Perform ! ReadableByteStreamControllerCallPullIfNeeded(controller).
        Self::readable_byte_stream_controller_call_pull_if_needed(ctx.clone(), objects)
    }

    fn readable_byte_steam_controller_convert_pull_into_descriptor(
        &mut self,
        pull_into_descriptor: PullIntoDescriptor<'js>,
    ) -> Result<Value<'js>> {
        // Let bytesFilled be pullIntoDescriptor’s bytes filled.
        let bytes_filled = pull_into_descriptor.bytes_filled;

        // Let elementSize be pullIntoDescriptor’s element size.
        let element_size = pull_into_descriptor.element_size;

        // Let buffer be ! TransferArrayBuffer(pullIntoDescriptor’s buffer).
        let buffer = transfer_array_buffer(pull_into_descriptor.buffer)?;

        // Return ! Construct(pullIntoDescriptor’s view constructor, « buffer, pullIntoDescriptor’s byte offset, bytesFilled ÷ elementSize »).
        pull_into_descriptor.view_constructor.construct((
            buffer,
            pull_into_descriptor.byte_offset,
            bytes_filled / element_size,
        ))
    }

    pub(super) fn readable_byte_stream_controller_respond<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, R>,
        bytes_written: usize,
    ) -> Result<()> {
        // Let firstDescriptor be controller.[[pendingPullIntos]][0].
        let first_descriptor = &mut objects.controller.pending_pull_intos[0];

        // Let state be controller.[[stream]].[[state]].
        match objects.stream.state {
            // If state is "closed",
            ReadableStreamState::Closed => {
                // If bytesWritten is not 0, throw a TypeError exception.
                if bytes_written != 0 {
                    return Err(Exception::throw_type(
                        &ctx,
                        "bytesWritten must be 0 when calling respond() on a closed stream",
                    ));
                }
            },
            // Otherwise,
            _ => {
                // If bytesWritten is 0, throw a TypeError exception.
                if bytes_written == 0 {
                    return Err(Exception::throw_type(
                        &ctx,
                        "bytesWritten must be greater than 0 when calling respond() on a readable stream",
                    ));
                }

                // If firstDescriptor’s bytes filled + bytesWritten > firstDescriptor’s byte length, throw a RangeError exception.
                if first_descriptor.bytes_filled + bytes_written > first_descriptor.byte_length {
                    return Err(Exception::throw_range(&ctx, "bytesWritten out of range'"));
                }
            },
        };

        // Set firstDescriptor’s buffer to ! TransferArrayBuffer(firstDescriptor’s buffer).
        first_descriptor.buffer = transfer_array_buffer(first_descriptor.buffer.clone())?;

        // Perform ? ReadableByteStreamControllerRespondInternal(controller, bytesWritten).
        Self::readable_byte_stream_controller_respond_internal(ctx, objects, bytes_written)
    }

    fn readable_byte_stream_controller_respond_internal<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, R>,
        bytes_written: usize,
    ) -> Result<()> {
        // Let firstDescriptor be controller.[[pendingPullIntos]][0].
        let first_descriptor_index = 0;

        // Perform ! ReadableByteStreamControllerInvalidateBYOBRequest(controller).
        objects
            .controller
            .readable_byte_stream_controller_invalidate_byob_request();

        // Let state be controller.[[stream]].[[state]].
        match objects.stream.state {
            // If state is "closed",
            ReadableStreamState::Closed => {
                // Perform ! ReadableByteStreamControllerRespondInClosedState(controller, firstDescriptor).
                objects = Self::readable_byte_stream_controller_respond_in_closed_state(
                    ctx.clone(),
                    objects,
                    first_descriptor_index,
                )?;
            },
            // Otherwise
            _ => {
                // Perform ? ReadableByteStreamControllerRespondInReadableState(controller, bytesWritten, firstDescriptor).
                objects = Self::readable_byte_stream_controller_respond_in_readable_state(
                    ctx.clone(),
                    objects,
                    bytes_written,
                    first_descriptor_index,
                )?
            },
        };

        _ = Self::readable_byte_stream_controller_call_pull_if_needed(ctx, objects)?;
        Ok(())
    }

    fn readable_byte_stream_controller_respond_in_closed_state<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: ReadableByteStreamObjects<'js, R>,
        first_descriptor_index: usize,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // If firstDescriptor’s reader type is "none", perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
        if let PullIntoDescriptorReaderType::None =
            objects.controller.pending_pull_intos[first_descriptor_index].reader_type
        {
            objects
                .controller
                .readable_byte_stream_controller_shift_pending_pull_into();
        }

        // If ! ReadableStreamHasBYOBReader(stream) is true,
        objects.with_reader(
            Ok,
            |mut objects| {
                // While ! ReadableStreamGetNumReadIntoRequests(stream) > 0,
                while ReadableStream::readable_stream_get_num_read_into_requests(&objects.reader)
                    > 0
                {
                    // Let pullIntoDescriptor be ! ReadableByteStreamControllerShiftPendingPullInto(controller).
                    let pull_into_descriptor = objects
                        .controller
                        .readable_byte_stream_controller_shift_pending_pull_into();

                    // Perform ! ReadableByteStreamControllerCommitPullIntoDescriptor(stream, pullIntoDescriptor).
                    objects = Self::readable_byte_stream_controller_commit_pull_into_descriptor(
                        ctx.clone(),
                        objects,
                        pull_into_descriptor,
                    )?;
                }

                Ok(objects)
            },
            Ok,
        )
    }

    fn readable_byte_stream_controller_respond_in_readable_state<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        // Let stream be controller.[[stream]].
        mut objects: ReadableByteStreamObjects<'js, R>,
        bytes_written: usize,
        pull_into_descriptor_index: usize,
    ) -> Result<ReadableByteStreamObjects<'js, R>> {
        // Perform ! ReadableByteStreamControllerFillHeadPullIntoDescriptor(controller, bytesWritten, pullIntoDescriptor).
        objects
            .controller
            .readable_byte_stream_controller_fill_head_pull_into_descriptor(
                bytes_written,
                &mut PullIntoDescriptorRefMut::Index(pull_into_descriptor_index),
            );

        // If pullIntoDescriptor’s reader type is "none",
        if let PullIntoDescriptorReaderType::None =
            objects.controller.pending_pull_intos[pull_into_descriptor_index].reader_type
        {
            // Perform ? ReadableByteStreamControllerEnqueueDetachedPullIntoToQueue(controller, pullIntoDescriptor).
            objects = Self::readable_byte_stream_enqueue_detached_pull_into_to_queue(
                ctx.clone(),
                objects,
                pull_into_descriptor_index,
            )?;
            // Perform ! ReadableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(controller).
            // Return.
            return Self::readable_byte_stream_controller_process_pull_into_descriptors_using_queue(
                &ctx, objects,
            );
        }

        // If pullIntoDescriptor’s bytes filled < pullIntoDescriptor’s minimum fill, return.
        if objects.controller.pending_pull_intos[pull_into_descriptor_index].bytes_filled
            < objects.controller.pending_pull_intos[pull_into_descriptor_index].minimum_fill
        {
            return Ok(objects);
        }

        // Perform ! ReadableByteStreamControllerShiftPendingPullInto(controller).
        let mut pull_into_descriptor = objects
            .controller
            .readable_byte_stream_controller_shift_pending_pull_into();

        // Let remainderSize be the remainder after dividing pullIntoDescriptor’s bytes filled by pullIntoDescriptor’s element size.
        let remainder_size = pull_into_descriptor.bytes_filled % pull_into_descriptor.element_size;

        // If remainderSize > 0,
        if remainder_size > 0 {
            // Let end be pullIntoDescriptor’s byte offset + pullIntoDescriptor’s bytes filled.
            let end = pull_into_descriptor.byte_offset + pull_into_descriptor.bytes_filled;

            let buffer = pull_into_descriptor.buffer.clone();

            // Perform ? ReadableByteStreamControllerEnqueueClonedChunkToQueue(controller, pullIntoDescriptor’s buffer, end − remainderSize, remainderSize).
            objects = Self::readable_byte_stream_controller_enqueue_cloned_chunk_to_queue(
                ctx.clone(),
                objects,
                &buffer,
                end - remainder_size,
                remainder_size,
            )?;
        }

        // Set pullIntoDescriptor’s bytes filled to pullIntoDescriptor’s bytes filled − remainderSize.
        pull_into_descriptor.bytes_filled -= remainder_size;

        // Perform ! ReadableByteStreamControllerCommitPullIntoDescriptor(controller.[[stream]], pullIntoDescriptor).
        objects = Self::readable_byte_stream_controller_commit_pull_into_descriptor(
            ctx.clone(),
            objects,
            pull_into_descriptor,
        )?;

        // Perform ! ReadableByteStreamControllerProcessPullIntoDescriptorsUsingQueue(controller).
        Self::readable_byte_stream_controller_process_pull_into_descriptors_using_queue(
            &ctx, objects,
        )
    }

    pub(super) fn readable_byte_stream_controller_respond_with_new_view<
        R: ReadableStreamReader<'js>,
    >(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, R>,
        view: ViewBytes<'js>,
    ) -> Result<()> {
        // Let firstDescriptor be controller.[[pendingPullIntos]][0].
        let first_descriptor_index = 0;

        let (buffer, byte_length, byte_offset) = view.get_array_buffer()?;

        // Let state be controller.[[stream]].[[state]].
        match objects.stream.state {
            // If state is "closed",
            ReadableStreamState::Closed => {
                // If view.[[ByteLength]] is not 0, throw a TypeError exception.
                if byte_length != 0 {
                    return Err(Exception::throw_type(&ctx, "The view's length must be 0 when calling respondWithNewView() on a closed stream"));
                }
            },
            // Otherwise
            _ => {
                // If view.[[ByteLength]] is 0, throw a TypeError exception.
                if byte_length == 0 {
                    return Err(Exception::throw_type(&ctx, "The view's length must be greater than 0 when calling respondWithNewView() on a readable stream"));
                }
            },
        };

        {
            let first_descriptor =
                &mut objects.controller.pending_pull_intos[first_descriptor_index];

            // If firstDescriptor’s byte offset + firstDescriptor’ bytes filled is not view.[[ByteOffset]], throw a RangeError exception.
            if first_descriptor.byte_offset + first_descriptor.bytes_filled != byte_offset {
                return Err(Exception::throw_range(
                    &ctx,
                    "The region specified by view does not match byobRequest",
                ));
            };

            // If firstDescriptor’s buffer byte length is not view.[[ViewedArrayBuffer]].[[ByteLength]], throw a RangeError exception.
            if first_descriptor.buffer_byte_length != buffer.len() {
                return Err(Exception::throw_range(
                    &ctx,
                    "The buffer of view has different capacity than byobRequest",
                ));
            };

            // If firstDescriptor’s bytes filled + view.[[ByteLength]] > firstDescriptor’s byte length, throw a RangeError exception.
            if first_descriptor.bytes_filled + byte_length > first_descriptor.byte_length {
                return Err(Exception::throw_range(
                    &ctx,
                    "The region specified by view is larger than byobRequest",
                ));
            }

            // Set firstDescriptor’s buffer to ? TransferArrayBuffer(view.[[ViewedArrayBuffer]]).
            first_descriptor.buffer = transfer_array_buffer(buffer)?;
        }

        // Perform ? ReadableByteStreamControllerRespondInternal(controller, viewByteLength).
        Self::readable_byte_stream_controller_respond_internal(ctx, objects, byte_length)
    }

    fn readable_byte_stream_controller_fill_head_pull_into_descriptor<'a>(
        &mut self,
        size: usize,
        pull_into_descriptor_ref: &mut PullIntoDescriptorRefMut<'js, 'a>,
    ) {
        let pull_into_descriptor = match pull_into_descriptor_ref {
            PullIntoDescriptorRefMut::Index(i) => &mut self.pending_pull_intos[*i],
            PullIntoDescriptorRefMut::Owned(r) => *r,
        };

        // Set pullIntoDescriptor’s bytes filled to bytes filled + size.
        pull_into_descriptor.bytes_filled += size;
    }

    fn start_algorithm<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableByteStreamObjects<'js, R>,
        start_algorithm: StartAlgorithm<'js>,
    ) -> Result<(
        Value<'js>,
        ReadableStreamClassObjects<'js, OwnedBorrowMut<'js, Self>, R>,
    )> {
        let objects_class = objects.into_inner();

        Ok((
            start_algorithm.call(
                ctx,
                ReadableStreamControllerClass::ReadableStreamByteController(
                    objects_class.controller.clone(),
                ),
            )?,
            objects_class,
        ))
    }

    fn pull_algorithm<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableByteStreamObjects<'js, R>,
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
                ReadableStreamControllerClass::ReadableStreamByteController(
                    objects_class.controller.clone(),
                ),
            )?,
            objects_class,
        ))
    }

    fn cancel_algorithm<R: ReadableStreamReader<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableByteStreamObjects<'js, R>,
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
impl<'js> ReadableByteStreamController<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    // readonly attribute unrestricted double? desiredSize;
    #[qjs(get)]
    fn byob_request(
        ctx: Ctx<'js>,
        controller: This<OwnedBorrowMut<'js, Self>>,
    ) -> Result<Null<Class<'js, ReadableStreamBYOBRequest<'js>>>> {
        let (request, _) =
            Self::readable_byte_stream_controller_get_byob_request(ctx, controller.0)?;
        Ok(request)
    }

    // readonly attribute unrestricted double? desiredSize;
    #[qjs(get)]
    fn desired_size(&self) -> Null<f64> {
        let stream = OwnedBorrow::from_class(self.stream.clone());
        self.readable_byte_stream_controller_get_desired_size(&stream)
    }

    // undefined close();
    fn close(ctx: Ctx<'js>, controller: This<OwnedBorrowMut<'js, Self>>) -> Result<()> {
        // If this.[[closeRequested]] is true, throw a TypeError exception.
        if controller.close_requested {
            return Err(Exception::throw_type(&ctx, "close() called more than once"));
        }

        let objects = ReadableStreamObjects::from_byte_controller(controller.0).refresh_reader();

        if !matches!(objects.stream.state, ReadableStreamState::Readable) {
            return Err(Exception::throw_type(
                &ctx,
                "close() called when stream is not readable",
            ));
        };

        // Perform ? ReadableByteStreamControllerClose(this).
        Self::readable_byte_stream_controller_close(ctx, objects)?;
        Ok(())
    }

    // undefined enqueue(ArrayBufferView chunk);
    fn enqueue(
        this: This<OwnedBorrowMut<'js, Self>>,
        ctx: Ctx<'js>,
        chunk: Value<'js>,
    ) -> Result<()> {
        let chunk = ViewBytes::from_value(&ctx, &this.function_array_buffer_is_view, Some(&chunk))?;

        let (array_buffer, byte_length, _) = chunk.get_array_buffer()?;

        // If chunk.[[ByteLength]] is 0, throw a TypeError exception.
        if byte_length == 0 {
            return Err(Exception::throw_type(
                &ctx,
                "chunk must have non-zero byteLength",
            ));
        }

        // If chunk.[[ViewedArrayBuffer]].[[ArrayBufferByteLength]] is 0, throw a TypeError exception.
        if array_buffer.is_empty() {
            return Err(Exception::throw_type(
                &ctx,
                "chunk must have non-zero buffer byteLength",
            ));
        }

        // If this.[[closeRequested]] is true, throw a TypeError exception.
        if this.close_requested {
            return Err(Exception::throw_type(&ctx, "stream is closed or draining"));
        }

        let objects = ReadableStreamObjects::from_byte_controller(this.0).refresh_reader();

        // If this.[[stream]].[[state]] is not "readable", throw a TypeError exception.
        if !matches!(objects.stream.state, ReadableStreamState::Readable) {
            return Err(Exception::throw_type(
                &ctx,
                "The stream is not in the readable state and cannot be enqueued to",
            ));
        };

        // Return ? ReadableByteStreamControllerEnqueue(this, chunk).
        Self::readable_byte_stream_controller_enqueue(&ctx, objects, chunk)?;
        Ok(())
    }

    // undefined error(optional any e);
    fn error(
        ctx: Ctx<'js>,
        controller: This<OwnedBorrowMut<'js, Self>>,
        e: Opt<Value<'js>>,
    ) -> Result<()> {
        let objects = ReadableStreamObjects::from_byte_controller(controller.0).refresh_reader();

        // Perform ! ReadableByteStreamControllerError(this, e).
        Self::readable_byte_stream_controller_error(objects, e.0.unwrap_or_undefined(&ctx))?;
        Ok(())
    }
}

impl<'js> ReadableStreamController<'js> for ReadableByteStreamControllerOwned<'js> {
    type Class = ReadableByteStreamControllerClass<'js>;

    fn with_controller<C, O>(
        self,
        ctx: C,
        _: impl FnOnce(
            C,
            ReadableStreamDefaultControllerOwned<'js>,
        ) -> Result<(O, ReadableStreamDefaultControllerOwned<'js>)>,
        byte: impl FnOnce(
            C,
            ReadableByteStreamControllerOwned<'js>,
        ) -> Result<(O, ReadableByteStreamControllerOwned<'js>)>,
    ) -> Result<(O, Self)> {
        let (ctx, reader) = byte(ctx, self)?;
        Ok((ctx, reader))
    }

    fn into_inner(self) -> Self::Class {
        OwnedBorrowMut::into_inner(self)
    }

    fn from_class(class: Self::Class) -> Self {
        OwnedBorrowMut::from_class(class)
    }

    fn into_erased(self) -> ReadableStreamControllerOwned<'js> {
        ReadableStreamControllerOwned::ReadableStreamByteController(self)
    }

    fn try_from_erased(erased: ReadableStreamControllerOwned<'js>) -> Option<Self> {
        match erased {
            ReadableStreamControllerOwned::ReadableStreamDefaultController(_) => None,
            ReadableStreamControllerOwned::ReadableStreamByteController(r) => Some(r),
        }
    }

    fn pull_steps(
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamDefaultReaderObjects<'js, Self>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, Self>> {
        // If this.[[queueTotalSize]] > 0,
        if objects.controller.queue_total_size > 0 {
            // Perform ! ReadableByteStreamControllerFillReadRequestFromQueue(this, readRequest).
            // Return.
            return ReadableByteStreamController::readable_byte_stream_controller_fill_read_request_from_queue(
                ctx,
                objects,
                read_request,
            );
        }

        // Let autoAllocateChunkSize be this.[[autoAllocateChunkSize]].
        let auto_allocate_chunk_size = objects.controller.auto_allocate_chunk_size;

        // If autoAllocateChunkSize is not undefined,
        if let Some(auto_allocate_chunk_size) = auto_allocate_chunk_size {
            // Let buffer be Construct(%ArrayBuffer%, « autoAllocateChunkSize »).
            let buffer: ArrayBuffer = match objects
                .controller
                .constructor_array_buffer
                .construct((auto_allocate_chunk_size,))
            {
                // If buffer is an abrupt completion,
                Err(Error::Exception) => {
                    // Perform readRequest’s error steps, given buffer.[[Value]].
                    return read_request.error_steps_typed(objects, ctx.catch());
                },
                Err(err) => return Err(err),
                Ok(buffer) => buffer,
            };

            // Let pullIntoDescriptor be a new pull-into descriptor with...
            let pull_into_descriptor = PullIntoDescriptor {
                buffer,
                buffer_byte_length: auto_allocate_chunk_size,
                byte_offset: 0,
                byte_length: auto_allocate_chunk_size,
                bytes_filled: 0,
                minimum_fill: 1,
                element_size: 1,
                view_constructor: objects
                    .controller
                    .array_constructor_primordials
                    .constructor_uint8array
                    .clone(),
                reader_type: PullIntoDescriptorReaderType::Default,
            };

            // Append pullIntoDescriptor to this.[[pendingPullIntos]].
            objects
                .controller
                .pending_pull_intos
                .push_back(pull_into_descriptor);
        }

        // Perform ! ReadableStreamAddReadRequest(stream, readRequest).
        objects
            .stream
            .readable_stream_add_read_request(&mut objects.reader, read_request);

        // Perform ! ReadableByteStreamControllerCallPullIfNeeded(this).
        ReadableByteStreamController::readable_byte_stream_controller_call_pull_if_needed(
            ctx.clone(),
            objects,
        )
    }

    fn cancel_steps<R: ReadableStreamReader<'js>>(
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamObjects<'js, Self, R>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, ReadableStreamObjects<'js, Self, R>)> {
        // Perform ! ReadableByteStreamControllerClearPendingPullIntos(this).
        objects
            .controller
            .readable_byte_stream_controller_clear_pending_pull_intos();

        // Perform ! ResetQueue(this).
        objects.controller.reset_queue();

        // Let result be the result of performing this.[[cancelAlgorithm]], passing in reason.
        let (result, objects_class) =
            ReadableByteStreamController::cancel_algorithm(ctx.clone(), objects, reason)?;

        objects = ReadableStreamObjects::from_class(objects_class);

        // Perform ! ReadableByteStreamControllerClearAlgorithms(this).
        objects
            .controller
            .readable_byte_stream_controller_clear_algorithms();

        // Return result.
        Ok((result, objects))
    }

    fn release_steps(&mut self) {
        // If this.[[pendingPullIntos]] is not empty,
        if !self.pending_pull_intos.is_empty() {
            // Let firstPendingPullInto be this.[[pendingPullIntos]][0].
            let first_pending_pull_into = &mut self.pending_pull_intos[0];

            // Set firstPendingPullInto’s reader type to "none".
            first_pending_pull_into.reader_type = PullIntoDescriptorReaderType::None;

            // Set this.[[pendingPullIntos]] to the list « firstPendingPullInto ».
            _ = self.pending_pull_intos.split_off(1);
        }
    }
}

#[derive(JsLifetime, Trace, Clone)]
#[rquickjs::class]
pub(crate) struct ReadableStreamBYOBRequest<'js> {
    pub(super) view: Option<ViewBytes<'js>>,
    controller: Option<ReadableByteStreamControllerClass<'js>>,
}

#[methods(rename_all = "camelCase")]
impl<'js> ReadableStreamBYOBRequest<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>) -> Result<Class<'js, Self>> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    #[qjs(get)]
    fn view(&self) -> Null<ViewBytes<'js>> {
        Null(self.view.clone())
    }

    fn respond(
        ctx: Ctx<'js>,
        byob_request: This<OwnedBorrowMut<'js, Self>>,
        bytes_written: usize,
    ) -> Result<()> {
        // If this.[[controller]] is undefined, throw a TypeError exception.
        let (controller, view) = match (&byob_request.controller, &byob_request.view) {
            (Some(controller), Some(view)) => (controller.clone(), view),
            _ => {
                return Err(Exception::throw_type(
                    &ctx,
                    "This BYOB request has been invalidated",
                ));
            },
        };
        let (buffer, _, _) = view.get_array_buffer()?;
        drop(byob_request);

        // If ! IsDetachedBuffer(this.[[view]].[[ArrayBuffer]]) is true, throw a TypeError exception.
        if buffer.as_bytes().is_none() {
            return Err(Exception::throw_type(
                &ctx,
                "The BYOB request's buffer has been detached and so cannot be used as a response",
            ));
        }

        let objects =
            ReadableStreamObjects::from_byte_controller(OwnedBorrowMut::from_class(controller))
                .refresh_reader();

        // Perform ? ReadableByteStreamControllerRespond(this.[[controller]], bytesWritten).
        ReadableByteStreamController::readable_byte_stream_controller_respond(
            ctx,
            objects,
            bytes_written,
        )
    }

    fn respond_with_new_view(
        ctx: Ctx<'js>,
        byob_request: This<OwnedBorrowMut<'js, Self>>,
        view: Opt<Value<'js>>,
    ) -> Result<()> {
        // If this.[[controller]] is undefined, throw a TypeError exception.
        let controller = match &byob_request.controller {
            Some(controller) => controller.clone(),
            _ => {
                return Err(Exception::throw_type(
                    &ctx,
                    "This BYOB request has been invalidated",
                ));
            },
        };
        drop(byob_request);

        let controller = OwnedBorrowMut::from_class(controller);

        let view = ViewBytes::from_value(
            &ctx,
            &controller.function_array_buffer_is_view,
            view.0.as_ref(),
        )?;

        let (buffer, _, _) = view.get_array_buffer()?;

        // If ! IsDetachedBuffer(view.[[ViewedArrayBuffer]]) is true, throw a TypeError exception.
        if buffer.as_bytes().is_none() {
            return Err(Exception::throw_type(
                &ctx,
                "The given view's buffer has been detached and so cannot be used as a response",
            ));
        }

        let objects = ReadableStreamObjects::from_byte_controller(controller).refresh_reader();

        // Return ? ReadableByteStreamControllerRespondWithNewView(this.[[controller]], view).
        ReadableByteStreamController::readable_byte_stream_controller_respond_with_new_view(
            ctx, objects, view,
        )
    }
}

#[derive(JsLifetime)]
pub(super) struct PullIntoDescriptor<'js> {
    buffer: ArrayBuffer<'js>,
    buffer_byte_length: usize,
    byte_offset: usize,
    byte_length: usize,
    bytes_filled: usize,
    minimum_fill: usize,
    element_size: usize,
    view_constructor: Constructor<'js>,
    reader_type: PullIntoDescriptorReaderType,
}

impl<'js> Trace<'js> for PullIntoDescriptor<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        self.buffer.trace(tracer);
        self.buffer_byte_length.trace(tracer);
        self.byte_offset.trace(tracer);
        self.byte_length.trace(tracer);
        self.bytes_filled.trace(tracer);
        self.minimum_fill.trace(tracer);
        self.element_size.trace(tracer);
        self.view_constructor.trace(tracer);
        self.reader_type.trace(tracer);
    }
}

enum PullIntoDescriptorRefMut<'js, 'a> {
    Index(usize),
    Owned(&'a mut PullIntoDescriptor<'js>),
}

#[derive(Trace, Clone, Copy)]
enum PullIntoDescriptorReaderType {
    Default,
    Byob,
    None,
}

#[derive(JsLifetime)]
struct ReadableByteStreamQueueEntry<'js> {
    buffer: ArrayBuffer<'js>,
    byte_offset: usize,
    byte_length: usize,
}

impl<'js> Trace<'js> for ReadableByteStreamQueueEntry<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        self.buffer.trace(tracer);
        self.byte_offset.trace(tracer);
        self.byte_length.trace(tracer)
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
