use std::collections::VecDeque;

use rquickjs::class::Tracer;
use rquickjs::prelude::This;
use rquickjs::{
    class::{OwnedBorrowMut, Trace},
    methods,
    prelude::Opt,
    Class, Ctx, Exception, Promise, Result, Value,
};
use rquickjs::{IntoJs, JsLifetime, Object};

use crate::{
    readable::{
        byob_reader::ReadableStreamBYOBReaderOwned,
        controller::ReadableStreamController,
        objects::{ReadableStreamDefaultReaderObjects, ReadableStreamObjects},
        reader::{
            ReadableStreamGenericReader, ReadableStreamReader, ReadableStreamReaderOwned,
            UndefinedReader,
        },
        stream::{ReadableStreamOwned, ReadableStreamState},
    },
    utils::{
        promise::{promise_rejected_with_constructor, ResolveablePromise},
        UnwrapOrUndefined,
    },
};

#[derive(Trace)]
#[rquickjs::class]
pub(crate) struct ReadableStreamDefaultReader<'js> {
    pub(super) generic: ReadableStreamGenericReader<'js>,
    pub(super) read_requests: VecDeque<Box<dyn ReadableStreamReadRequest<'js> + 'js>>,
}

pub(crate) type ReadableStreamDefaultReaderClass<'js> =
    Class<'js, ReadableStreamDefaultReader<'js>>;
pub(crate) type ReadableStreamDefaultReaderOwned<'js> =
    OwnedBorrowMut<'js, ReadableStreamDefaultReader<'js>>;

unsafe impl<'js> JsLifetime<'js> for ReadableStreamDefaultReader<'js> {
    type Changed<'to> = ReadableStreamDefaultReader<'to>;
}

impl<'js> ReadableStreamDefaultReader<'js> {
    pub(super) fn readable_stream_default_reader_error_read_requests<
        C: ReadableStreamController<'js>,
    >(
        mut objects: ReadableStreamDefaultReaderObjects<'js, C>,
        e: Value<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>> {
        // Let readRequests be reader.[[readRequests]].
        let read_requests = &mut objects.reader.read_requests;

        // Set reader.[[readRequests]] to a new empty list.
        let read_requests = read_requests.split_off(0);

        // For each readRequest of readRequests,
        for read_request in read_requests {
            // Perform readRequest’s error steps, given e.
            objects = read_request.error_steps_typed(objects, e.clone())?;
        }

        Ok(objects)
    }

    pub(super) fn readable_stream_default_reader_read<
        'closure,
        C: ReadableStreamController<'js>,
    >(
        ctx: &Ctx<'js>,
        // Let stream be reader.[[stream]].
        mut objects: ReadableStreamDefaultReaderObjects<'js, C>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>> {
        // Set stream.[[disturbed]] to true.
        objects.stream.disturbed = true;
        match objects.stream.state {
            // If stream.[[state]] is "closed", perform readRequest’s close steps.
            ReadableStreamState::Closed => read_request.close_steps_typed(ctx, objects),
            // Otherwise, if stream.[[state]] is "errored", perform readRequest’s error steps given stream.[[storedError]].
            ReadableStreamState::Errored(ref stored_error) => {
                let stored_error = stored_error.clone();
                read_request.error_steps_typed(objects, stored_error)
            },
            // Otherwise,
            _ => {
                // Perform ! stream.[[controller]].[[PullSteps]](readRequest).
                C::pull_steps(ctx, objects, read_request)
            },
        }
    }

    pub(super) fn set_up_readable_stream_default_reader(
        ctx: &Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
    ) -> Result<(ReadableStreamOwned<'js>, Class<'js, Self>)> {
        // If ! IsReadableStreamLocked(stream) is true, throw a TypeError exception.
        if stream.is_readable_stream_locked() {
            return Err(Exception::throw_type(
                ctx,
                "This stream has already been locked for exclusive reading by another reader",
            ));
        }

        // Perform ! ReadableStreamReaderGenericInitialize(reader, stream).
        let generic =
            ReadableStreamGenericReader::readable_stream_reader_generic_initialize(ctx, stream)?;
        let mut stream = OwnedBorrowMut::from_class(generic.stream.clone().unwrap());

        let reader = Class::instance(
            ctx.clone(),
            Self {
                generic,
                // Set reader.[[readRequests]] to a new empty list.
                read_requests: VecDeque::new(),
            },
        )?;

        stream.reader = Some(reader.clone().into());

        Ok((stream, reader))
    }

    pub(super) fn readable_stream_default_reader_release<C: ReadableStreamController<'js>>(
        mut objects: ReadableStreamDefaultReaderObjects<'js, C>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>> {
        // Perform ! ReadableStreamReaderGenericRelease(reader).
        objects
            .reader
            .generic
            .readable_stream_reader_generic_release(&mut objects.stream, || {
                objects.controller.release_steps()
            })?;

        // Let e be a new TypeError exception.
        let e: Value = objects
            .stream
            .constructor_type_error
            .call(("Reader was released",))?;

        // Perform ! ReadableStreamDefaultReaderErrorReadRequests(reader, e).
        Self::readable_stream_default_reader_error_read_requests(objects, e)
    }
}

#[methods(rename_all = "camelCase")]
impl<'js> ReadableStreamDefaultReader<'js> {
    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, stream: ReadableStreamOwned<'js>) -> Result<Class<'js, Self>> {
        // Perform ? SetUpReadableStreamDefaultReader(this, stream).
        let (_, reader) = Self::set_up_readable_stream_default_reader(&ctx, stream)?;
        Ok(reader)
    }

    fn read(ctx: Ctx<'js>, reader: This<OwnedBorrowMut<'js, Self>>) -> Result<Promise<'js>> {
        if reader.generic.stream.is_none() {
            // If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
            return promise_rejected_with_constructor(
                &reader.generic.constructor_type_error,
                &reader.generic.promise_primordials,
                "Cannot read from a stream using a released reader",
            );
        }

        let objects = ReadableStreamObjects::from_default_reader(reader.0);

        // Let promise be a new promise.
        let promise = ResolveablePromise::new(&ctx)?;

        // Let readRequest be a new read request with the following items:
        #[derive(Trace)]
        struct ReadRequest<'js> {
            promise: ResolveablePromise<'js>,
        }

        impl<'js> ReadableStreamReadRequest<'js> for ReadRequest<'js> {
            // chunk steps, given chunk
            // Resolve promise with «[ "value" → chunk, "done" → false ]».
            fn chunk_steps(
                &self,
                objects: ReadableStreamDefaultReaderObjects<'js>,
                chunk: Value<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                self.promise.resolve(ReadableStreamReadResult {
                    value: Some(chunk),
                    done: false,
                })?;

                Ok(objects)
            }

            // close steps
            // Resolve promise with «[ "value" → undefined, "done" → true ]».
            fn close_steps(
                &self,
                _: &Ctx<'js>,
                objects: ReadableStreamDefaultReaderObjects<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                self.promise.resolve(ReadableStreamReadResult {
                    value: None,
                    done: true,
                })?;
                Ok(objects)
            }

            fn error_steps(
                &self,
                objects: ReadableStreamDefaultReaderObjects<'js>,
                e: Value<'js>,
            ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
                self.promise.reject(e)?;
                Ok(objects)
            }
        }

        // Perform ! ReadableStreamDefaultReaderRead(this, readRequest).
        Self::readable_stream_default_reader_read(
            &ctx,
            objects,
            ReadRequest {
                promise: promise.clone(),
            },
        )?;

        // Return promise.
        Ok(promise.promise)
    }

    fn release_lock(reader: This<OwnedBorrowMut<'js, Self>>) -> Result<()> {
        if reader.generic.stream.is_none() {
            // If this.[[stream]] is undefined, return.
            return Ok(());
        }

        let objects = ReadableStreamObjects::from_default_reader(reader.0);

        // Perform ! ReadableStreamDefaultReaderRelease(this).
        Self::readable_stream_default_reader_release(objects)?;
        Ok(())
    }

    #[qjs(get)]
    fn closed(&self) -> Promise<'js> {
        self.generic.closed_promise.promise.clone()
    }

    fn cancel(
        ctx: Ctx<'js>,
        reader: This<OwnedBorrowMut<'js, Self>>,
        reason: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        if reader.generic.stream.is_none() {
            // If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
            return promise_rejected_with_constructor(
                &reader.generic.constructor_type_error,
                &reader.generic.promise_primordials,
                "Cannot cancel a stream using a released reader",
            );
        };

        let objects = ReadableStreamObjects::from_default_reader(reader.0);

        // Return ! ReadableStreamReaderGenericCancel(this, reason).
        let (promise, _) = ReadableStreamGenericReader::readable_stream_reader_generic_cancel(
            ctx.clone(),
            objects,
            reason.0.unwrap_or_undefined(&ctx),
        )?;
        Ok(promise)
    }
}

impl<'js> ReadableStreamReader<'js> for ReadableStreamDefaultReaderOwned<'js> {
    type Class = ReadableStreamDefaultReaderClass<'js>;

    fn with_reader<C>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            ReadableStreamDefaultReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamDefaultReaderOwned<'js>)>,
        _: impl FnOnce(
            C,
            ReadableStreamBYOBReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamBYOBReaderOwned<'js>)>,
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

    fn try_from_erased(erased: Option<ReadableStreamReaderOwned<'js>>) -> Option<Self> {
        match erased {
            Some(ReadableStreamReaderOwned::ReadableStreamDefaultReader(r)) => Some(r),
            _ => None,
        }
    }
}

pub(super) trait ReadableStreamDefaultReaderOrUndefined<'js>:
    ReadableStreamReader<'js>
{
}

impl<'js> ReadableStreamDefaultReaderOrUndefined<'js> for ReadableStreamDefaultReaderOwned<'js> {}

impl<'js> ReadableStreamDefaultReaderOrUndefined<'js>
    for Option<ReadableStreamDefaultReaderOwned<'js>>
{
}

impl ReadableStreamDefaultReaderOrUndefined<'_> for UndefinedReader {}

pub(super) trait ReadableStreamReadRequest<'js>: Trace<'js> {
    fn chunk_steps_typed<C: ReadableStreamController<'js>>(
        &self,
        objects: ReadableStreamDefaultReaderObjects<'js, C>,
        chunk: Value<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>>
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
        objects: ReadableStreamDefaultReaderObjects<'js>,
        chunk: Value<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js>>;

    fn close_steps_typed<C: ReadableStreamController<'js>>(
        &self,
        ctx: &Ctx<'js>,
        objects: ReadableStreamDefaultReaderObjects<'js, C>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>>
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
        objects: ReadableStreamDefaultReaderObjects<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js>>;

    fn error_steps_typed<C: ReadableStreamController<'js>>(
        &self,
        objects: ReadableStreamDefaultReaderObjects<'js, C>,
        reason: Value<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>>
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
        objects: ReadableStreamDefaultReaderObjects<'js>,
        reason: Value<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js>>;
}

impl<'js> Trace<'js> for Box<dyn ReadableStreamReadRequest<'js> + 'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.as_ref().trace(tracer);
    }
}

impl<'js> ReadableStreamReadRequest<'js> for Box<dyn ReadableStreamReadRequest<'js> + 'js> {
    fn chunk_steps(
        &self,
        objects: ReadableStreamDefaultReaderObjects<'js>,
        chunk: Value<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
        self.as_ref().chunk_steps(objects, chunk)
    }

    fn close_steps(
        &self,
        ctx: &Ctx<'js>,
        objects: ReadableStreamDefaultReaderObjects<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
        self.as_ref().close_steps(ctx, objects)
    }

    fn error_steps(
        &self,
        objects: ReadableStreamDefaultReaderObjects<'js>,
        reason: Value<'js>,
    ) -> Result<ReadableStreamDefaultReaderObjects<'js>> {
        self.as_ref().error_steps(objects, reason)
    }
}

pub(super) struct ReadableStreamReadResult<'js> {
    pub(super) value: Option<Value<'js>>,
    pub(super) done: bool,
}

impl<'js> IntoJs<'js> for ReadableStreamReadResult<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        let obj = Object::new(ctx.clone())?;
        obj.set("value", self.value)?;
        obj.set("done", self.done)?;
        Ok(obj.into_value())
    }
}
