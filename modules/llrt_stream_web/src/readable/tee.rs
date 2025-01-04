use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use llrt_utils::primordials::Primordial;
use rquickjs::{
    class::{OwnedBorrowMut, Trace},
    function::Constructor,
    prelude::{List, OnceFn},
    ArrayBuffer, Class, Ctx, Error, Function, IntoJs, JsLifetime, Promise, Result, Value,
};

use super::{
    byob_reader::ReadableStreamReadIntoRequest,
    byob_reader::{ReadableStreamBYOBReaderOwned, ViewBytes},
    byte_controller::ReadableByteStreamControllerOwned,
    controller::ReadableStreamController,
    default_controller::ReadableStreamDefaultController,
    default_controller::ReadableStreamDefaultControllerOwned,
    default_reader::ReadableStreamDefaultReaderOwned,
    promise_resolved_with,
    reader::UndefinedReader,
    CancelAlgorithm, PullAlgorithm, ReadableByteStreamController, ReadableStream,
    ReadableStreamBYOBReader, ReadableStreamClass, ReadableStreamClassObjects,
    ReadableStreamControllerClass, ReadableStreamControllerOwned, ReadableStreamDefaultReader,
    ReadableStreamObjects, ReadableStreamReadRequest, ReadableStreamReader,
    ReadableStreamReaderClass, ReadableStreamReaderOwned, StartAlgorithm,
};
use crate::{upon_promise, ResolveablePromise};

type ReadableStreamPair<'js> = (ReadableStreamClass<'js>, ReadableStreamClass<'js>);

impl<'js> ReadableStream<'js> {
    pub(super) fn readable_stream_tee<C: ReadableStreamController<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableStreamObjects<'js, C, UndefinedReader>,
        clone_for_branch_2: bool,
    ) -> Result<ReadableStreamPair<'js>> {
        let (streams, _) = objects.with_controller(
            ctx,
            |ctx, objects| {
                // Return ? ReadableStreamDefaultTee(stream, cloneForBranch2).
                let (streams, objects) =
                    Self::readable_stream_default_tee(ctx, objects, clone_for_branch_2)?;
                Ok((streams, objects.clear_reader()))
            },
            |ctx, objects| {
                // If stream.[[controller]] implements ReadableByteStreamController, return ? ReadableByteStreamTee(stream).
                Self::readable_byte_stream_tee(ctx, objects)
            },
        )?;

        Ok(streams)
    }

    fn readable_stream_default_tee(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableStreamDefaultControllerOwned<'js>,
            UndefinedReader,
        >,
        clone_for_branch_2: bool,
    ) -> Result<(
        ReadableStreamPair<'js>,
        ReadableStreamObjects<
            'js,
            ReadableStreamDefaultControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
    )> {
        // Let reader be ? AcquireReadableStreamDefaultReader(stream).
        let (stream, reader) = ReadableStreamReaderClass::acquire_readable_stream_default_reader(
            ctx.clone(),
            objects.stream,
        )?;
        objects.stream = stream;
        // Let reading be false.
        let reading = Rc::new(AtomicBool::new(false));
        // Let readAgain be false.
        let read_again = Rc::new(AtomicBool::new(false));
        // Let canceled1 be false.
        // Let canceled2 be false.
        // Let reason1 be undefined.
        let reason_1 = Rc::new(OnceCell::new());
        // Let reason2 be undefined.
        let reason_2 = Rc::new(OnceCell::new());
        // Let branch1 be undefined.
        let branch_1: Rc<
            OnceCell<
                ReadableStreamClassObjects<
                    'js,
                    ReadableStreamDefaultControllerOwned<'js>,
                    UndefinedReader,
                >,
            >,
        > = Rc::new(OnceCell::new());
        // Let branch2 be undefined.
        let branch_2: Rc<
            OnceCell<
                ReadableStreamClassObjects<
                    'js,
                    ReadableStreamDefaultControllerOwned<'js>,
                    UndefinedReader,
                >,
            >,
        > = Rc::new(OnceCell::new());
        // Let cancelPromise be a new promise.
        let cancel_promise = ResolveablePromise::new(&ctx)?;

        // Let startAlgorithm be an algorithm that returns undefined.
        let start_algorithm = StartAlgorithm::ReturnUndefined;

        let objects_class = objects.into_inner().set_reader(reader);

        let pull_algorithm = PullAlgorithm::from_fn({
            let objects_class = objects_class.clone();
            let reason_1 = reason_1.clone();
            let reason_2 = reason_2.clone();
            let branch_1 = branch_1.clone();
            let branch_2 = branch_2.clone();
            let cancel_promise = cancel_promise.clone();
            move |ctx: Ctx<'js>, _| {
                let objects = ReadableStreamObjects::from_class(objects_class.clone());
                Self::readable_stream_default_pull_algorithm(
                    ctx,
                    objects,
                    clone_for_branch_2,
                    reading.clone(),
                    read_again.clone(),
                    reason_1.clone(),
                    reason_2.clone(),
                    branch_1.clone(),
                    branch_2.clone(),
                    cancel_promise.clone(),
                )
            }
        });
        let cancel_algorithm_1 = CancelAlgorithm::from_fn({
            let objects_class = objects_class.clone();
            let reason_1 = reason_1.clone();
            let reason_2 = reason_2.clone();
            let cancel_promise = cancel_promise.clone();
            move |reason: Value<'js>| {
                let objects = ReadableStreamObjects::from_class(objects_class.clone());
                Self::readable_stream_cancel_1_algorithm(
                    reason.ctx().clone(),
                    objects,
                    reason_1,
                    reason_2,
                    cancel_promise,
                    reason,
                )
            }
        });

        let cancel_algorithm_2 = CancelAlgorithm::from_fn({
            let objects_class = objects_class.clone();
            let reason_1 = reason_1.clone();
            let reason_2 = reason_2.clone();
            let cancel_promise = cancel_promise.clone();
            move |reason: Value<'js>| {
                let objects = ReadableStreamObjects::from_class(objects_class.clone());
                Self::readable_stream_cancel_2_algorithm(
                    reason.ctx().clone(),
                    objects,
                    reason_1,
                    reason_2,
                    cancel_promise,
                    reason,
                )
            }
        });

        // Set branch1 to ! CreateReadableStream(startAlgorithm, pullAlgorithm, cancel1Algorithm).
        let branch_1 = {
            let objects = Self::create_readable_stream(
                ctx.clone(),
                start_algorithm.clone(),
                pull_algorithm.clone(),
                cancel_algorithm_1,
                None,
                None,
            )?;
            _ = branch_1.set(objects.clone());
            objects
        };

        // Set branch2 to ! CreateReadableStream(startAlgorithm, pullAlgorithm, cancel2Algorithm).
        let branch_2 = {
            let objects = Self::create_readable_stream(
                ctx.clone(),
                start_algorithm,
                pull_algorithm,
                cancel_algorithm_2,
                None,
                None,
            )?;
            _ = branch_2.set(objects.clone());
            objects
        };

        upon_promise(
            ctx.clone(),
            objects_class
                .reader
                .borrow()
                .generic
                .closed_promise
                .promise
                .clone(),
            {
                let branch_1 = branch_1.clone();
                let branch_2 = branch_2.clone();
                move |ctx, result| match result {
                    Ok(()) => Ok(()),
                    // Upon rejection of reader.[[closedPromise]] with reason r,
                    Err(reason) => {
                        // Perform ! ReadableStreamDefaultControllerError(branch1.[[controller]], r).
                        let objects_1 =
                            ReadableStreamObjects::from_class_no_reader(branch_1).refresh_reader();
                        ReadableStreamDefaultController::readable_stream_default_controller_error(
                            objects_1,
                            reason.clone(),
                        )?;

                        // Perform ! ReadableStreamDefaultControllerError(branch2.[[controller]], r).
                        let objects_2 =
                            ReadableStreamObjects::from_class_no_reader(branch_2).refresh_reader();
                        ReadableStreamDefaultController::readable_stream_default_controller_error(
                            objects_2, reason,
                        )?;
                        // If canceled1 is false or canceled2 is false, resolve cancelPromise with undefined.
                        if reason_1.get().is_none() || reason_2.get().is_none() {
                            let () = cancel_promise.resolve(Value::new_undefined(ctx))?;
                        }

                        Ok(())
                    },
                }
            },
        )?;

        Ok((
            (branch_1.stream, branch_2.stream),
            ReadableStreamObjects::from_class(objects_class),
        ))
    }

    // Let pullAlgorithm be the following steps:
    #[allow(clippy::too_many_arguments)]
    fn readable_stream_default_pull_algorithm(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableStreamDefaultControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        >,
        clone_for_branch_2: bool,
        reading: Rc<AtomicBool>,
        read_again: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        branch_1: Rc<
            OnceCell<
                ReadableStreamClassObjects<
                    'js,
                    ReadableStreamDefaultControllerOwned<'js>,
                    UndefinedReader,
                >,
            >,
        >,
        branch_2: Rc<
            OnceCell<
                ReadableStreamClassObjects<
                    'js,
                    ReadableStreamDefaultControllerOwned<'js>,
                    UndefinedReader,
                >,
            >,
        >,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<Promise<'js>> {
        // If reading is true,
        if reading.load(Ordering::Relaxed) {
            // Set readAgain to true.
            read_again.store(true, Ordering::Relaxed);

            // Return a promise resolved with undefined.
            return promise_resolved_with(
                &ctx,
                &objects.stream.promise_primordials,
                Ok(Value::new_undefined(ctx.clone())),
            );
        }

        // Set reading to true.
        reading.store(true, Ordering::Relaxed);

        #[derive(Clone)]
        struct ReadRequest<'js> {
            clone_for_branch_2: bool,
            reading: Rc<AtomicBool>,
            read_again: Rc<AtomicBool>,
            reason_1: Rc<OnceCell<Value<'js>>>,
            reason_2: Rc<OnceCell<Value<'js>>>,
            branch_1: Rc<
                OnceCell<
                    ReadableStreamClassObjects<
                        'js,
                        ReadableStreamDefaultControllerOwned<'js>,
                        UndefinedReader,
                    >,
                >,
            >,
            branch_2: Rc<
                OnceCell<
                    ReadableStreamClassObjects<
                        'js,
                        ReadableStreamDefaultControllerOwned<'js>,
                        UndefinedReader,
                    >,
                >,
            >,
            cancel_promise: ResolveablePromise<'js>,

            structured_clone: Function<'js>,
        }

        impl<'js> Trace<'js> for ReadRequest<'js> {
            fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
                if let Some(r) = self.reason_1.get() {
                    r.trace(tracer)
                }
                if let Some(r) = self.reason_2.get() {
                    r.trace(tracer)
                }
                if let Some(b) = self.branch_1.get() {
                    b.trace(tracer)
                }
                if let Some(b) = self.branch_2.get() {
                    b.trace(tracer)
                }
                self.cancel_promise.trace(tracer);
            }
        }

        // Let readRequest be a read request with the following items:
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
                let this = self.clone();

                objects
                    .with_assert_default_controller(
                        |objects| {
                            let objects_class = objects.into_inner();
                            // Queue a microtask to perform the following steps:
                            let f = {
                                let ctx = ctx.clone();
                                let objects_class = objects_class.clone();
                                move || -> Result<()> {
                                    // Set readAgain to false.
                                    this.read_again.store(false, Ordering::Relaxed);

                                    // Let chunk1 and chunk2 be chunk.
                                    let chunk_1 = chunk.clone();
                                    let chunk_2 = chunk.clone();

                                    // If canceled2 is false and cloneForBranch2 is true,
                                    let chunk_2 = if this.reason_2.get().is_none() && this.clone_for_branch_2 {
                                        // Let cloneResult be StructuredClone(chunk2).
                                        let clone_result: Result<Value<'_>> = this.structured_clone
                                            .call((chunk_2,));
                                        match clone_result {
                                            // If cloneResult is an abrupt completion,
                                            Err(Error::Exception) => {
                                                let clone_result = ctx.catch();
                                                let objects_1 = ReadableStreamObjects::from_class(
                                                    this.branch_1.get().cloned().expect(
                                                        "canceled1 set without branch1 being initialised",
                                                    ),
                                                ).refresh_reader();

                                                // Perform ! ReadableStreamDefaultControllerError(branch1.[[controller]], cloneResult.[[Value]]).
                                                ReadableStreamDefaultController::readable_stream_default_controller_error(
                                                    objects_1,
                                                    clone_result.clone(),
                                                )?;

                                                let objects_2 = ReadableStreamObjects::from_class(
                                                    this.branch_2.get().cloned().clone().expect(
                                                        "canceled2 set without branch2 being initialised",
                                                    ),
                                                ).refresh_reader();

                                                // Perform ! ReadableStreamDefaultControllerError(branch2.[[controller]], cloneResult.[[Value]]).
                                                ReadableStreamDefaultController::readable_stream_default_controller_error(
                                                    objects_2,
                                                    clone_result.clone(),
                                                )?;

                                                // Resolve cancelPromise with ! ReadableStreamCancel(stream, cloneResult.[[Value]]).
                                                let (promise, _) =
                                                    ReadableStream::readable_stream_cancel(
                                                        ctx,
                                                        ReadableStreamObjects::from_class(objects_class),
                                                        clone_result,
                                                    )?;
                                                this.cancel_promise.resolve(promise)?;
                                                // Return.
                                                return Ok(());
                                            },
                                            Ok(clone_result) => {
                                                // Otherwise, set chunk2 to cloneResult.[[Value]].
                                                clone_result
                                            },
                                            Err(err) => return Err(err),
                                        }
                                    } else {
                                        chunk_2
                                    };

                                    // If canceled1 is false, perform ! ReadableStreamDefaultControllerEnqueue(branch1.[[controller]], chunk1).
                                    if this.reason_1.get().is_none() {
                                        let objects_1 = ReadableStreamObjects::from_class(
                                            this.branch_1
                                                .get()
                                                .cloned()
                                                .expect("canceled1 set without branch1 being initialised"),
                                        ).refresh_reader();

                                        ReadableStreamDefaultController::readable_stream_default_controller_enqueue(ctx.clone(), objects_1, chunk_1)?;
                                    }

                                    // If canceled2 is false, perform ! ReadableStreamDefaultControllerEnqueue(branch2.[[controller]], chunk2).
                                    if this.reason_2.get().is_none() {
                                        let objects_2 = ReadableStreamObjects::from_class(
                                            this.branch_2
                                                .get()
                                                .cloned()
                                                .expect("canceled2 set without branch2 being initialised"),
                                        ).refresh_reader();

                                        ReadableStreamDefaultController::readable_stream_default_controller_enqueue(ctx.clone(), objects_2, chunk_2)?;
                                    }

                                    // Set reading to false.
                                    this.reading.store(false, Ordering::Relaxed);

                                    // If readAgain is true, perform pullAlgorithm.
                                    if this.read_again.load(Ordering::Relaxed) {
                                        let objects = ReadableStreamObjects::from_class(objects_class);

                                        ReadableStream::readable_stream_default_pull_algorithm(
                                            ctx.clone(),
                                            objects,
                                            this.clone_for_branch_2,
                                            this.reading.clone(),
                                            this.read_again.clone(),
                                            this.reason_1.clone(),
                                            this.reason_2.clone(),
                                            this.branch_1.clone(),
                                            this.branch_2.clone(),
                                            this.cancel_promise.clone(),
                                        )?;
                                    }

                                    Ok(())
                                }
                            };

                            let () = Function::new(ctx, OnceFn::new(f))?.defer(())?;


                            Ok(ReadableStreamObjects::from_class(objects_class))
                        },
                    )
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
                // Set reading to false.
                self.reading.store(false, Ordering::Relaxed);
                // If canceled1 is false, perform ! ReadableStreamDefaultControllerClose(branch1.[[controller]]).
                if self.reason_1.get().is_none() {
                    let objects = ReadableStreamObjects::from_class(
                        self.branch_1
                            .get()
                            .expect("close called without branch1 being initialised")
                            .clone(),
                    )
                    .refresh_reader();

                    ReadableStreamDefaultController::readable_stream_default_controller_close(
                        ctx.clone(),
                        objects,
                    )?;
                }
                // If canceled2 is false, perform ! ReadableStreamDefaultControllerClose(branch2.[[controller]]).
                if self.reason_2.get().is_none() {
                    let objects = ReadableStreamObjects::from_class(
                        self.branch_2
                            .get()
                            .expect("close called without branch2 being initialised")
                            .clone(),
                    )
                    .refresh_reader();

                    ReadableStreamDefaultController::readable_stream_default_controller_close(
                        ctx.clone(),
                        objects,
                    )?;
                }
                // If canceled1 is false or canceled2 is false, resolve cancelPromise with undefined.
                if self.reason_1.get().is_none() || self.reason_2.get().is_none() {
                    self.cancel_promise
                        .resolve(Value::new_undefined(ctx.clone()))?
                }
                Ok(objects)
            }

            fn error_steps(
                &self,
                objects: ReadableStreamObjects<
                    'js,
                    ReadableStreamControllerOwned<'js>,
                    ReadableStreamDefaultReaderOwned<'js>,
                >,
                _: Value<'js>,
            ) -> Result<
                ReadableStreamObjects<
                    'js,
                    ReadableStreamControllerOwned<'js>,
                    ReadableStreamDefaultReaderOwned<'js>,
                >,
            > {
                // Set reading to false.
                self.reading.store(false, Ordering::Relaxed);
                Ok(objects)
            }
        }

        // Perform ! ReadableStreamDefaultReaderRead(reader, readRequest).
        objects = ReadableStreamDefaultReader::readable_stream_default_reader_read(
            &ctx,
            objects,
            ReadRequest {
                clone_for_branch_2,
                reading,
                read_again,
                reason_1,
                reason_2,
                branch_1,
                branch_2,
                cancel_promise,
                structured_clone: TeePrimordials::get(&ctx)?.structured_clone.clone(),
            },
        )?;

        // Return a promise resolved with undefined.
        promise_resolved_with(
            &ctx,
            &objects.stream.promise_primordials,
            Ok(Value::new_undefined(ctx.clone())),
        )
    }

    // Let cancel1Algorithm be the following steps, taking a reason argument:
    fn readable_stream_cancel_1_algorithm(
        ctx: Ctx<'js>,
        objects: ReadableStreamObjects<
            'js,
            impl ReadableStreamController<'js>,
            impl ReadableStreamReader<'js>,
        >,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        cancel_promise: ResolveablePromise<'js>,
        reason: Value<'js>,
    ) -> Result<Promise<'js>> {
        // Set canceled1 to true.
        // Set reason1 to reason.
        reason_1
            .set(reason.clone())
            .expect("First tee stream already has a cancel reason");

        // If canceled2 is true,
        if let Some(reason_2) = reason_2.get().cloned() {
            // Let compositeReason be ! CreateArrayFromList(« reason1, reason2 »).
            let composite_reason = List((reason, reason_2));
            // Let cancelResult be ! ReadableStreamCancel(stream, compositeReason).
            let (cancel_result, _) = ReadableStream::readable_stream_cancel(
                ctx.clone(),
                objects,
                composite_reason.into_js(&ctx)?,
            )?;
            // Resolve cancelPromise with cancelResult.
            cancel_promise.resolve(cancel_result)?;
        }

        // Return cancelPromise.
        Ok(cancel_promise.promise)
    }

    // Let cancel2Algorithm be the following steps, taking a reason argument:
    #[allow(clippy::too_many_arguments)]
    fn readable_stream_cancel_2_algorithm(
        ctx: Ctx<'js>,
        objects: ReadableStreamObjects<
            'js,
            impl ReadableStreamController<'js>,
            impl ReadableStreamReader<'js>,
        >,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        cancel_promise: ResolveablePromise<'js>,
        reason: Value<'js>,
    ) -> Result<Promise<'js>> {
        // Set canceled2 to true.
        // Set reason2 to reason.
        reason_2
            .set(reason.clone())
            .expect("Second tee stream already has a cancel reason");

        // If canceled1 is true,
        if let Some(reason_1) = reason_1.get().cloned() {
            // Let compositeReason be ! CreateArrayFromList(« reason1, reason2 »).
            let composite_reason = List((reason_1, reason));
            // Let cancelResult be ! ReadableStreamCancel(stream, compositeReason).
            let (cancel_result, _) = ReadableStream::readable_stream_cancel(
                ctx.clone(),
                objects,
                composite_reason.into_js(&ctx)?,
            )?;
            // Resolve cancelPromise with cancelResult.
            let () = cancel_promise.resolve(cancel_result)?;
        }

        // Return cancelPromise.
        Ok(cancel_promise.promise)
    }

    fn readable_byte_stream_tee(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
    ) -> Result<(
        ReadableStreamPair<'js>,
        ReadableStreamObjects<'js, ReadableByteStreamControllerOwned<'js>, UndefinedReader>,
    )> {
        // Let reader be ? AcquireReadableStreamDefaultReader(stream).
        let (stream, reader) = ReadableStreamReaderClass::acquire_readable_stream_default_reader(
            ctx.clone(),
            objects.stream,
        )?;
        objects.stream = stream;
        let reader: Rc<RefCell<ReadableStreamReaderClass<'js>>> =
            Rc::new(RefCell::new(reader.into()));
        // Let reading be false.
        let reading = Rc::new(AtomicBool::new(false));
        // Let readAgainForBranch1 be false.
        let read_again_for_branch_1 = Rc::new(AtomicBool::new(false));
        // Let readAgainForBranch2 be false.
        let read_again_for_branch_2 = Rc::new(AtomicBool::new(false));
        // Let canceled1 be false.
        // Let canceled2 be false.
        // Let reason1 be undefined.
        let reason_1 = Rc::new(OnceCell::new());
        // Let reason2 be undefined.
        let reason_2 = Rc::new(OnceCell::new());
        // Let branch1 be undefined.
        let branch_1: Rc<OnceCell<Class<'js, Self>>> = Rc::new(OnceCell::new());
        // Let branch2 be undefined.
        let branch_2: Rc<OnceCell<Class<'js, Self>>> = Rc::new(OnceCell::new());
        // Let cancelPromise be a new promise.
        let cancel_promise = ResolveablePromise::new(&ctx)?;

        let objects_class = objects.into_inner();

        // Let pull1Algorithm be the following steps:
        let pull_1_algorithm = PullAlgorithm::from_fn({
            let objects_class = objects_class.clone();
            let reader = reader.clone();
            let reading = reading.clone();
            let read_again_for_branch_1 = read_again_for_branch_1.clone();
            let read_again_for_branch_2 = read_again_for_branch_2.clone();
            let reason_1 = reason_1.clone();
            let reason_2 = reason_2.clone();
            let branch_1 = branch_1.clone();
            let branch_2 = branch_2.clone();
            let cancel_promise = cancel_promise.clone();
            move |ctx, branch_1_controller| {
                let objects = ReadableStreamObjects::from_class(objects_class.clone());

                let branch_1_controller = OwnedBorrowMut::from_class(match branch_1_controller {
                    ReadableStreamControllerClass::ReadableStreamByteController(c) => c,
                    _ => panic!(
                        "ReadableByteStream tee pull1 algorithm called without branch1 having a byte controller"
                    ),
                });

                let branch_2 = OwnedBorrowMut::from_class(branch_2.get().cloned().expect("ReadableByteStream tee pull1 algorithm called without branch2 being initialised"));
                let branch_2_controller = match branch_2.controller {
                    ReadableStreamControllerClass::ReadableStreamByteController(ref c) => {
                        OwnedBorrowMut::from_class(c.clone())
                    },
                    _ => {
                        panic!("ReadableByteStream tee pull1 algorithm called without branch2 having a byte controller")
                    },
                };

                Self::readable_byte_stream_pull_1_algorithm(
                        ctx,
                        objects,
                        reader.clone(),
                        reading.clone(),
                        read_again_for_branch_1.clone(),
                        read_again_for_branch_2.clone(),
                        reason_1.clone(),
                        reason_2.clone(),
                        ReadableStreamObjects {
                           stream: OwnedBorrowMut::from_class(branch_1.get().cloned().expect("ReadableByteStream tee pull1 algorithm called without branch1 being initialised")),
                           controller: branch_1_controller,
                           reader: UndefinedReader,
                        },
                        ReadableStreamObjects {
                           stream: branch_2,
                           controller: branch_2_controller,
                           reader: UndefinedReader,
                        },
                        cancel_promise.clone(),
                    )
            }
        });

        // Let pull2Algorithm be the following steps:
        let pull_2_algorithm = PullAlgorithm::from_fn({
            let objects_class = objects_class.clone();
            let reader = reader.clone();
            let reading = reading.clone();
            let read_again_for_branch_1 = read_again_for_branch_1.clone();
            let read_again_for_branch_2 = read_again_for_branch_2.clone();
            let reason_1 = reason_1.clone();
            let reason_2 = reason_2.clone();
            let branch_1 = branch_1.clone();
            let branch_2 = branch_2.clone();
            let cancel_promise = cancel_promise.clone();
            move |ctx, branch_2_controller| {
                let objects = ReadableStreamObjects::from_class(objects_class.clone());

                let branch_2 = OwnedBorrowMut::from_class(branch_2.get().cloned().expect("ReadableByteStream tee pull2 algorithm called without branch2 being initialised"));
                let branch_2_controller = match branch_2_controller {
                    ReadableStreamControllerClass::ReadableStreamByteController(ref c) => {
                        OwnedBorrowMut::from_class(c.clone())
                    },
                    _ => {
                        panic!("ReadableByteStream tee pull2 algorithm called without branch2 having a byte controller")
                    },
                };

                let branch_1 = OwnedBorrowMut::from_class(branch_1.get().cloned().expect("ReadableByteStream tee pull2 algorithm called without branch1 being initialised"));
                let branch_1_controller = match branch_1.controller {
                    ReadableStreamControllerClass::ReadableStreamByteController(ref c) => {
                        OwnedBorrowMut::from_class(c.clone())
                    },
                    _ => {
                        panic!("ReadableByteStream tee pull2 algorithm called without branch1 having a byte controller")
                    },
                };
                Self::readable_byte_stream_pull_2_algorithm(
                    ctx,
                    objects,
                    reader.clone(),
                    reading.clone(),
                    read_again_for_branch_1.clone(),
                    read_again_for_branch_2.clone(),
                    reason_1.clone(),
                    reason_2.clone(),
                    ReadableStreamObjects {
                        stream: branch_1,
                        controller: branch_1_controller,
                        reader: UndefinedReader,
                    },
                    ReadableStreamObjects {
                        stream: branch_2,
                        controller: branch_2_controller,
                        reader: UndefinedReader,
                    },
                    cancel_promise.clone(),
                )
            }
        });

        let cancel_algorithm_1 = CancelAlgorithm::from_fn({
            let objects_class = objects_class.clone();
            let reader = reader.clone();
            let reason_1 = reason_1.clone();
            let reason_2 = reason_2.clone();
            let cancel_promise = cancel_promise.clone();
            move |reason: Value<'js>| {
                let reader = ReadableStreamReaderOwned::from_class(reader.borrow().clone());
                let objects = ReadableStreamObjects::from_class(objects_class).set_reader(reader);
                Self::readable_stream_cancel_1_algorithm(
                    reason.ctx().clone(),
                    objects,
                    reason_1,
                    reason_2,
                    cancel_promise,
                    reason,
                )
            }
        });

        let cancel_algorithm_2 = CancelAlgorithm::from_fn({
            let objects_class = objects_class.clone();
            let reader = reader.clone();
            let reason_1 = reason_1.clone();
            let reason_2 = reason_2.clone();
            let cancel_promise = cancel_promise.clone();
            move |reason: Value<'js>| {
                let reader = ReadableStreamReaderOwned::from_class(reader.borrow().clone());
                let objects = ReadableStreamObjects::from_class(objects_class).set_reader(reader);
                Self::readable_stream_cancel_2_algorithm(
                    reason.ctx().clone(),
                    objects,
                    reason_1,
                    reason_2,
                    cancel_promise,
                    reason,
                )
            }
        });

        // Let startAlgorithm be an algorithm that returns undefined.
        let start_algorithm = StartAlgorithm::ReturnUndefined;

        // Set branch1 to ! CreateReadableByteStream(startAlgorithm, pull1Algorithm, cancel1Algorithm).
        let objects_1 = {
            let (s, c) = Self::create_readable_byte_stream(
                ctx.clone(),
                start_algorithm.clone(),
                pull_1_algorithm.clone(),
                cancel_algorithm_1,
            )?;
            _ = branch_1.set(s.clone());
            ReadableStreamClassObjects {
                stream: s,
                controller: c,
                reader: UndefinedReader,
            }
        };

        // Set branch2 to ! CreateReadableByteStream(startAlgorithm, pull2Algorithm, cancel2Algorithm).
        let objects_2 = {
            let (s, c) = Self::create_readable_byte_stream(
                ctx.clone(),
                start_algorithm,
                pull_2_algorithm,
                cancel_algorithm_2,
            )?;
            _ = branch_2.set(s.clone());
            ReadableStreamClassObjects {
                stream: s,
                controller: c,
                reader: UndefinedReader,
            }
        };

        // Perform forwardReaderError, given reader.
        let this_reader = reader.borrow().clone();
        Self::readable_byte_stream_forward_reader_error(
            ctx,
            reader,
            objects_1.clone(),
            objects_2.clone(),
            reason_1,
            reason_2,
            this_reader,
            cancel_promise,
        )?;

        // Return « branch1, branch2 ».
        Ok((
            (objects_1.stream, objects_2.stream),
            ReadableStreamObjects::from_class(objects_class),
        ))
    }

    // Let forwardReaderError be the following steps, taking a thisReader argument:
    #[allow(clippy::too_many_arguments)]
    fn readable_byte_stream_forward_reader_error(
        ctx: Ctx<'js>,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        objects_1: ReadableStreamClassObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        objects_2: ReadableStreamClassObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        this_reader: ReadableStreamReaderClass<'js>,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<()> {
        // Upon rejection of thisReader.[[closedPromise]] with reason r,
        upon_promise(
            ctx,
            this_reader.closed_promise(),
            move |ctx, result| match result {
                Err(r) => {
                    // If thisReader is not reader, return.
                    if !reader.borrow().eq(&this_reader) {
                        return Ok(());
                    }

                    let objects_1 =
                        ReadableStreamObjects::from_class_no_reader(objects_1).refresh_reader();

                    // Perform ! ReadableByteStreamControllerError(branch1.[[controller]], r).
                    ReadableByteStreamController::readable_byte_stream_controller_error(
                        objects_1,
                        r.clone(),
                    )?;

                    let objects_2 =
                        ReadableStreamObjects::from_class_no_reader(objects_2).refresh_reader();

                    // Perform ! ReadableByteStreamControllerError(branch2.[[controller]], r).
                    ReadableByteStreamController::readable_byte_stream_controller_error(
                        objects_2,
                        r.clone(),
                    )?;

                    // If canceled1 is false or canceled2 is false, resolve cancelPromise with undefined.
                    if reason_1.get().is_none() || reason_2.get().is_none() {
                        let () = cancel_promise.resolve(Value::new_undefined(ctx))?;
                    }
                    Ok(())
                },
                Ok(()) => Ok(()),
            },
        )?;
        Ok(())
    }

    // Let pullWithDefaultReader be the following steps:
    #[allow(clippy::too_many_arguments)]
    fn readable_byte_stream_pull_with_default_reader(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        objects_1: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        objects_2: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<ReadableStreamObjects<'js, ReadableByteStreamControllerOwned<'js>, UndefinedReader>>
    {
        let objects_class_1 = objects_1.into_inner();
        let objects_class_2 = objects_2.into_inner();

        // If reader implements ReadableStreamBYOBReader,
        let current_reader = reader.borrow().clone();
        let current_reader = match current_reader {
            ReadableStreamReaderClass::ReadableStreamBYOBReader(r) => {
                let byob_reader = OwnedBorrowMut::from_class(r.clone());

                // Perform ! ReadableStreamBYOBReaderRelease(reader).
                objects = ReadableStreamBYOBReader::readable_stream_byob_reader_release(
                    objects.set_reader(byob_reader),
                )?
                .clear_reader();
                // Set reader to ! AcquireReadableStreamDefaultReader(stream).
                let (s, new_reader) =
                    ReadableStreamReaderClass::acquire_readable_stream_default_reader(
                        ctx.clone(),
                        objects.stream,
                    )?;
                objects.stream = s;
                reader.replace(new_reader.clone().into());

                // Perform forwardReaderError, given reader.
                Self::readable_byte_stream_forward_reader_error(
                    ctx.clone(),
                    reader.clone(),
                    objects_class_1.clone(),
                    objects_class_2.clone(),
                    reason_1.clone(),
                    reason_2.clone(),
                    new_reader.clone().into(),
                    cancel_promise.clone(),
                )?;
                new_reader
            },
            ReadableStreamReaderClass::ReadableStreamDefaultReader(r) => r,
        };

        // Let readRequest be a read request with the following items:
        #[derive(Clone)]
        struct ReadRequest<'js> {
            reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
            reading: Rc<AtomicBool>,
            read_again_for_branch_1: Rc<AtomicBool>,
            read_again_for_branch_2: Rc<AtomicBool>,
            reason_1: Rc<OnceCell<Value<'js>>>,
            reason_2: Rc<OnceCell<Value<'js>>>,
            objects_class_1: ReadableStreamClassObjects<
                'js,
                ReadableByteStreamControllerOwned<'js>,
                UndefinedReader,
            >,
            objects_class_2: ReadableStreamClassObjects<
                'js,
                ReadableByteStreamControllerOwned<'js>,
                UndefinedReader,
            >,
            cancel_promise: ResolveablePromise<'js>,
        }

        impl<'js> Trace<'js> for ReadRequest<'js> {
            fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
                if let Ok(r) = self.reader.try_borrow() {
                    r.trace(tracer)
                }
                if let Some(r) = self.reason_1.get() {
                    r.trace(tracer)
                }
                if let Some(r) = self.reason_2.get() {
                    r.trace(tracer)
                }
                self.objects_class_1.trace(tracer);
                self.objects_class_2.trace(tracer);
                self.cancel_promise.trace(tracer);
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
                let this = self.clone();

                objects.with_assert_byte_controller(|objects| {
                    let constructor_uint8array = objects.controller.constructor_uint8array.clone();
                    let function_array_buffer_is_view = objects.controller.function_array_buffer_is_view.clone();
                    let chunk = ViewBytes::from_value(&ctx, &function_array_buffer_is_view, &chunk)?;
                    let objects_class = objects.into_inner();
                    // Queue a microtask to perform the following steps:
                    let f = {
                        let ctx = ctx.clone();
                        let objects_class = objects_class.clone();
                        move || -> Result<()> {
                            // Set readAgainForBranch1 to false.
                            this.read_again_for_branch_1.store(false, Ordering::Relaxed);
                            // Set readAgainForBranch2 to false.
                            this.read_again_for_branch_2.store(false, Ordering::Relaxed);

                            // Let chunk1 and chunk2 be chunk.
                            let chunk_1 = chunk.clone();
                            let mut chunk_2 = chunk.clone();

                            // If canceled1 is false and canceled2 is false,
                            if this.reason_1.get().is_none() && this.reason_2.get().is_none() {
                                // Let cloneResult be CloneAsUint8Array(chunk).
                                match clone_as_uint8_array(ctx.clone(), &constructor_uint8array, &function_array_buffer_is_view, chunk) {
                                    // If cloneResult is an abrupt completion,
                                    Err(Error::Exception) => {
                                        let err = ctx.catch();

                                        let objects_1 =
                                            ReadableStreamObjects::from_class(this.objects_class_1);

                                        // Perform ! ReadableByteStreamControllerError(branch1.[[controller]], cloneResult.[[Value]]).
                                        ReadableByteStreamController::readable_byte_stream_controller_error(
                                                objects_1,
                                                err.clone(),
                                            )?;

                                        let objects_2 =
                                            ReadableStreamObjects::from_class(this.objects_class_2);

                                        // Perform ! ReadableByteStreamControllerError(branch2.[[controller]], cloneResult.[[Value]]).
                                        ReadableByteStreamController::readable_byte_stream_controller_error(
                                                objects_2,
                                                err.clone(),
                                            )?;

                                        // Resolve cancelPromise with ! ReadableStreamCancel(stream, cloneResult.[[Value]]).
                                        let (promise, _) = ReadableStream::readable_stream_cancel(
                                            ctx,
                                            ReadableStreamObjects::from_class(objects_class),
                                            err.clone(),
                                        )?;
                                        this.cancel_promise.resolve(promise)?;

                                        // Return.
                                        return Ok(());
                                    },
                                    // Otherwise, set chunk2 to cloneResult.[[Value]].
                                    Ok(clone_result) => chunk_2 = clone_result,
                                    Err(err) => return Err(err),
                                };
                            }

                            // If canceled1 is false, perform ! ReadableByteStreamControllerEnqueue(branch1.[[controller]], chunk1).
                            if this.reason_1.get().is_none() {
                                let objects_1 = ReadableStreamObjects::from_class_no_reader(
                                    this.objects_class_1.clone(),
                                ).refresh_reader();
                                ReadableByteStreamController::readable_byte_stream_controller_enqueue(
                                    &ctx, objects_1, chunk_1,
                                )?;
                            }

                            // If canceled2 is false, perform ! ReadableByteStreamControllerEnqueue(branch2.[[controller]], chunk2).
                            if this.reason_2.get().is_none() {
                                let objects_2 = ReadableStreamObjects::from_class_no_reader(
                                    this.objects_class_2.clone(),
                                ).refresh_reader();
                                ReadableByteStreamController::readable_byte_stream_controller_enqueue(
                                    &ctx, objects_2, chunk_2,
                                )?;
                            }

                            // Set reading to false.
                            this.reading.store(false, Ordering::Relaxed);

                            let objects_1 = ReadableStreamObjects::from_class(this.objects_class_1);
                            let objects_2 = ReadableStreamObjects::from_class(this.objects_class_2);

                            let objects = ReadableStreamObjects::from_class_no_reader(objects_class);

                            // If readAgainForBranch1 is true, perform pull1Algorithm.
                            if this.read_again_for_branch_1.load(Ordering::Relaxed) {
                                ReadableStream::readable_byte_stream_pull_1_algorithm(
                                    ctx.clone(),
                                    objects,
                                    this.reader,
                                    this.reading,
                                    this.read_again_for_branch_1,
                                    this.read_again_for_branch_2,
                                    this.reason_1,
                                    this.reason_2,
                                    objects_1,
                                    objects_2,
                                    this.cancel_promise,
                                )?;
                            } else if this.read_again_for_branch_2.load(Ordering::Relaxed) {
                                // Otherwise, if readAgainForBranch2 is true, perform pull2Algorithm.
                                ReadableStream::readable_byte_stream_pull_2_algorithm(
                                    ctx.clone(),
                                    objects,
                                    this.reader,
                                    this.reading,
                                    this.read_again_for_branch_1,
                                    this.read_again_for_branch_2,
                                    this.reason_1,
                                    this.reason_2,
                                    objects_1,
                                    objects_2,
                                    this.cancel_promise,
                                )?;
                            }

                            Ok(())
                        }
                    };

                    let () = Function::new(ctx, OnceFn::new(f))?.defer(())?;

                    let objects = ReadableStreamObjects::from_class(objects_class);

                    Ok(objects)
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
            > {
                // Set reading to false.
                self.reading.store(false, Ordering::Relaxed);

                let mut objects_1 =
                    ReadableStreamObjects::from_class_no_reader(self.objects_class_1.clone())
                        .refresh_reader();

                let mut objects_2 =
                    ReadableStreamObjects::from_class_no_reader(self.objects_class_2.clone())
                        .refresh_reader();

                // If canceled1 is false, perform ! ReadableByteStreamControllerClose(branch1.[[controller]]).
                if self.reason_1.get().is_none() {
                    objects_1 =
                        ReadableByteStreamController::readable_byte_stream_controller_close(
                            ctx.clone(),
                            objects_1,
                        )?;
                }
                // If canceled2 is false, perform ! ReadableByteStreamControllerClose(branch2.[[controller]]).
                if self.reason_2.get().is_none() {
                    objects_2 =
                        ReadableByteStreamController::readable_byte_stream_controller_close(
                            ctx.clone(),
                            objects_2,
                        )?;
                }
                // If branch1.[[controller]].[[pendingPullIntos]] is not empty, perform ! ReadableByteStreamControllerRespond(branch1.[[controller]], 0).
                if !objects_1.controller.pending_pull_intos.is_empty() {
                    ReadableByteStreamController::readable_byte_stream_controller_respond(
                        ctx.clone(),
                        objects_1,
                        0,
                    )?
                }

                // If branch2.[[controller]].[[pendingPullIntos]] is not empty, perform ! ReadableByteStreamControllerRespond(branch2.[[controller]], 0).
                if !objects_2.controller.pending_pull_intos.is_empty() {
                    ReadableByteStreamController::readable_byte_stream_controller_respond(
                        ctx.clone(),
                        objects_2,
                        0,
                    )?
                }

                // If canceled1 is false or canceled2 is false, resolve cancelPromise with undefined.
                if self.reason_1.get().is_none() || self.reason_2.get().is_none() {
                    self.cancel_promise
                        .resolve(Value::new_undefined(ctx.clone()))?
                }
                Ok(objects)
            }

            fn error_steps(
                &self,
                objects: ReadableStreamObjects<
                    'js,
                    ReadableStreamControllerOwned<'js>,
                    ReadableStreamDefaultReaderOwned<'js>,
                >,
                _: Value<'js>,
            ) -> Result<
                ReadableStreamObjects<
                    'js,
                    ReadableStreamControllerOwned<'js>,
                    ReadableStreamDefaultReaderOwned<'js>,
                >,
            > {
                // Set reading to false.
                self.reading.store(false, Ordering::Relaxed);
                Ok(objects)
            }
        }

        // Perform ! ReadableStreamDefaultReaderRead(reader, readRequest).
        Ok(
            ReadableStreamDefaultReader::readable_stream_default_reader_read(
                &ctx,
                ReadableStreamObjects {
                    stream: objects.stream,
                    controller: objects.controller,
                    reader: OwnedBorrowMut::from_class(current_reader),
                },
                ReadRequest {
                    reader,
                    reading,
                    read_again_for_branch_1,
                    read_again_for_branch_2,
                    reason_1,
                    reason_2,
                    objects_class_1,
                    objects_class_2,
                    cancel_promise,
                },
            )?
            .clear_reader(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn readable_byte_stream_pull_with_byob_reader(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        objects_1: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        objects_2: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        cancel_promise: ResolveablePromise<'js>,
        view: ViewBytes<'js>,
        for_branch_2: bool,
    ) -> Result<ReadableStreamObjects<'js, ReadableByteStreamControllerOwned<'js>, UndefinedReader>>
    {
        let objects_1 = objects_1.into_inner();
        let objects_2 = objects_2.into_inner();

        // If reader implements ReadableStreamDefaultReader,
        let current_reader = reader.borrow().clone();
        let current_reader = match current_reader {
            ReadableStreamReaderClass::ReadableStreamDefaultReader(r) => {
                let default_reader = OwnedBorrowMut::from_class(r.clone());

                // Perform ! ReadableStreamDefaultReaderRelease(reader).
                objects = ReadableStreamDefaultReader::readable_stream_default_reader_release(
                    objects.set_reader(default_reader),
                )?
                .clear_reader();

                // Set reader to ! AcquireReadableStreamBYOBReader(stream).
                let (s, new_reader) =
                    ReadableStreamReaderClass::acquire_readable_stream_byob_reader(
                        ctx.clone(),
                        objects.stream,
                    )?;
                objects.stream = s;
                reader.replace(new_reader.clone().into());

                // Perform forwardReaderError, given reader.
                Self::readable_byte_stream_forward_reader_error(
                    ctx.clone(),
                    reader.clone(),
                    objects_1.clone(),
                    objects_2.clone(),
                    reason_1.clone(),
                    reason_2.clone(),
                    new_reader.clone().into(),
                    cancel_promise.clone(),
                )?;

                new_reader
            },
            ReadableStreamReaderClass::ReadableStreamBYOBReader(r) => r.clone(),
        };

        // Let byobBranch be branch2 if forBranch2 is true, and branch1 otherwise.
        // Let otherBranch be branch2 if forBranch2 is false, and branch1 otherwise.
        let (byob_objects, other_objects) = if for_branch_2 {
            (objects_2.clone(), objects_1.clone())
        } else {
            (objects_1.clone(), objects_2.clone())
        };

        // Let readIntoRequest be a read-into request with the following items:
        #[derive(Clone)]
        struct ReadIntoRequest<'js> {
            reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
            reading: Rc<AtomicBool>,
            read_again_for_branch_1: Rc<AtomicBool>,
            read_again_for_branch_2: Rc<AtomicBool>,
            reason_1: Rc<OnceCell<Value<'js>>>,
            reason_2: Rc<OnceCell<Value<'js>>>,
            objects_1: ReadableStreamClassObjects<
                'js,
                ReadableByteStreamControllerOwned<'js>,
                UndefinedReader,
            >,
            objects_2: ReadableStreamClassObjects<
                'js,
                ReadableByteStreamControllerOwned<'js>,
                UndefinedReader,
            >,
            byob_objects: ReadableStreamClassObjects<
                'js,
                ReadableByteStreamControllerOwned<'js>,
                UndefinedReader,
            >,
            other_objects: ReadableStreamClassObjects<
                'js,
                ReadableByteStreamControllerOwned<'js>,
                UndefinedReader,
            >,
            cancel_promise: ResolveablePromise<'js>,
            for_branch_2: bool,
        }

        impl<'js> Trace<'js> for ReadIntoRequest<'js> {
            fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
                if let Ok(r) = self.reader.try_borrow() {
                    r.trace(tracer)
                }
                if let Some(r) = self.reason_1.get() {
                    r.trace(tracer)
                }
                if let Some(r) = self.reason_2.get() {
                    r.trace(tracer)
                }
                self.objects_1.trace(tracer);
                self.objects_2.trace(tracer);
                self.byob_objects.trace(tracer);
                self.other_objects.trace(tracer);
                self.cancel_promise.trace(tracer);
            }
        }

        impl<'js> ReadableStreamReadIntoRequest<'js> for ReadIntoRequest<'js> {
            fn chunk_steps(
                &self,
                objects: ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
                chunk: Value<'js>,
            ) -> Result<
                ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
            > {
                let ctx = chunk.ctx().clone();

                let constructor_uint8array = objects.controller.constructor_uint8array.clone();
                let function_array_buffer_is_view =
                    objects.controller.function_array_buffer_is_view.clone();
                let chunk = ViewBytes::from_value(&ctx, &function_array_buffer_is_view, &chunk)?;

                let objects_class = objects.into_inner();

                // Queue a microtask to perform the following steps:
                let f = {
                    let ctx = ctx.clone();
                    let objects_class = objects_class.clone();
                    let this = self.clone();
                    move || -> Result<()> {
                        // Set readAgainForBranch1 to false.
                        this.read_again_for_branch_1.store(false, Ordering::Relaxed);
                        // Set readAgainForBranch2 to false.
                        this.read_again_for_branch_2.store(false, Ordering::Relaxed);

                        // Let byobCanceled be canceled2 if forBranch2 is true, and canceled1 otherwise.
                        // Let otherCanceled be canceled2 if forBranch2 is false, and canceled1 otherwise.
                        let (byob_canceled, other_canceled) = if this.for_branch_2 {
                            (this.reason_2.get().is_some(), this.reason_1.get().is_some())
                        } else {
                            (this.reason_1.get().is_some(), this.reason_2.get().is_some())
                        };

                        // If otherCanceled is false,
                        if !other_canceled {
                            // Let cloneResult be CloneAsUint8Array(chunk).
                            match clone_as_uint8_array(
                                ctx.clone(),
                                &constructor_uint8array,
                                &function_array_buffer_is_view,
                                chunk.clone(),
                            ) {
                                // If cloneResult is an abrupt completion,
                                Err(Error::Exception) => {
                                    let err = ctx.catch();

                                    let byob_objects = ReadableStreamObjects::from_class_no_reader(
                                        this.byob_objects.clone(),
                                    )
                                    .refresh_reader();

                                    // Perform ! ReadableByteStreamControllerError(byobBranch.[[controller]], cloneResult.[[Value]]).
                                    ReadableByteStreamController::readable_byte_stream_controller_error(
                                        byob_objects,
                                        err.clone(),
                                    )?;

                                    let other_objects =
                                        ReadableStreamObjects::from_class_no_reader(
                                            this.other_objects.clone(),
                                        )
                                        .refresh_reader();

                                    // Perform ! ReadableByteStreamControllerError(otherBranch.[[controller]], cloneResult.[[Value]]).
                                    ReadableByteStreamController::readable_byte_stream_controller_error(
                                        other_objects,
                                        err.clone(),
                                    )?;

                                    // Resolve cancelPromise with ! ReadableStreamCancel(stream, cloneResult.[[Value]]).
                                    let (promise, _) = ReadableStream::readable_stream_cancel(
                                        ctx,
                                        ReadableStreamObjects::from_class(objects_class),
                                        err.clone(),
                                    )?;
                                    this.cancel_promise.resolve(promise)?;

                                    // Return.
                                    return Ok(());
                                },
                                // Otherwise, let clonedChunk be cloneResult.[[Value]].
                                Ok(cloned_chunk) => {
                                    // If byobCanceled is false, perform ! ReadableByteStreamControllerRespondWithNewView(byobBranch.[[controller]], chunk).
                                    if !byob_canceled {
                                        let byob_objects =
                                            ReadableStreamObjects::from_class_no_reader(
                                                this.byob_objects.clone(),
                                            )
                                            .refresh_reader();

                                        ReadableByteStreamController::readable_byte_stream_controller_respond_with_new_view(ctx.clone(), byob_objects, chunk)?;
                                    }

                                    let other_objects =
                                        ReadableStreamObjects::from_class_no_reader(
                                            this.other_objects.clone(),
                                        )
                                        .refresh_reader();

                                    // Perform ! ReadableByteStreamControllerEnqueue(otherBranch.[[controller]], clonedChunk).
                                    ReadableByteStreamController::readable_byte_stream_controller_enqueue(&ctx, other_objects, cloned_chunk)?;
                                },
                                Err(err) => return Err(err),
                            };
                        } else if !byob_canceled {
                            let byob_objects = ReadableStreamObjects::from_class_no_reader(
                                this.byob_objects.clone(),
                            )
                            .refresh_reader();

                            // Otherwise, if byobCanceled is false, perform ! ReadableByteStreamControllerRespondWithNewView(byobBranch.[[controller]], chunk).
                            ReadableByteStreamController::readable_byte_stream_controller_respond_with_new_view(ctx.clone(), byob_objects, chunk)?;
                        }

                        let objects_1 = ReadableStreamObjects::from_class(this.objects_1.clone());
                        let objects_2 = ReadableStreamObjects::from_class(this.objects_2.clone());

                        // Set reading to false.
                        this.reading.store(false, Ordering::Relaxed);

                        // If readAgainForBranch1 is true, perform pull1Algorithm.
                        if this.read_again_for_branch_1.load(Ordering::Relaxed) {
                            ReadableStream::readable_byte_stream_pull_1_algorithm(
                                ctx.clone(),
                                ReadableStreamObjects::from_class(objects_class).clear_reader(),
                                this.reader.clone(),
                                this.reading.clone(),
                                this.read_again_for_branch_1.clone(),
                                this.read_again_for_branch_2.clone(),
                                this.reason_1.clone(),
                                this.reason_2.clone(),
                                objects_1,
                                objects_2,
                                this.cancel_promise.clone(),
                            )?;
                        } else if this.read_again_for_branch_2.load(Ordering::Relaxed) {
                            // Otherwise, if readAgainForBranch2 is true, perform pull2Algorithm.
                            ReadableStream::readable_byte_stream_pull_2_algorithm(
                                ctx.clone(),
                                ReadableStreamObjects::from_class(objects_class).clear_reader(),
                                this.reader.clone(),
                                this.reading.clone(),
                                this.read_again_for_branch_1.clone(),
                                this.read_again_for_branch_2.clone(),
                                this.reason_1.clone(),
                                this.reason_2.clone(),
                                objects_1,
                                objects_2,
                                this.cancel_promise.clone(),
                            )?;
                        }

                        Ok(())
                    }
                };

                let () = Function::new(ctx, OnceFn::new(f))?.defer(())?;

                let objects = ReadableStreamObjects::from_class(objects_class);

                Ok(objects)
            }

            fn close_steps(
                &self,
                objects: ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
                chunk: Value<'js>,
            ) -> Result<
                ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
            > {
                let ctx = chunk.ctx().clone();

                // Set reading to false.
                self.reading.store(false, Ordering::Relaxed);

                // Let byobCanceled be canceled2 if forBranch2 is true, and canceled1 otherwise.
                // Let otherCanceled be canceled2 if forBranch2 is false, and canceled1 otherwise.
                let (byob_canceled, other_canceled) = if self.for_branch_2 {
                    (self.reason_2.get().is_some(), self.reason_1.get().is_some())
                } else {
                    (self.reason_1.get().is_some(), self.reason_2.get().is_some())
                };

                // If byobCanceled is false, perform ! ReadableByteStreamControllerClose(byobBranch.[[controller]]).
                if !byob_canceled {
                    let byob_objects =
                        ReadableStreamObjects::from_class_no_reader(self.byob_objects.clone())
                            .refresh_reader();

                    ReadableByteStreamController::readable_byte_stream_controller_close(
                        ctx.clone(),
                        byob_objects,
                    )?;
                }
                // If otherCanceled is false, perform ! ReadableByteStreamControllerClose(otherBranch.[[controller]]).
                if !other_canceled {
                    let other_objects =
                        ReadableStreamObjects::from_class_no_reader(self.other_objects.clone())
                            .refresh_reader();

                    ReadableByteStreamController::readable_byte_stream_controller_close(
                        ctx.clone(),
                        other_objects,
                    )?;
                }

                // If chunk is not undefined,
                if !chunk.is_undefined() {
                    let chunk = ViewBytes::from_value(
                        &ctx,
                        &objects.controller.function_array_buffer_is_view,
                        &chunk,
                    )?;

                    // If byobCanceled is false, perform ! ReadableByteStreamControllerRespondWithNewView(byobBranch.[[controller]], chunk).
                    if !byob_canceled {
                        let byob_objects =
                            ReadableStreamObjects::from_class_no_reader(self.byob_objects.clone())
                                .refresh_reader();

                        ReadableByteStreamController::readable_byte_stream_controller_respond_with_new_view(ctx.clone(), byob_objects, chunk)?;
                    }

                    let other_objects =
                        ReadableStreamObjects::from_class_no_reader(self.other_objects.clone())
                            .refresh_reader();

                    // If otherCanceled is false and otherBranch.[[controller]].[[pendingPullIntos]] is not empty, perform ! ReadableByteStreamControllerRespond(otherBranch.[[controller]], 0).
                    if !other_canceled && !other_objects.controller.pending_pull_intos.is_empty() {
                        ReadableByteStreamController::readable_byte_stream_controller_respond(
                            ctx.clone(),
                            other_objects,
                            0,
                        )?;
                    }
                }

                // If byobCanceled is false or otherCanceled is false, resolve cancelPromise with undefined.
                if !byob_canceled || !other_canceled {
                    self.cancel_promise
                        .resolve(Value::new_undefined(ctx.clone()))?
                }

                Ok(objects)
            }

            fn error_steps(
                &self,
                objects: ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
                _: Value<'js>,
            ) -> Result<
                ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
            > {
                // Set reading to false.
                self.reading.store(false, Ordering::Relaxed);
                Ok(objects)
            }
        }

        // Perform ! ReadableStreamBYOBReaderRead(reader, view, 1, readIntoRequest).
        Ok(ReadableStreamBYOBReader::readable_stream_byob_reader_read(
            &ctx,
            objects.set_reader(OwnedBorrowMut::from_class(current_reader)),
            view,
            1,
            ReadIntoRequest {
                reader,
                reading,
                read_again_for_branch_1,
                read_again_for_branch_2,
                reason_1,
                reason_2,
                objects_1,
                objects_2,
                byob_objects,
                other_objects,
                cancel_promise,
                for_branch_2,
            },
        )?
        .clear_reader())
    }

    // Let pull1Algorithm be the following steps:
    #[allow(clippy::too_many_arguments)]
    fn readable_byte_stream_pull_1_algorithm(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        mut objects_1: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        objects_2: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<Promise<'js>> {
        // If reading is true,
        if reading.swap(true, Ordering::Relaxed) {
            // Set readAgainForBranch1 to true.
            read_again_for_branch_1.store(true, Ordering::Relaxed);
            // Return a promise resolved with undefined.
            return promise_resolved_with(
                &ctx.clone(),
                &objects.stream.promise_primordials,
                Ok(Value::new_undefined(ctx)),
            );
        }
        // Set reading to true.

        // Let byobRequest be ! ReadableByteStreamControllerGetBYOBRequest(branch1.[[controller]]).
        let (byob_request, branch_1_controller) =
            ReadableByteStreamController::readable_byte_stream_controller_get_byob_request(
                ctx.clone(),
                objects_1.controller,
            )?;
        objects_1.controller = branch_1_controller;

        // If byobRequest is null, perform pullWithDefaultReader.
        objects = match byob_request.0 {
            None => Self::readable_byte_stream_pull_with_default_reader(
                ctx.clone(),
                objects,
                reader.clone(),
                reading.clone(),
                read_again_for_branch_1,
                read_again_for_branch_2,
                reason_1,
                reason_2,
                objects_1,
                objects_2,
                cancel_promise.clone(),
            )?,
            // Otherwise, perform pullWithBYOBReader, given byobRequest.[[view]] and false.
            Some(byob_request) => {
                let view = byob_request.borrow().view.clone().expect(
                    "ReadableByteStream tee pull1Algorithm called with invalidated byobRequest",
                );
                Self::readable_byte_stream_pull_with_byob_reader(
                    ctx.clone(),
                    objects,
                    reader.clone(),
                    reading.clone(),
                    read_again_for_branch_1,
                    read_again_for_branch_2,
                    reason_1,
                    reason_2,
                    objects_1,
                    objects_2,
                    cancel_promise.clone(),
                    view,
                    false,
                )?
            },
        };

        // Return a promise resolved with undefined.
        return promise_resolved_with(
            &ctx.clone(),
            &objects.stream.promise_primordials,
            Ok(Value::new_undefined(ctx)),
        );
    }

    // Let pull2Algorithm be the following steps:
    #[allow(clippy::too_many_arguments)]
    fn readable_byte_stream_pull_2_algorithm(
        ctx: Ctx<'js>,
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        objects_1: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        mut objects_2: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            UndefinedReader,
        >,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<Promise<'js>> {
        // If reading is true,
        if reading.swap(true, Ordering::Relaxed) {
            // Set readAgainForBranch2 to true.
            read_again_for_branch_2.store(true, Ordering::Relaxed);
            // Return a promise resolved with undefined.
            return promise_resolved_with(
                &ctx.clone(),
                &objects.stream.promise_primordials,
                Ok(Value::new_undefined(ctx)),
            );
        }
        // Set reading to true.

        // Let byobRequest be ! ReadableByteStreamControllerGetBYOBRequest(branch2.[[controller]]).
        let (byob_request, branch_2_controller) =
            ReadableByteStreamController::readable_byte_stream_controller_get_byob_request(
                ctx.clone(),
                objects_2.controller,
            )?;
        objects_2.controller = branch_2_controller;

        // If byobRequest is null, perform pullWithDefaultReader.
        objects = match byob_request.0 {
            None => Self::readable_byte_stream_pull_with_default_reader(
                ctx.clone(),
                objects,
                reader.clone(),
                reading.clone(),
                read_again_for_branch_1,
                read_again_for_branch_2,
                reason_1,
                reason_2,
                objects_1,
                objects_2,
                cancel_promise,
            )?,
            // Otherwise, perform pullWithBYOBReader, given byobRequest.[[view]] and true.
            Some(byob_request) => Self::readable_byte_stream_pull_with_byob_reader(
                ctx.clone(),
                objects,
                reader.clone(),
                reading.clone(),
                read_again_for_branch_1,
                read_again_for_branch_2,
                reason_1,
                reason_2,
                objects_1,
                objects_2,
                cancel_promise,
                byob_request.borrow().view.clone().expect(
                    "ReadableByteStream tee pull2Algorithm called with invalidated byobRequest",
                ),
                true,
            )?,
        };

        // Return a promise resolved with undefined.
        return promise_resolved_with(
            &ctx.clone(),
            &objects.stream.promise_primordials,
            Ok(Value::new_undefined(ctx)),
        );
    }
}

fn clone_as_uint8_array<'js>(
    ctx: Ctx<'js>,
    constructor_uint8array: &Constructor<'js>,
    function_array_buffer_is_view: &Function<'js>,
    chunk: ViewBytes<'js>,
) -> Result<ViewBytes<'js>> {
    let (buffer, byte_length, byte_offset) = chunk.get_array_buffer()?;

    // Let buffer be ? CloneArrayBuffer(O.[[ViewedArrayBuffer]], O.[[ByteOffset]], O.[[ByteLength]], %ArrayBuffer%).
    let buffer = ArrayBuffer::new_copy(
        ctx.clone(),
        &buffer
            .as_bytes()
            .expect("CloneAsUInt8Array called on detached buffer")
            [byte_offset..byte_offset + byte_length],
    )?;

    // Let array be ! Construct(%Uint8Array%, « buffer »).
    // Return array.
    ViewBytes::from_value(
        &ctx,
        function_array_buffer_is_view,
        &constructor_uint8array.construct((buffer,))?,
    )
}

#[derive(JsLifetime)]
struct TeePrimordials<'js> {
    structured_clone: Function<'js>,
}

impl<'js> Primordial<'js> for TeePrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            structured_clone: ctx.globals().get("structuredClone")?,
        })
    }
}
