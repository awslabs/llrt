use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use rquickjs::{
    class::{OwnedBorrowMut, Trace, Tracer},
    function::Constructor,
    prelude::{List, OnceFn},
    ArrayBuffer, Class, Ctx, Error, Function, IntoJs, JsLifetime, Promise, Result, Value,
};

use crate::{
    readable::{
        byob_reader::{ReadableStreamBYOBReader, ReadableStreamReadIntoRequest, ViewBytes},
        byte_controller::{ReadableByteStreamController, ReadableByteStreamControllerOwned},
        controller::{ReadableStreamController, ReadableStreamControllerClass},
        default_controller::{
            ReadableStreamDefaultController, ReadableStreamDefaultControllerOwned,
        },
        default_reader::{
            ReadableStreamDefaultReader, ReadableStreamDefaultReaderOwned,
            ReadableStreamReadRequest,
        },
        objects::{ReadableByteStreamObjects, ReadableStreamDefaultControllerObjects},
        objects::{
            ReadableStreamBYOBObjects, ReadableStreamClassObjects,
            ReadableStreamDefaultReaderObjects, ReadableStreamObjects,
        },
        reader::{
            ReadableStreamReader, ReadableStreamReaderClass, ReadableStreamReaderOwned,
            UndefinedReader,
        },
        stream::{
            algorithms::{CancelAlgorithm, PullAlgorithm, StartAlgorithm},
            ReadableStream, ReadableStreamClass,
        },
    },
    utils::promise::{upon_promise, ResolveablePromise},
};

/// State for tee() operation, stored in a Class for GC tracing
#[rquickjs::class]
pub struct TeeState<'js> {
    pub stream: ReadableStreamClass<'js>,
    pub controller: Class<'js, ReadableStreamDefaultController<'js>>,
    pub reader: Class<'js, ReadableStreamDefaultReader<'js>>,
    pub cancel_promise: ResolveablePromise<'js>,
    pub reading: Rc<AtomicBool>,
    pub read_again: Rc<AtomicBool>,
    pub reason_1: Rc<OnceCell<Value<'js>>>,
    pub reason_2: Rc<OnceCell<Value<'js>>>,
    pub branch_1: Rc<
        OnceCell<
            ReadableStreamClassObjects<
                'js,
                ReadableStreamDefaultControllerOwned<'js>,
                UndefinedReader,
            >,
        >,
    >,
    pub branch_2: Rc<
        OnceCell<
            ReadableStreamClassObjects<
                'js,
                ReadableStreamDefaultControllerOwned<'js>,
                UndefinedReader,
            >,
        >,
    >,
}

unsafe impl<'js> JsLifetime<'js> for TeeState<'js> {
    type Changed<'to> = TeeState<'to>;
}

impl<'js> Trace<'js> for TeeState<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.stream.trace(tracer);
        self.controller.trace(tracer);
        self.reader.trace(tracer);
        self.cancel_promise.trace(tracer);
        if let Some(r) = self.reason_1.get() {
            r.trace(tracer);
        }
        if let Some(r) = self.reason_2.get() {
            r.trace(tracer);
        }
        if let Some(b) = self.branch_1.get() {
            b.trace(tracer);
        }
        if let Some(b) = self.branch_2.get() {
            b.trace(tracer);
        }
    }
}

type ReadableStreamPair<'js> = (ReadableStreamClass<'js>, ReadableStreamClass<'js>);

impl<'js> ReadableStream<'js> {
    pub(super) fn readable_stream_tee<C: ReadableStreamController<'js>>(
        ctx: Ctx<'js>,
        objects: ReadableStreamObjects<'js, C, UndefinedReader>,
    ) -> Result<ReadableStreamPair<'js>> {
        let (streams, _) = objects.with_controller(
            ctx,
            |ctx, objects| {
                let (streams, objects) = Self::readable_stream_default_tee(ctx, objects)?;
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
        mut objects: ReadableStreamDefaultControllerObjects<'js, UndefinedReader>,
    ) -> Result<(
        ReadableStreamPair<'js>,
        ReadableStreamDefaultControllerObjects<'js, ReadableStreamDefaultReaderOwned<'js>>,
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

        let objects_class: ReadableStreamClassObjects<
            'js,
            ReadableStreamDefaultControllerOwned<'js>,
            ReadableStreamDefaultReaderOwned<'js>,
        > = objects.into_inner().set_reader(reader);

        // Create TeeState to hold all JS values for GC tracing
        let tee_state = Class::instance(
            ctx.clone(),
            TeeState {
                stream: objects_class.stream.clone(),
                controller: objects_class.controller.clone(),
                reader: objects_class.reader.clone(),
                cancel_promise: cancel_promise.clone(),
                reading: reading.clone(),
                read_again: read_again.clone(),
                reason_1: reason_1.clone(),
                reason_2: reason_2.clone(),
                branch_1: branch_1.clone(),
                branch_2: branch_2.clone(),
            },
        )?;

        let pull_algorithm = PullAlgorithm::from_tee_state(tee_state.clone());
        let cancel_algorithm_1 = CancelAlgorithm::from_tee_state_1(tee_state.clone());
        let cancel_algorithm_2 = CancelAlgorithm::from_tee_state_2(tee_state.clone());

        // Set branch1 to ! CreateReadableStream(startAlgorithm, pullAlgorithm, cancel1Algorithm).
        let branch_1_objects = {
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
        let branch_2_objects = {
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
                let tee_state = tee_state.clone();
                let branch_1_objects = branch_1_objects.clone();
                let branch_2_objects = branch_2_objects.clone();
                move |_, result| match result {
                    Ok(()) => Ok(()),
                    // Upon rejection of reader.[[closedPromise]] with reason r,
                    Err(reason) => {
                        // Perform ! ReadableStreamDefaultControllerError(branch1.[[controller]], r).
                        let objects_1 =
                            ReadableStreamObjects::from_class_no_reader(branch_1_objects)
                                .refresh_reader();
                        ReadableStreamDefaultController::readable_stream_default_controller_error(
                            objects_1,
                            reason.clone(),
                        )?;

                        // Perform ! ReadableStreamDefaultControllerError(branch2.[[controller]], r).
                        let objects_2 =
                            ReadableStreamObjects::from_class_no_reader(branch_2_objects)
                                .refresh_reader();
                        ReadableStreamDefaultController::readable_stream_default_controller_error(
                            objects_2, reason,
                        )?;
                        // If canceled1 is false or canceled2 is false, resolve cancelPromise with undefined.
                        let state = tee_state.borrow();
                        if state.reason_1.get().is_none() || state.reason_2.get().is_none() {
                            state.cancel_promise.resolve_undefined()?;
                        }

                        Ok(())
                    },
                }
            },
        )?;

        Ok((
            (branch_1_objects.stream, branch_2_objects.stream),
            ReadableStreamObjects::from_class(objects_class),
        ))
    }
}

/// Pull algorithm for tee - called from PullAlgorithm::Tee
pub fn tee_pull_algorithm<'js>(
    ctx: Ctx<'js>,
    state: Class<'js, TeeState<'js>>,
) -> Result<Promise<'js>> {
    let state_ref = state.borrow();

    // If reading is true, set readAgain to true and return resolved promise
    if state_ref.reading.load(Ordering::Acquire) {
        state_ref.read_again.store(true, Ordering::Release);
        return Ok(state_ref
            .stream
            .borrow()
            .promise_primordials
            .promise_resolved_with_undefined
            .clone());
    }

    // Set reading to true
    state_ref.reading.store(true, Ordering::Release);

    let objects_class: ReadableStreamClassObjects<
        'js,
        ReadableStreamDefaultControllerOwned<'js>,
        ReadableStreamDefaultReaderOwned<'js>,
    > = ReadableStreamClassObjects {
        stream: state_ref.stream.clone(),
        controller: state_ref.controller.clone(),
        reader: state_ref.reader.clone(),
    };
    drop(state_ref);

    let mut objects = ReadableStreamObjects::from_class(objects_class.clone());

    // ReadRequest that just holds TeeState
    #[derive(Clone)]
    struct TeeReadRequest<'js>(Class<'js, TeeState<'js>>);

    impl<'js> Trace<'js> for TeeReadRequest<'js> {
        fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
            self.0.trace(tracer);
        }
    }

    impl<'js> ReadableStreamReadRequest<'js> for TeeReadRequest<'js> {
        fn chunk_steps(
            &self,
            objects: ReadableStreamDefaultReaderObjects<'js>,
            chunk: Value<'js>,
        ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
            let ctx = chunk.ctx().clone();
            let state = self.0.clone();

            objects.with_assert_default_controller(|objects| {
                let objects_class = objects.into_inner();
                let f = {
                    let ctx = ctx.clone();
                    let _objects_class = objects_class.clone();
                    move || -> Result<()> {
                        let s = state.borrow();
                        s.read_again.store(false, Ordering::Release);

                        let chunk_1 = chunk.clone();
                        let chunk_2 = chunk;

                        if s.reason_1.get().is_none() {
                            let objects_1 = ReadableStreamObjects::from_class(
                                s.branch_1.get().cloned().expect("branch1 not set"),
                            )
                            .refresh_reader();
                            ReadableStreamDefaultController::readable_stream_default_controller_enqueue(
                                ctx.clone(),
                                objects_1,
                                chunk_1,
                            )?;
                        }

                        if s.reason_2.get().is_none() {
                            let objects_2 = ReadableStreamObjects::from_class(
                                s.branch_2.get().cloned().expect("branch2 not set"),
                            )
                            .refresh_reader();
                            ReadableStreamDefaultController::readable_stream_default_controller_enqueue(
                                ctx.clone(),
                                objects_2,
                                chunk_2,
                            )?;
                        }

                        s.reading.store(false, Ordering::Release);

                        if s.read_again.load(Ordering::Acquire) {
                            drop(s);
                            tee_pull_algorithm(ctx, state)?;
                        }

                        Ok(())
                    }
                };

                let () = Function::new(ctx, OnceFn::new(f))?.defer(())?;
                Ok(ReadableStreamObjects::from_class(objects_class))
            })
        }

        fn close_steps(
            &self,
            ctx: &Ctx<'js>,
            objects: ReadableStreamDefaultReaderObjects<'js>,
        ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
            let s = self.0.borrow();
            s.reading.store(false, Ordering::Release);

            if s.reason_1.get().is_none() {
                let objects_1 = ReadableStreamObjects::from_class(
                    s.branch_1.get().cloned().expect("branch1 not set"),
                )
                .refresh_reader();
                ReadableStreamDefaultController::readable_stream_default_controller_close(
                    ctx.clone(),
                    objects_1,
                )?;
            }

            if s.reason_2.get().is_none() {
                let objects_2 = ReadableStreamObjects::from_class(
                    s.branch_2.get().cloned().expect("branch2 not set"),
                )
                .refresh_reader();
                ReadableStreamDefaultController::readable_stream_default_controller_close(
                    ctx.clone(),
                    objects_2,
                )?;
            }

            if s.reason_1.get().is_none() || s.reason_2.get().is_none() {
                s.cancel_promise.resolve_undefined()?;
            }

            Ok(objects)
        }

        fn error_steps(
            &self,
            objects: ReadableStreamDefaultReaderObjects<'js>,
            _e: Value<'js>,
        ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
            self.0.borrow().reading.store(false, Ordering::Release);
            Ok(objects)
        }
    }

    objects = ReadableStreamDefaultReader::readable_stream_default_reader_read(
        &ctx,
        objects,
        TeeReadRequest(state),
    )?;

    Ok(objects
        .stream
        .promise_primordials
        .promise_resolved_with_undefined
        .clone())
}

/// Cancel algorithm for tee - called from CancelAlgorithm::Tee1/Tee2
pub fn tee_cancel_algorithm<'js>(
    ctx: Ctx<'js>,
    state: Class<'js, TeeState<'js>>,
    reason: Value<'js>,
    branch: usize,
) -> Result<Promise<'js>> {
    let state_ref = state.borrow();
    let objects_class: ReadableStreamClassObjects<
        'js,
        ReadableStreamDefaultControllerOwned<'js>,
        ReadableStreamDefaultReaderOwned<'js>,
    > = ReadableStreamClassObjects {
        stream: state_ref.stream.clone(),
        controller: state_ref.controller.clone(),
        reader: state_ref.reader.clone(),
    };
    let objects = ReadableStreamObjects::from_class(objects_class);
    ReadableStream::tee_cancel_algorithm_impl(
        ctx,
        objects,
        [&state_ref.reason_1, &state_ref.reason_2],
        state_ref.cancel_promise.clone(),
        reason,
        branch,
    )
}

impl<'js> ReadableStream<'js> {
    // Cancel algorithm for tee - handles both branches
    pub(crate) fn tee_cancel_algorithm_impl(
        ctx: Ctx<'js>,
        objects: ReadableStreamObjects<
            'js,
            impl ReadableStreamController<'js>,
            impl ReadableStreamReader<'js>,
        >,
        reasons: [&Rc<OnceCell<Value<'js>>>; 2],
        cancel_promise: ResolveablePromise<'js>,
        reason: Value<'js>,
        branch: usize,
    ) -> Result<Promise<'js>> {
        let other = 1 - branch;

        // Set canceled[branch] to true, set reason[branch] to reason
        reasons[branch]
            .set(reason.clone())
            .expect("tee stream already has a cancel reason");

        // If other branch is also canceled
        if let Some(other_reason) = reasons[other].get().cloned() {
            // CreateArrayFromList with reasons in correct order
            let composite_reason = if branch == 0 {
                List((reason, other_reason))
            } else {
                List((other_reason, reason))
            };
            let (cancel_result, _) = ReadableStream::readable_stream_cancel(
                ctx.clone(),
                objects,
                composite_reason.into_js(&ctx)?,
            )?;
            cancel_promise.resolve(cancel_result)?;
        }

        Ok(cancel_promise.promise)
    }

    fn readable_byte_stream_tee(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, UndefinedReader>,
    ) -> Result<(
        ReadableStreamPair<'js>,
        ReadableByteStreamObjects<'js, UndefinedReader>,
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
                        ReadableStreamObjects::new_byte(OwnedBorrowMut::from_class(branch_1.get().cloned().expect("ReadableByteStream tee pull1 algorithm called without branch1 being initialised")), branch_1_controller) ,
                        ReadableStreamObjects::new_byte(branch_2, branch_2_controller),
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
                    ReadableStreamObjects::new_byte(branch_1, branch_1_controller),
                    ReadableStreamObjects::new_byte(branch_2, branch_2_controller),
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
                Self::tee_cancel_algorithm_impl(
                    reason.ctx().clone(),
                    objects,
                    [&reason_1, &reason_2],
                    cancel_promise,
                    reason,
                    0,
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
                Self::tee_cancel_algorithm_impl(
                    reason.ctx().clone(),
                    objects,
                    [&reason_1, &reason_2],
                    cancel_promise,
                    reason,
                    1,
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
            move |_, result| match result {
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
                        cancel_promise.resolve_undefined()?;
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
        mut objects: ReadableByteStreamObjects<'js, UndefinedReader>,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        objects_1: ReadableByteStreamObjects<'js, UndefinedReader>,
        objects_2: ReadableByteStreamObjects<'js, UndefinedReader>,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<ReadableByteStreamObjects<'js, UndefinedReader>> {
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
                objects: ReadableStreamDefaultReaderObjects<'js>,
                chunk: Value<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                let ctx = chunk.ctx().clone();
                let this = self.clone();

                objects.with_assert_byte_controller(|objects| {
                    let constructor_uint8array = objects.controller.array_constructor_primordials.constructor_uint8array.clone();
                    let function_array_buffer_is_view = objects.controller.function_array_buffer_is_view.clone();
                    let chunk = ViewBytes::from_value(&ctx, &function_array_buffer_is_view, Some(&chunk))?;
                    let objects_class = objects.into_inner();
                    // Queue a microtask to perform the following steps:
                    let f = {
                        let ctx = ctx.clone();
                        let objects_class = objects_class.clone();
                        move || -> Result<()> {
                            // Set readAgainForBranch1 to false.
                            this.read_again_for_branch_1.store(false, Ordering::Release);
                            // Set readAgainForBranch2 to false.
                            this.read_again_for_branch_2.store(false, Ordering::Release);

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
                            this.reading.store(false, Ordering::Release);

                            let objects_1 = ReadableStreamObjects::from_class(this.objects_class_1);
                            let objects_2 = ReadableStreamObjects::from_class(this.objects_class_2);

                            let objects = ReadableStreamObjects::from_class_no_reader(objects_class);

                            // If readAgainForBranch1 is true, perform pull1Algorithm.
                            if this.read_again_for_branch_1.load(Ordering::Acquire) {
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
                            } else if this.read_again_for_branch_2.load(Ordering::Acquire) {
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
                objects: ReadableStreamDefaultReaderObjects<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                // Set reading to false.
                self.reading.store(false, Ordering::Release);

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
                    self.cancel_promise.resolve_undefined()?
                }
                Ok(objects)
            }

            fn error_steps(
                &self,
                objects: ReadableStreamDefaultReaderObjects<'js>,
                _: Value<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                // Set reading to false.
                self.reading.store(false, Ordering::Release);
                Ok(objects)
            }
        }

        // Perform ! ReadableStreamDefaultReaderRead(reader, readRequest).
        Ok(
            ReadableStreamDefaultReader::readable_stream_default_reader_read(
                &ctx,
                objects.set_reader(OwnedBorrowMut::from_class(current_reader)),
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
        mut objects: ReadableByteStreamObjects<'js, UndefinedReader>,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        objects_1: ReadableByteStreamObjects<'js, UndefinedReader>,
        objects_2: ReadableByteStreamObjects<'js, UndefinedReader>,
        cancel_promise: ResolveablePromise<'js>,
        view: ViewBytes<'js>,
        for_branch_2: bool,
    ) -> Result<ReadableByteStreamObjects<'js, UndefinedReader>> {
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
                objects: ReadableStreamBYOBObjects<'js>,
                chunk: Value<'js>,
            ) -> Result<ReadableStreamBYOBObjects<'js>> {
                let ctx = chunk.ctx().clone();

                let constructor_uint8array = objects
                    .controller
                    .array_constructor_primordials
                    .constructor_uint8array
                    .clone();
                let function_array_buffer_is_view =
                    objects.controller.function_array_buffer_is_view.clone();
                let chunk =
                    ViewBytes::from_value(&ctx, &function_array_buffer_is_view, Some(&chunk))?;

                let objects_class = objects.into_inner();

                // Queue a microtask to perform the following steps:
                let f = {
                    let ctx = ctx.clone();
                    let objects_class = objects_class.clone();
                    let this = self.clone();
                    move || -> Result<()> {
                        // Set readAgainForBranch1 to false.
                        this.read_again_for_branch_1.store(false, Ordering::Release);
                        // Set readAgainForBranch2 to false.
                        this.read_again_for_branch_2.store(false, Ordering::Release);

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
                        this.reading.store(false, Ordering::Release);

                        // If readAgainForBranch1 is true, perform pull1Algorithm.
                        if this.read_again_for_branch_1.load(Ordering::Acquire) {
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
                        } else if this.read_again_for_branch_2.load(Ordering::Acquire) {
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
                objects: ReadableStreamBYOBObjects<'js>,
                chunk: Value<'js>,
            ) -> Result<ReadableStreamBYOBObjects<'js>> {
                let ctx = chunk.ctx().clone();

                // Set reading to false.
                self.reading.store(false, Ordering::Release);

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
                        Some(&chunk),
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
                    self.cancel_promise.resolve_undefined()?
                }

                Ok(objects)
            }

            fn error_steps(
                &self,
                objects: ReadableStreamBYOBObjects<'js>,
                _: Value<'js>,
            ) -> Result<ReadableStreamBYOBObjects<'js>> {
                // Set reading to false.
                self.reading.store(false, Ordering::Release);
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
        mut objects: ReadableByteStreamObjects<'js, UndefinedReader>,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        mut objects_1: ReadableByteStreamObjects<'js, UndefinedReader>,
        objects_2: ReadableByteStreamObjects<'js, UndefinedReader>,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<Promise<'js>> {
        // If reading is true,
        if reading.swap(true, Ordering::AcqRel) {
            // Set readAgainForBranch1 to true.
            read_again_for_branch_1.store(true, Ordering::Release);
            // Return a promise resolved with undefined.
            return Ok(objects
                .stream
                .promise_primordials
                .promise_resolved_with_undefined
                .clone());
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
        Ok(objects
            .stream
            .promise_primordials
            .promise_resolved_with_undefined
            .clone())
    }

    // Let pull2Algorithm be the following steps:
    #[allow(clippy::too_many_arguments)]
    fn readable_byte_stream_pull_2_algorithm(
        ctx: Ctx<'js>,
        mut objects: ReadableByteStreamObjects<'js, UndefinedReader>,
        reader: Rc<RefCell<ReadableStreamReaderClass<'js>>>,
        reading: Rc<AtomicBool>,
        read_again_for_branch_1: Rc<AtomicBool>,
        read_again_for_branch_2: Rc<AtomicBool>,
        reason_1: Rc<OnceCell<Value<'js>>>,
        reason_2: Rc<OnceCell<Value<'js>>>,
        objects_1: ReadableByteStreamObjects<'js, UndefinedReader>,
        mut objects_2: ReadableByteStreamObjects<'js, UndefinedReader>,
        cancel_promise: ResolveablePromise<'js>,
    ) -> Result<Promise<'js>> {
        // If reading is true,
        if reading.swap(true, Ordering::AcqRel) {
            // Set readAgainForBranch2 to true.
            read_again_for_branch_2.store(true, Ordering::Release);
            // Return a promise resolved with undefined.
            return Ok(objects
                .stream
                .promise_primordials
                .promise_resolved_with_undefined
                .clone());
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
        Ok(objects
            .stream
            .promise_primordials
            .promise_resolved_with_undefined
            .clone())
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
        Some(&constructor_uint8array.construct((buffer,))?),
    )
}
