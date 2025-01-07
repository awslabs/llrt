use llrt_utils::option::Null;
use rquickjs::{
    class::{JsClass, OwnedBorrow, OwnedBorrowMut, Trace},
    function::Constructor,
    prelude::{Opt, This},
    Class, Ctx, Exception, JsLifetime, Promise, Result, Value,
};

use crate::utils::{
    promise::{promise_rejected_with, PromisePrimordials, ResolveablePromise},
    UnwrapOrUndefined,
};

use super::{
    default_controller::WritableStreamDefaultController, objects::WritableStreamObjects,
    writer::WritableStreamWriter, WritableStream, WritableStreamOwned, WritableStreamState,
};

#[rquickjs::class]
#[derive(JsLifetime, Trace)]
pub(crate) struct WritableStreamDefaultWriter<'js> {
    pub(crate) ready_promise: ResolveablePromise<'js>,
    pub(crate) closed_promise: ResolveablePromise<'js>,
    pub(super) stream: Option<Class<'js, WritableStream<'js>>>,

    #[qjs(skip_trace)]
    constructor_type_error: Constructor<'js>,
    #[qjs(skip_trace)]
    promise_primordials: PromisePrimordials<'js>,
}

pub(crate) type WritableStreamDefaultWriterClass<'js> =
    Class<'js, WritableStreamDefaultWriter<'js>>;
pub(crate) type WritableStreamDefaultWriterOwned<'js> =
    OwnedBorrowMut<'js, WritableStreamDefaultWriter<'js>>;

#[rquickjs::methods(rename_all = "camelCase")]
impl<'js> WritableStreamDefaultWriter<'js> {
    // this is required by web platform tests
    #[qjs(get)]
    pub fn constructor(ctx: Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        <WritableStreamDefaultWriter as JsClass>::constructor(&ctx)
    }

    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>, stream: WritableStreamOwned<'js>) -> Result<Class<'js, Self>> {
        // Perform ? SetUpWritableStreamDefaultWriter(this, stream).
        let (_, writer) = Self::set_up_writable_stream_default_writer(&ctx, stream)?;
        Ok(writer)
    }

    #[qjs(get)]
    fn closed(writer: This<OwnedBorrowMut<'js, Self>>) -> Promise<'js> {
        // Return this.[[closedPromise]].
        writer.0.closed_promise.promise.clone()
    }

    #[qjs(get)]
    fn desired_size(ctx: Ctx<'js>, writer: This<OwnedBorrowMut<'js, Self>>) -> Result<Null<f64>> {
        match writer.0.stream {
            // If this.[[stream]] is undefined, throw a TypeError exception.
            None => Err(Exception::throw_type(
                &ctx,
                "Cannot desiredSize a stream using a released writer",
            )),
            Some(ref stream) => {
                // Return ! WritableStreamDefaultWriterGetDesiredSize(this).
                Self::writable_stream_default_writer_get_desired_size(&OwnedBorrowMut::from_class(
                    stream.clone(),
                ))
            },
        }
    }

    #[qjs(get)]
    fn ready(writer: This<OwnedBorrowMut<'js, Self>>) -> Promise<'js> {
        // Return this.[[readyPromise]].
        writer.0.ready_promise.promise.clone()
    }

    fn abort(
        ctx: Ctx<'js>,
        writer: This<OwnedBorrowMut<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        // If this.[[stream]] is undefined, throw a TypeError exception.
        if writer.0.stream.is_none() {
            let e: Value = writer
                .constructor_type_error
                .call(("Cannot abort a stream using a released writer",))?;

            promise_rejected_with(&writer.promise_primordials, e)
        } else {
            let objects = WritableStreamObjects::from_writer(writer.0);

            // Return ! WritableStreamDefaultWriterAbort(this, reason).
            Self::writable_stream_default_writer_abort(ctx.clone(), objects, reason.0)
        }
    }

    fn close(ctx: Ctx<'js>, writer: This<OwnedBorrowMut<'js, Self>>) -> Result<Promise<'js>> {
        // If this.[[stream]] is undefined, throw a TypeError exception.
        if writer.0.stream.is_none() {
            let e: Value = writer
                .constructor_type_error
                .call(("Cannot close a stream using a released writer",))?;

            promise_rejected_with(&writer.promise_primordials, e)
        } else {
            let objects = WritableStreamObjects::from_writer(writer.0);

            // If ! WritableStreamCloseQueuedOrInFlight(stream) is true, return a promise rejected with a TypeError exception.
            if objects.stream.writable_stream_close_queued_or_in_flight() {
                let e: Value = objects
                    .writer
                    .constructor_type_error
                    .call(("Cannot close an already-closing stream",))?;

                return promise_rejected_with(&objects.stream.promise_primordials, e);
            }

            // Return ! WritableStreamDefaultWriterClose(this).
            Self::writable_stream_default_writer_close(ctx, objects)
        }
    }

    fn release_lock(writer: This<OwnedBorrowMut<'js, Self>>) -> Result<()> {
        // If stream is undefined, return.
        if writer.0.stream.is_none() {
            Ok(())
        } else {
            let objects = WritableStreamObjects::from_writer(writer.0);

            // Perform ! WritableStreamDefaultWriterRelease(this).
            Self::writable_stream_default_writer_release(objects)
        }
    }

    fn write(
        ctx: Ctx<'js>,
        writer: This<OwnedBorrowMut<'js, Self>>,
        chunk: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        // If this.[[stream]] is undefined, throw a TypeError exception.
        if writer.0.stream.is_none() {
            let e: Value = writer
                .constructor_type_error
                .call(("Cannot write a stream using a released writer",))?;

            promise_rejected_with(&writer.promise_primordials, e)
        } else {
            let objects = WritableStreamObjects::from_writer(writer.0);

            // Return ! WritableStreamDefaultWriterWrite(this, chunk).
            Self::writable_stream_default_writer_write(
                ctx.clone(),
                objects,
                chunk.0.unwrap_or_undefined(&ctx),
            )
        }
    }
}

impl<'js> WritableStreamDefaultWriter<'js> {
    pub(crate) fn acquire_writable_stream_default_writer(
        ctx: &Ctx<'js>,
        stream: WritableStreamOwned<'js>,
    ) -> Result<(WritableStreamOwned<'js>, Class<'js, Self>)> {
        Self::set_up_writable_stream_default_writer(ctx, stream)
    }

    pub(super) fn set_up_writable_stream_default_writer(
        ctx: &Ctx<'js>,
        mut stream: WritableStreamOwned<'js>,
    ) -> Result<(WritableStreamOwned<'js>, Class<'js, Self>)> {
        // If ! IsWritableStreamLocked(stream) is true, throw a TypeError exception.
        if stream.is_writable_stream_locked() {
            return Err(Exception::throw_type(
                ctx,
                "This stream has already been locked for exclusive writing by another writer",
            ));
        }

        let promise_primordials = stream.promise_primordials.clone();
        let constructor_type_error = stream.constructor_type_error.clone();
        let stream_class = stream.into_inner();
        stream = OwnedBorrowMut::from_class(stream_class.clone());

        let (ready_promise, closed_promise) = match stream.state {
            WritableStreamState::Writable => {
                let ready_promise =
                    if !stream.writable_stream_close_queued_or_in_flight() && stream.backpressure {
                        // If ! WritableStreamCloseQueuedOrInFlight(stream) is false and stream.[[backpressure]] is true, set writer.[[readyPromise]] to a new promise.
                        ResolveablePromise::new(ctx)?
                    } else {
                        // Otherwise, set writer.[[readyPromise]] to a promise resolved with undefined.
                        ResolveablePromise::resolved_with_undefined(&stream.promise_primordials)
                    };

                // Set writer.[[closedPromise]] to a new promise.
                (ready_promise, ResolveablePromise::new(ctx)?)
            },
            WritableStreamState::Erroring(ref stored_error) => {
                let ready_promise = ResolveablePromise::rejected_with(
                    &stream.promise_primordials,
                    stored_error.clone(),
                )?;
                ready_promise.set_is_handled()?;
                // Set writer.[[closedPromise]] to a new promise.
                (ready_promise, ResolveablePromise::new(ctx)?)
            },
            WritableStreamState::Closed => {
                let promise =
                    ResolveablePromise::resolved_with_undefined(&stream.promise_primordials);
                // Set writer.[[readyPromise]] to a promise resolved with undefined.
                // Set writer.[[closedPromise]] to a promise resolved with undefined.
                (promise.clone(), promise)
            },
            // Let storedError be stream.[[storedError]].
            WritableStreamState::Errored(ref stored_error) => {
                let promise = ResolveablePromise::rejected_with(
                    &stream.promise_primordials,
                    stored_error.clone(),
                )?;
                promise.set_is_handled()?;
                // Set writer.[[readyPromise]] to a promise rejected with storedError.
                // Set writer.[[readyPromise]].[[PromiseIsHandled]] to true.
                // Set writer.[[closedPromise]] to a promise rejected with storedError.
                // Set writer.[[closedPromise]].[[PromiseIsHandled]] to true.
                (promise.clone(), promise)
            },
        };

        let writer = Self {
            ready_promise,
            closed_promise,
            // Set writer.[[stream]] to stream.
            stream: Some(stream_class),
            promise_primordials,
            constructor_type_error,
        };

        let writer = Class::instance(ctx.clone(), writer)?;

        stream.writer = Some(writer.clone());

        Ok((stream, writer))
    }

    pub(super) fn writable_stream_default_writer_ensure_ready_promise_rejected(
        &mut self,
        promise_primordials: &PromisePrimordials<'js>,
        error: Value<'js>,
    ) -> Result<()> {
        if self.ready_promise.is_pending() {
            // If writer.[[readyPromise]].[[PromiseState]] is "pending", reject writer.[[readyPromise]] with error.
            self.ready_promise.reject(error)?;
        } else {
            // Otherwise, set writer.[[readyPromise]] to a promise rejected with error.
            self.ready_promise = ResolveablePromise::rejected_with(promise_primordials, error)?;
        }

        // Set writer.[[readyPromise]].[[PromiseIsHandled]] to true.
        self.ready_promise.set_is_handled()?;
        Ok(())
    }

    pub(super) fn writable_stream_default_writer_ensure_closed_promise_rejected(
        &mut self,
        promise_primordials: &PromisePrimordials<'js>,
        error: Value<'js>,
    ) -> Result<()> {
        if self.closed_promise.is_pending() {
            // If writer.[[closedPromise]].[[PromiseState]] is "pending", reject writer.[[closedPromise]] with error.
            self.closed_promise.reject(error)?;
        } else {
            // Otherwise, set writer.[[closedPromise]] to a promise rejected with error.
            self.closed_promise = ResolveablePromise::rejected_with(promise_primordials, error)?;
        }

        // Set writer.[[closedPromise]].[[PromiseIsHandled]] to true.
        self.closed_promise.set_is_handled()?;
        Ok(())
    }

    pub(super) fn writable_stream_default_writer_get_desired_size(
        // Let stream be writer.[[stream]].
        stream: &WritableStream<'js>,
    ) -> Result<Null<f64>> {
        // Let state be stream.[[state]].
        // If state is "errored" or "erroring", return null.
        if matches!(
            stream.state,
            WritableStreamState::Errored(_) | WritableStreamState::Erroring(_)
        ) {
            return Ok(Null(None));
        }

        // If state is "closed", return 0.
        if matches!(stream.state, WritableStreamState::Closed) {
            return Ok(Null(Some(0.0)));
        }

        // Return ! WritableStreamDefaultControllerGetDesiredSize(stream.[[controller]]).
        let controller = OwnedBorrow::from_class(
            stream
                .controller
                .clone()
                .expect("Stream in state writable must have a controller"),
        );

        Ok(Null(Some(
            controller.writable_stream_default_controller_get_desired_size(),
        )))
    }

    fn writable_stream_default_writer_abort(
        ctx: Ctx<'js>,
        objects: WritableStreamObjects<'js, OwnedBorrowMut<'js, Self>>,
        reason: Option<Value<'js>>,
    ) -> Result<Promise<'js>> {
        // Return ! WritableStreamAbort(stream, reason).
        let (promise, _) = WritableStream::writable_stream_abort(ctx, objects, reason)?;
        Ok(promise)
    }

    fn writable_stream_default_writer_close(
        ctx: Ctx<'js>,
        objects: WritableStreamObjects<'js, OwnedBorrowMut<'js, Self>>,
    ) -> Result<Promise<'js>> {
        // Return ! WritableStreamClose(stream).
        let (promise, _) = WritableStream::writable_stream_close(ctx, objects)?;
        Ok(promise)
    }

    pub(crate) fn writable_stream_default_writer_close_with_error_propagation(
        ctx: Ctx<'js>,
        // Let stream be writer.[[stream]].
        objects: WritableStreamObjects<'js, OwnedBorrowMut<'js, Self>>,
    ) -> Result<Promise<'js>> {
        // Let state be stream.[[state]].
        // If ! WritableStreamCloseQueuedOrInFlight(stream) is true or state is "closed", return a promise resolved with undefined.
        if objects.stream.writable_stream_close_queued_or_in_flight()
            || matches!(objects.stream.state, WritableStreamState::Closed)
        {
            return Ok(objects
                .stream
                .promise_primordials
                .promise_resolved_with_undefined
                .clone());
        }

        // If state is "errored", return a promise rejected with stream.[[storedError]].
        if let WritableStreamState::Errored(ref stored_error) = objects.stream.state {
            return promise_rejected_with(
                &objects.stream.promise_primordials,
                stored_error.clone(),
            );
        }

        // Return ! WritableStreamDefaultWriterClose(writer).
        Self::writable_stream_default_writer_close(ctx, objects)
    }

    pub(crate) fn writable_stream_default_writer_release(
        mut objects: WritableStreamObjects<'js, OwnedBorrowMut<'js, Self>>,
    ) -> Result<()> {
        // Let releasedError be a new TypeError.
        let released_error: Value = objects.stream.constructor_type_error.call((
            "Writer was released and can no longer be used to monitor the stream's closedness",
        ))?;

        // Perform ! WritableStreamDefaultWriterEnsureReadyPromiseRejected(writer, releasedError).
        objects
            .writer
            .writable_stream_default_writer_ensure_ready_promise_rejected(
                &objects.stream.promise_primordials,
                released_error.clone(),
            )?;
        // Perform ! WritableStreamDefaultWriterEnsureClosedPromiseRejected(writer, releasedError).
        objects
            .writer
            .writable_stream_default_writer_ensure_closed_promise_rejected(
                &objects.stream.promise_primordials,
                released_error,
            )?;

        // Set stream.[[writer]] to undefined.
        objects.stream.writer = None;
        // Set writer.[[stream]] to undefined.
        objects.writer.stream = None;

        Ok(())
    }

    pub(crate) fn writable_stream_default_writer_write(
        ctx: Ctx<'js>,
        objects: WritableStreamObjects<'js, OwnedBorrowMut<'js, Self>>,
        chunk: Value<'js>,
    ) -> Result<Promise<'js>> {
        // Let chunkSize be ! WritableStreamDefaultControllerGetChunkSize(controller, chunk).
        let (chunk_size, mut objects) =
            WritableStreamDefaultController::writable_stream_default_controller_get_chunk_size(
                ctx.clone(),
                objects,
                chunk.clone(),
            )?;

        let stream_class = objects.stream.into_inner();
        objects.stream = OwnedBorrowMut::from_class(stream_class.clone());

        // If stream is not equal to writer.[[stream]], return a promise rejected with a TypeError exception.
        if objects.writer.stream != Some(stream_class) {
            let e: Value = objects
                .stream
                .constructor_type_error
                .call(("Cannot write to a stream using a released writer",))?;

            return promise_rejected_with(&objects.stream.promise_primordials, e);
        }

        // Let state be stream.[[state]].
        // If state is "errored", return a promise rejected with stream.[[storedError]].
        if let WritableStreamState::Errored(ref stored_error) = objects.stream.state {
            return promise_rejected_with(
                &objects.stream.promise_primordials,
                stored_error.clone(),
            );
        }

        // If ! WritableStreamCloseQueuedOrInFlight(stream) is true or state is "closed", return a promise rejected with a TypeError exception indicating that the stream is closing or closed.
        if objects.stream.writable_stream_close_queued_or_in_flight()
            || matches!(objects.stream.state, WritableStreamState::Closed)
        {
            let e: Value = objects
                .stream
                .constructor_type_error
                .call(("The stream is closing or closed and cannot be written to",))?;

            return promise_rejected_with(&objects.stream.promise_primordials, e);
        }

        // If state is "erroring", return a promise rejected with stream.[[storedError]].
        if let WritableStreamState::Erroring(ref stored_error) = objects.stream.state {
            return promise_rejected_with(
                &objects.stream.promise_primordials,
                stored_error.clone(),
            );
        }

        // Let promise be ! WritableStreamAddWriteRequest(stream).
        let promise = objects.stream.writable_stream_add_write_request(&ctx);
        // Perform ! WritableStreamDefaultControllerWrite(controller, chunk, chunkSize).
        WritableStreamDefaultController::writable_stream_default_controller_write(
            ctx, objects, chunk, chunk_size,
        )?;

        // Return promise.
        promise
    }
}

impl<'js> WritableStreamWriter<'js> for WritableStreamDefaultWriterOwned<'js> {
    type Class = WritableStreamDefaultWriterClass<'js>;

    fn with_writer<C>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            WritableStreamDefaultWriterOwned<'js>,
        ) -> Result<(C, WritableStreamDefaultWriterOwned<'js>)>,
        _: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)> {
        default(ctx, self)
    }

    fn into_inner(self) -> Self::Class {
        self.into_inner()
    }

    fn from_class(class: Self::Class) -> Self {
        OwnedBorrowMut::from_class(class)
    }
}
