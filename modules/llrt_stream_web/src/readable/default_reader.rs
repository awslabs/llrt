use std::collections::VecDeque;

use rquickjs::prelude::This;
use rquickjs::JsLifetime;
use rquickjs::{
    class::{OwnedBorrowMut, Trace},
    methods,
    prelude::Opt,
    Class, Ctx, Exception, Promise, Result, Value,
};

use super::controller::{ReadableStreamController, ReadableStreamControllerOwned};
use super::reader::{ReadableStreamGenericReader, ReadableStreamReaderOwned, UndefinedReader};
use super::{
    byob_reader::ReadableStreamBYOBReaderOwned, promise_rejected_with, ReadableStreamObjects,
    ReadableStreamOwned, ReadableStreamReadRequest, ReadableStreamReadResult, ReadableStreamReader,
    ReadableStreamState,
};
use crate::{downgrade_owned_borrow_mut, ResolveablePromise};

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
        mut objects: ReadableStreamObjects<'js, C, OwnedBorrowMut<'js, Self>>,
        e: Value<'js>,
    ) -> Result<ReadableStreamObjects<'js, C, OwnedBorrowMut<'js, Self>>> {
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
        mut objects: ReadableStreamObjects<'js, C, OwnedBorrowMut<'js, Self>>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) -> Result<()> {
        // Set stream.[[disturbed]] to true.
        objects.stream.disturbed = true;
        match objects.stream.state {
            // If stream.[[state]] is "closed", perform readRequest’s close steps.
            ReadableStreamState::Closed => {
                read_request.close_steps_typed(ctx, objects)?;
            },
            // Otherwise, if stream.[[state]] is "errored", perform readRequest’s error steps given stream.[[storedError]].
            ReadableStreamState::Errored => {
                let stored_error = objects
                    .stream
                    .stored_error
                    .clone()
                    .expect("stream in error state without stored error");
                read_request.error_steps_typed(objects, stored_error)?;
            },
            // Otherwise,
            _ => {
                // Perform ! stream.[[controller]].[[PullSteps]](readRequest).
                C::pull_steps(ctx, objects, read_request)?;
            },
        }

        Ok(())
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
        let generic = ReadableStreamGenericReader::readable_stream_reader_generic_initialize(
            ctx,
            downgrade_owned_borrow_mut(stream),
        )?;
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
        ctx: &Ctx<'js>,
        mut objects: ReadableStreamObjects<'js, C, OwnedBorrowMut<'js, Self>>,
    ) -> Result<ReadableStreamObjects<'js, C, OwnedBorrowMut<'js, Self>>> {
        // Perform ! ReadableStreamReaderGenericRelease(reader).
        objects
            .reader
            .generic
            .readable_stream_reader_generic_release(ctx, &mut objects.stream, || {
                objects.controller.release_steps()
            })?;

        // Let e be a new TypeError exception.
        let e: Value = ctx.eval(r#"new TypeError("Reader was released")"#)?;

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
        let stream = if let Some(ref stream) = reader.generic.stream {
            OwnedBorrowMut::from_class(stream.clone())
        } else {
            // If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
            let e: Value =
                ctx.eval(r#"new TypeError("Cannot read from a stream using a released reader")"#)?;
            return promise_rejected_with(&ctx, e);
        };

        let controller = ReadableStreamControllerOwned::from_class(stream.controller.clone());

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
                self.promise.resolve(ReadableStreamReadResult {
                    value: None,
                    done: true,
                })?;
                Ok(objects)
            }

            fn error_steps(
                &self,
                objects: ReadableStreamObjects<
                    'js,
                    ReadableStreamControllerOwned<'js>,
                    ReadableStreamDefaultReaderOwned<'js>,
                >,
                e: Value<'js>,
            ) -> Result<
                ReadableStreamObjects<
                    'js,
                    ReadableStreamControllerOwned<'js>,
                    ReadableStreamDefaultReaderOwned<'js>,
                >,
            > {
                self.promise.reject(e)?;
                Ok(objects)
            }
        }

        let objects = ReadableStreamObjects {
            stream,
            controller,
            reader: reader.0,
        };

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

    fn release_lock(ctx: Ctx<'js>, reader: This<OwnedBorrowMut<'js, Self>>) -> Result<()> {
        let stream = match reader.generic.stream.clone() {
            // If this.[[stream]] is undefined, return.
            None => {
                return Ok(());
            },
            Some(stream) => OwnedBorrowMut::from_class(stream),
        };

        let controller = ReadableStreamControllerOwned::from_class(stream.controller.clone());

        let objects = ReadableStreamObjects {
            stream,
            controller,
            reader: reader.0,
        };

        // Perform ! ReadableStreamDefaultReaderRelease(this).
        Self::readable_stream_default_reader_release(&ctx, objects)?;
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
        let stream = match reader.generic.stream.clone() {
            // If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
            None => {
                let e: Value =
                    ctx.eval(r#"new TypeError("Cannot cancel a stream using a released reader")"#)?;
                return promise_rejected_with(&ctx, e);
            },
            Some(stream) => OwnedBorrowMut::from_class(stream),
        };

        let controller = ReadableStreamControllerOwned::from_class(stream.controller.clone());

        let objects = ReadableStreamObjects {
            stream,
            controller,
            reader: reader.0,
        };

        // Return ! ReadableStreamReaderGenericCancel(this, reason).
        let (promise, _) = ReadableStreamGenericReader::readable_stream_reader_generic_cancel(
            ctx.clone(),
            objects,
            reason.0.unwrap_or(Value::new_undefined(ctx)),
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
