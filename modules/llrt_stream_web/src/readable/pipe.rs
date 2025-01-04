use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use llrt_abort::AbortSignal;
use rquickjs::{
    class::{OwnedBorrow, Trace},
    prelude::{OnceFn, This},
    Class, Ctx, Function, Promise, Result, Value,
};

use super::{
    default_reader::ReadableStreamDefaultReaderOwned, objects::ReadableStreamClassObjects,
    objects::ReadableStreamObjects, reader::ReadableStreamReaderClass, ReadableStream,
    ReadableStreamControllerOwned, ReadableStreamDefaultReader, ReadableStreamOwned,
    ReadableStreamReadRequest, ReadableStreamState, WritableStreamDefaultWriter,
};
use crate::{
    promise_resolved_with, upon_promise,
    writable::{
        WritableStream, WritableStreamClassObjects, WritableStreamDefaultWriterOwned,
        WritableStreamObjects, WritableStreamOwned, WritableStreamState,
    },
    PromisePrimordials, ResolveablePromise, Undefined,
};

impl<'js> ReadableStream<'js> {
    pub(super) fn readable_stream_pipe_to(
        ctx: Ctx<'js>,
        source: ReadableStreamOwned<'js>,
        dest: WritableStreamOwned<'js>,
        prevent_close: bool,
        prevent_abort: bool,
        prevent_cancel: bool,
        signal: Option<Class<'js, AbortSignal<'js>>>,
    ) -> Result<Promise<'js>> {
        let (source_stored_error, source_closed) = match source.state {
            ReadableStreamState::Errored(ref stored_error) => (Some(stored_error.clone()), false),
            ReadableStreamState::Closed => (None, true),
            _ => (None, false),
        };
        let dest_stored_error = dest.stored_error();
        let dest_closing = dest.writable_stream_close_queued_or_in_flight()
            || matches!(dest.state, WritableStreamState::Closed);

        let source_controller = source.controller.clone();

        let dest_controller = dest
            .controller
            .clone()
            .expect("pipeTo called on writable stream without controller");

        // If source.[[controller]] implements ReadableByteStreamController, let reader be either ! AcquireReadableStreamBYOBReader(source) or ! AcquireReadableStreamDefaultReader(source), at the user agent’s discretion.
        // Otherwise, let reader be ! AcquireReadableStreamDefaultReader(source).
        let (mut source, reader) =
            ReadableStreamReaderClass::acquire_readable_stream_default_reader(ctx.clone(), source)?;

        let source_closed_promise = reader.borrow().generic.closed_promise.promise.clone();

        // Let writer be ! AcquireWritableStreamDefaultWriter(dest).
        let (dest, writer) =
            WritableStreamDefaultWriter::acquire_writable_stream_default_writer(&ctx, dest)?;

        let dest_closed_promise = writer.borrow().closed_promise.promise.clone();

        // Set source.[[disturbed]] to true.
        source.disturbed = true;

        let current_write = Rc::new(RefCell::new(promise_resolved_with(
            &ctx,
            &source.promise_primordials,
            Ok(Value::new_undefined(ctx.clone())),
        )?));

        let promise_primordials = source.promise_primordials.clone();
        let constructor_type_error = source.constructor_type_error.clone();

        let mut pipe_to = PipeTo {
            source_objects: ReadableStreamClassObjects {
                stream: source.into_inner(),
                controller: source_controller,
                reader,
            },
            dest_objects: WritableStreamClassObjects {
                stream: dest.into_inner(),
                controller: dest_controller,
                writer,
            },
            current_write,
            // Let shuttingDown be false.
            shutting_down: Rc::new(AtomicBool::new(false)),
            signal,
            abort_callback: None,
            // Let promise be a new promise.
            promise: ResolveablePromise::new(&ctx)?,
            promise_primordials: promise_primordials.clone(),
        };

        // If signal is not undefined,
        if let Some(signal) = &pipe_to.signal {
            // Let abortAlgorithm be the following steps:
            let abort_algorithm = {
                let signal = signal.clone();
                let pipe_to = pipe_to.clone();
                move |ctx: Ctx<'js>| -> Result<()> {
                    // Let error be signal’s abort reason.
                    let error = signal
                        .borrow()
                        .reason()
                        .unwrap_or(Value::new_undefined(ctx.clone()));

                    // Let actions be an empty ordered set.
                    let mut actions =
                        Vec::<Box<dyn FnOnce(Ctx<'js>) -> Result<Promise<'js>>>>::new();

                    // If preventAbort is false, append the following action to actions:
                    if !prevent_abort {
                        let dest_objects = pipe_to.dest_objects.clone();
                        let error = error.clone();
                        actions.push(Box::new(move |ctx| {
                            let dest_objects = WritableStreamObjects::from_class(dest_objects);

                            if matches!(dest_objects.stream.state, WritableStreamState::Writable) {
                                // If dest.[[state]] is "writable", return ! WritableStreamAbort(dest, error).
                                let (promise, _) = WritableStream::writable_stream_abort(
                                    ctx,
                                    dest_objects,
                                    Some(error.clone()),
                                )?;
                                Ok(promise)
                            } else {
                                // Otherwise, return a promise resolved with undefined.
                                promise_resolved_with(
                                    &ctx,
                                    &dest_objects.stream.promise_primordials,
                                    Ok(Value::new_undefined(ctx.clone())),
                                )
                            }
                        }));
                    }

                    // If preventCancel is false, append the following action action to actions:
                    if !prevent_cancel {
                        let source_objects = pipe_to.source_objects.clone();
                        let error = error.clone();
                        actions.push(Box::new(move |ctx| {
                            let source_objects = ReadableStreamObjects::from_class(source_objects);

                            if let ReadableStreamState::Readable = source_objects.stream.state {
                                // If source.[[state]] is "readable", return ! ReadableStreamCancel(source, error).
                                let (promise, _) = ReadableStream::readable_stream_cancel(
                                    ctx,
                                    source_objects,
                                    error.clone(),
                                )?;

                                Ok(promise)
                            } else {
                                // Otherwise, return a promise resolved with undefined.
                                promise_resolved_with(
                                    &ctx,
                                    &source_objects.stream.promise_primordials,
                                    Ok(Value::new_undefined(ctx.clone())),
                                )
                            }
                        }));
                    }

                    // Shutdown with an action consisting of getting a promise to wait for all of the actions in actions, and with error.
                    pipe_to.shutdown_with_action(
                        ctx,
                        move |ctx| {
                            let promises: Vec<Promise<'js>> = actions
                                .into_iter()
                                .map(|action| action(ctx.clone()))
                                .collect::<Result<Vec<_>>>()?;

                            let all_promises: Promise<'js> =
                                promise_primordials.promise_all.call((
                                    This(promise_primordials.promise_constructor.clone()),
                                    promises,
                                ))?;

                            Ok(all_promises)
                        },
                        Some(error),
                    )
                }
            };

            // If signal is aborted, perform abortAlgorithm and return promise.
            {
                let signal = signal.borrow();

                if signal.aborted {
                    abort_algorithm(ctx.clone())?;

                    return Ok(pipe_to.promise.promise);
                }
            }

            let abort_callback = pipe_to
                .abort_callback
                .insert(Function::new(ctx.clone(), OnceFn::new(abort_algorithm))?);

            // Add abortAlgorithm to signal.
            AbortSignal::set_on_abort(This(signal.clone()), ctx.clone(), abort_callback.clone())?;
        }

        // In parallel but not really; see #905, using reader and writer, read all chunks from source and write them to dest.
        // Due to the locking provided by the reader and writer, the exact manner in which this happens is not observable to author code, and so there is flexibility in how this is done.
        // The following constraints apply regardless of the exact algorithm used:

        // Errors must be propagated forward
        PipeTo::is_or_becomes_errored(
            ctx.clone(),
            source_stored_error,
            source_closed_promise.clone(),
            {
                let pipe_to = pipe_to.clone();
                move |ctx, stored_error| {
                    if !prevent_abort {
                        pipe_to.shutdown_with_action(
                            ctx,
                            {
                                let pipe_to = pipe_to.clone();
                                let stored_error = stored_error.clone();
                                move |ctx| {
                                    let dest_objects = WritableStreamObjects::from_class(
                                        pipe_to.dest_objects.clone(),
                                    );

                                    let (promise, _) = WritableStream::writable_stream_abort(
                                        ctx,
                                        dest_objects,
                                        Some(stored_error),
                                    )?;

                                    Ok(promise)
                                }
                            },
                            Some(stored_error),
                        )
                    } else {
                        pipe_to.shutdown(ctx, Some(stored_error))
                    }
                }
            },
        )?;

        // Errors must be propagated backward
        PipeTo::is_or_becomes_errored(ctx.clone(), dest_stored_error, dest_closed_promise, {
            let pipe_to = pipe_to.clone();
            move |ctx, stored_error| {
                if !prevent_cancel {
                    pipe_to.shutdown_with_action(
                        ctx,
                        {
                            let pipe_to = pipe_to.clone();
                            let stored_error = stored_error.clone();
                            move |ctx| {
                                let source_objects = ReadableStreamObjects::from_class(
                                    pipe_to.source_objects.clone(),
                                );

                                let (promise, _) = ReadableStream::readable_stream_cancel(
                                    ctx,
                                    source_objects,
                                    stored_error,
                                )?;

                                Ok(promise)
                            }
                        },
                        Some(stored_error),
                    )
                } else {
                    pipe_to.shutdown(ctx, Some(stored_error))
                }
            }
        })?;

        // Closing must be propagated forward
        PipeTo::is_or_becomes_closed(ctx.clone(), source_closed, source_closed_promise, {
            let pipe_to = pipe_to.clone();
            move |ctx| {
                if !prevent_close {
                    pipe_to.shutdown_with_action(
                        ctx,
                        {
                            let pipe_to = pipe_to.clone();
                            move |ctx| {
                                let dest_objects = WritableStreamObjects::from_class(pipe_to.dest_objects);

                                WritableStreamDefaultWriter::writable_stream_default_writer_close_with_error_propagation(ctx, dest_objects)
                            }
                        },
                        None,
                    )
                } else {
                    pipe_to.shutdown(ctx, None)
                }
            }
        })?;

        // Closing must be propagated backward
        if dest_closing {
            let dest_closed: Value<'js> = constructor_type_error.call((
                "the destination writable stream closed before all data could be piped to it",
            ))?;

            if !prevent_cancel {
                pipe_to.shutdown_with_action(
                    ctx.clone(),
                    {
                        let pipe_to = pipe_to.clone();
                        let dest_closed = dest_closed.clone();
                        move |ctx| {
                            let source_objects =
                                ReadableStreamObjects::from_class(pipe_to.source_objects.clone());

                            let (promise, _) = ReadableStream::readable_stream_cancel(
                                ctx,
                                source_objects,
                                dest_closed,
                            )?;

                            Ok(promise)
                        }
                    },
                    Some(dest_closed),
                )?;
            } else {
                pipe_to.shutdown(ctx.clone(), Some(dest_closed))?;
            }
        }

        let result_promise = pipe_to.promise.promise.clone();
        let pipe_loop_promise = pipe_to.pipe_loop(ctx)?;
        pipe_loop_promise.set_is_handled()?;

        Ok(result_promise)
    }
}

#[derive(Clone)]
struct PipeTo<'js> {
    source_objects: ReadableStreamClassObjects<
        'js,
        ReadableStreamControllerOwned<'js>,
        ReadableStreamDefaultReaderOwned<'js>,
    >,
    dest_objects: WritableStreamClassObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
    current_write: Rc<RefCell<Promise<'js>>>,
    shutting_down: Rc<AtomicBool>,
    signal: Option<Class<'js, AbortSignal<'js>>>,
    abort_callback: Option<Function<'js>>,
    promise: ResolveablePromise<'js>,

    promise_primordials: PromisePrimordials<'js>,
}

impl<'js> PipeTo<'js> {
    // Using reader and writer, read all chunks from this and write them to dest
    // - Backpressure must be enforced
    // - Shutdown must stop all activity
    fn pipe_loop(self, ctx: Ctx<'js>) -> Result<ResolveablePromise<'js>> {
        let loop_promise = ResolveablePromise::new(&ctx)?;

        self.next(ctx, false, loop_promise.clone())?;

        Ok(loop_promise)
    }

    fn next(&self, ctx: Ctx<'js>, done: bool, loop_promise: ResolveablePromise<'js>) -> Result<()> {
        if done {
            loop_promise.resolve(Value::new_undefined(ctx))?;
        } else {
            let pipe_step_promise = self.pipe_step(ctx.clone())?;
            upon_promise(ctx, pipe_step_promise, {
                {
                    let pipe_to = self.clone();
                    move |ctx, result| match result {
                        Ok(done) => pipe_to.next(ctx, done, loop_promise),
                        Err(err) => loop_promise.reject(err),
                    }
                }
            })?;
        }

        Ok(())
    }

    fn pipe_step(&self, ctx: Ctx<'js>) -> Result<Promise<'js>> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return promise_resolved_with(
                &ctx,
                &self.promise_primordials,
                Ok(Value::new_bool(ctx.clone(), true)),
            );
        }

        let writer_ready = self
            .dest_objects
            .writer
            .borrow()
            .ready_promise
            .promise
            .clone();

        writer_ready.then()?.call((
            This(writer_ready.clone()),
            Function::new(
                ctx.clone(),
                OnceFn::new({
                    let current_write = self.current_write.clone();
                    let source_objects = self.source_objects.clone();
                    let dest_objects = self.dest_objects.clone();
                    move |ctx: Ctx<'js>| -> Result<Promise<'js>> {
                        let read_promise = ResolveablePromise::new(&ctx)?;

                        struct ReadRequest<'js> {
                            dest_objects:
                                WritableStreamClassObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
                            current_write: Rc<RefCell<Promise<'js>>>,
                            read_promise: ResolveablePromise<'js>,
                        }

                        impl<'js> Trace<'js> for ReadRequest<'js> {
                            fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
                                self.current_write.as_ref().borrow().trace(tracer);
                                self.read_promise.trace(tracer);
                            }
                        }

                        impl<'js> ReadableStreamReadRequest<'js> for ReadRequest<'js> {
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
                                let ctx = chunk.ctx().clone();

                                // calling write can trigger user code; ensure we don't hold locks
                                let objects = objects.into_inner();

                                let dest_objects = WritableStreamObjects::from_class(self.dest_objects.clone());
                                let write_promise = WritableStreamDefaultWriter::writable_stream_default_writer_write(ctx.clone(), dest_objects, chunk)?;

                                let write_promise: Promise<'js> = write_promise.then()?.call((
                                    This(write_promise.clone()),
                                    Value::new_undefined(ctx.clone()),
                                    Function::new(ctx.clone(), || {}),
                                ))?;

                                self.current_write.replace(write_promise);
                                self.read_promise.resolve(Value::new_bool(ctx.clone(), false))?;

                                Ok(ReadableStreamObjects::from_class(objects))
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
                                self.read_promise
                                    .resolve(Value::new_bool(ctx.clone(), true))?;

                                Ok(objects)
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
                                self.read_promise.reject(reason)?;

                                Ok(objects)
                            }
                        }

                        let objects = ReadableStreamObjects::from_class(source_objects);

                        let promise = read_promise.promise.clone();

                        ReadableStreamDefaultReader::readable_stream_default_reader_read(
                            &ctx,
                            objects,
                            ReadRequest { current_write, read_promise, dest_objects },
                        )?;

                        Ok(promise)
                    }
                }),
            ),
        ))
    }

    fn is_or_becomes_errored(
        ctx: Ctx<'js>,
        stored_error: Option<Value<'js>>,
        promise: Promise<'js>,
        action: impl FnOnce(Ctx<'js>, Value<'js>) -> Result<()> + 'js,
    ) -> Result<()> {
        if let Some(stored_error) = stored_error {
            action(ctx, stored_error)
        } else {
            promise.then()?.call((
                This(promise.clone()),
                Value::new_undefined(ctx.clone()),
                Function::new(ctx.clone(), OnceFn::new(action)),
            ))
        }
    }

    fn is_or_becomes_closed(
        ctx: Ctx<'js>,
        already_closed: bool,
        promise: Promise<'js>,
        action: impl FnOnce(Ctx<'js>) -> Result<()> + 'js,
    ) -> Result<()> {
        if already_closed {
            action(ctx)
        } else {
            promise.then()?.call((
                This(promise.clone()),
                Function::new(ctx.clone(), OnceFn::new(action)),
                Value::new_undefined(ctx.clone()),
            ))
        }
    }

    fn shutdown_with_action(
        &self,
        ctx: Ctx<'js>,
        action: impl FnOnce(Ctx<'js>) -> Result<Promise<'js>> + 'js,
        original_error: Option<Value<'js>>,
    ) -> Result<()> {
        if self.shutting_down.swap(true, Ordering::Relaxed) {
            // already shutting down
            return Ok(());
        }

        let do_the_rest = {
            let pipe_to = self.clone();
            move |ctx: Ctx<'js>| -> Result<()> {
                let action_promise = action(ctx.clone())?;
                upon_promise(ctx, action_promise, move |ctx, result| match result {
                    Ok(()) => pipe_to.finalize(ctx, original_error),
                    Err(new_error) => pipe_to.finalize(ctx, Some(new_error)),
                })?;
                Ok(())
            }
        };

        let writable = {
            let dest_stream = OwnedBorrow::from_class(self.dest_objects.stream.clone());
            matches!(dest_stream.state, WritableStreamState::Writable)
                && !dest_stream.writable_stream_close_queued_or_in_flight()
        };

        if writable {
            let wait_promise =
                Self::wait_for_writes_to_finish(ctx.clone(), self.current_write.clone())?;
            wait_promise.then()?.call((
                This(wait_promise.clone()),
                Function::new(ctx, OnceFn::new(|ctx: Ctx<'js>| do_the_rest(ctx))),
            ))
        } else {
            do_the_rest(ctx)
        }
    }

    fn shutdown(&self, ctx: Ctx<'js>, error: Option<Value<'js>>) -> Result<()> {
        if self.shutting_down.swap(true, Ordering::Relaxed) {
            // already shutting down
            return Ok(());
        }

        let writable = {
            let dest_stream = OwnedBorrow::from_class(self.dest_objects.stream.clone());
            matches!(dest_stream.state, WritableStreamState::Writable)
                && !dest_stream.writable_stream_close_queued_or_in_flight()
        };

        if writable {
            let wait_promise =
                Self::wait_for_writes_to_finish(ctx.clone(), self.current_write.clone())?;
            let pipe_to = self.clone();
            wait_promise.then()?.call((
                This(wait_promise.clone()),
                Function::new(
                    ctx,
                    OnceFn::new(move |ctx: Ctx<'js>| pipe_to.finalize(ctx, error)),
                ),
            ))
        } else {
            self.finalize(ctx, error)
        }
    }

    fn wait_for_writes_to_finish(
        ctx: Ctx<'js>,
        current_write: Rc<RefCell<Promise<'js>>>,
    ) -> Result<Promise<'js>> {
        let old_current_write: Promise<'js> = current_write.as_ref().borrow().clone();

        old_current_write.then()?.call((
            This(old_current_write.clone()),
            Function::new(
                ctx.clone(),
                OnceFn::new({
                    move |ctx: Ctx<'js>| -> Result<Undefined<Promise<'js>>> {
                        if !old_current_write.eq(&current_write.as_ref().borrow()) {
                            Ok(Undefined(Some(Self::wait_for_writes_to_finish(
                                ctx,
                                current_write,
                            )?)))
                        } else {
                            Ok(Undefined(None))
                        }
                    }
                }),
            ),
            Value::new_undefined(ctx),
        ))
    }

    fn finalize(&self, ctx: Ctx<'js>, error: Option<Value<'js>>) -> Result<()> {
        let source_objects = ReadableStreamObjects::from_class(self.source_objects.clone());
        let dest_objects = WritableStreamObjects::from_class(self.dest_objects.clone());

        WritableStreamDefaultWriter::writable_stream_default_writer_release(dest_objects)?;
        ReadableStreamDefaultReader::readable_stream_default_reader_release(source_objects)?;

        if let (Some(signal), Some(abort_callback)) = (&self.signal, &self.abort_callback) {
            AbortSignal::remove_on_abort(
                This(signal.clone()),
                ctx.clone(),
                abort_callback.clone(),
            )?;
        }

        if let Some(error) = error {
            self.promise.reject(error)
        } else {
            self.promise.resolve(Value::new_undefined(ctx))
        }
    }
}
