use rquickjs::{
    class::{OwnedBorrow, OwnedBorrowMut, Trace, Tracer},
    function::Constructor,
    Ctx, Error, FromJs, Function, IntoJs, JsLifetime, Promise, Result, Value,
};

use super::{
    byob_reader::{ReadableStreamBYOBReaderClass, ReadableStreamBYOBReaderOwned},
    controller::ReadableStreamController,
    default_reader::{ReadableStreamDefaultReaderClass, ReadableStreamDefaultReaderOwned},
    objects::ReadableStreamObjects,
    ReadableStream, ReadableStreamBYOBReader, ReadableStreamClass, ReadableStreamDefaultReader,
    ReadableStreamOwned, ReadableStreamState,
};
use crate::{PromisePrimordials, ResolveablePromise};

pub(super) trait ReadableStreamReader<'js>: Sized + 'js {
    type Class: Clone + Trace<'js>;

    fn with_reader<C>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            ReadableStreamDefaultReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamDefaultReaderOwned<'js>)>,
        byob: impl FnOnce(
            C,
            ReadableStreamBYOBReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamBYOBReaderOwned<'js>)>,
        none: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)>;

    fn into_inner(self) -> Self::Class;

    fn from_class(class: Self::Class) -> Self;

    fn try_from_erased(erased: Option<ReadableStreamReaderOwned<'js>>) -> Option<Self>;
}

// typedef (ReadableStreamDefaultController or ReadableByteStreamController) ReadableStreamController;
#[derive(JsLifetime, Clone, PartialEq, Eq)]
pub(super) enum ReadableStreamReaderClass<'js> {
    ReadableStreamDefaultReader(ReadableStreamDefaultReaderClass<'js>),
    ReadableStreamBYOBReader(ReadableStreamBYOBReaderClass<'js>),
}

impl<'js> ReadableStreamReaderClass<'js> {
    pub(super) fn closed_promise(&self) -> Promise<'js> {
        match self {
            Self::ReadableStreamDefaultReader(r) => {
                r.borrow().generic.closed_promise.promise.clone()
            },
            Self::ReadableStreamBYOBReader(r) => r.borrow().generic.closed_promise.promise.clone(),
        }
    }
}

impl<'js> From<ReadableStreamDefaultReaderClass<'js>> for ReadableStreamReaderClass<'js> {
    fn from(value: ReadableStreamDefaultReaderClass<'js>) -> Self {
        Self::ReadableStreamDefaultReader(value)
    }
}

impl<'js> From<ReadableStreamBYOBReaderClass<'js>> for ReadableStreamReaderClass<'js> {
    fn from(value: ReadableStreamBYOBReaderClass<'js>) -> Self {
        Self::ReadableStreamBYOBReader(value)
    }
}

pub(super) enum ReadableStreamReaderOwned<'js> {
    ReadableStreamDefaultReader(ReadableStreamDefaultReaderOwned<'js>),
    ReadableStreamBYOBReader(ReadableStreamBYOBReaderOwned<'js>),
}

impl<'js> ReadableStreamReader<'js> for ReadableStreamReaderOwned<'js> {
    type Class = ReadableStreamReaderClass<'js>;

    fn with_reader<C>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            ReadableStreamDefaultReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamDefaultReaderOwned<'js>)>,
        byob: impl FnOnce(
            C,
            ReadableStreamBYOBReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamBYOBReaderOwned<'js>)>,
        _: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)> {
        match self {
            Self::ReadableStreamDefaultReader(r) => {
                let (ctx, r) = default(ctx, r)?;
                Ok((ctx, Self::ReadableStreamDefaultReader(r)))
            },
            Self::ReadableStreamBYOBReader(r) => {
                let (ctx, r) = byob(ctx, r)?;
                Ok((ctx, Self::ReadableStreamBYOBReader(r)))
            },
        }
    }

    fn into_inner(self) -> Self::Class {
        match self {
            ReadableStreamReaderOwned::ReadableStreamDefaultReader(r) => {
                ReadableStreamReaderClass::ReadableStreamDefaultReader(r.into_inner())
            },
            ReadableStreamReaderOwned::ReadableStreamBYOBReader(r) => {
                ReadableStreamReaderClass::ReadableStreamBYOBReader(r.into_inner())
            },
        }
    }

    fn from_class(class: Self::Class) -> Self {
        match class {
            ReadableStreamReaderClass::ReadableStreamDefaultReader(r) => {
                Self::ReadableStreamDefaultReader(OwnedBorrowMut::from_class(r))
            },
            ReadableStreamReaderClass::ReadableStreamBYOBReader(r) => {
                Self::ReadableStreamBYOBReader(OwnedBorrowMut::from_class(r))
            },
        }
    }

    fn try_from_erased(erased: Option<ReadableStreamReaderOwned<'js>>) -> Option<Self> {
        erased
    }
}

impl<'js, T: ReadableStreamReader<'js>> ReadableStreamReader<'js> for Option<T> {
    type Class = Option<<T as ReadableStreamReader<'js>>::Class>;

    fn with_reader<C>(
        self,
        mut ctx: C,
        default: impl FnOnce(
            C,
            ReadableStreamDefaultReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamDefaultReaderOwned<'js>)>,
        byob: impl FnOnce(
            C,
            ReadableStreamBYOBReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamBYOBReaderOwned<'js>)>,
        none: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)> {
        match self {
            Some(mut reader) => {
                (ctx, reader) = reader.with_reader(ctx, default, byob, none)?;
                Ok((ctx, Some(reader)))
            },
            None => Ok((none(ctx)?, None)),
        }
    }

    fn into_inner(self) -> Self::Class {
        self.map(ReadableStreamReader::into_inner)
    }

    fn from_class(class: Self::Class) -> Self {
        class.map(ReadableStreamReader::from_class)
    }

    fn try_from_erased(erased: Option<ReadableStreamReaderOwned<'js>>) -> Option<Self> {
        match erased {
            Some(r) => Some(Some(T::try_from_erased(Some(r))?)),
            None => Some(None),
        }
    }
}

#[derive(Clone, Trace)]
pub(super) struct UndefinedReader;

impl<'js> ReadableStreamReader<'js> for UndefinedReader {
    type Class = UndefinedReader;

    fn with_reader<C>(
        self,
        ctx: C,
        _: impl FnOnce(
            C,
            ReadableStreamDefaultReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamDefaultReaderOwned<'js>)>,
        _: impl FnOnce(
            C,
            ReadableStreamBYOBReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamBYOBReaderOwned<'js>)>,
        none: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)> {
        Ok((none(ctx)?, self))
    }

    fn into_inner(self) -> Self::Class {
        UndefinedReader
    }

    fn from_class(_: Self::Class) -> Self {
        UndefinedReader
    }

    fn try_from_erased(erased: Option<ReadableStreamReaderOwned<'js>>) -> Option<Self> {
        match erased {
            None => Some(UndefinedReader),
            _ => None,
        }
    }
}

impl<'js> From<ReadableStreamDefaultReaderOwned<'js>> for ReadableStreamReaderOwned<'js> {
    fn from(value: ReadableStreamDefaultReaderOwned<'js>) -> Self {
        Self::ReadableStreamDefaultReader(value)
    }
}

impl<'js> From<ReadableStreamBYOBReaderOwned<'js>> for ReadableStreamReaderOwned<'js> {
    fn from(value: ReadableStreamBYOBReaderOwned<'js>) -> Self {
        Self::ReadableStreamBYOBReader(value)
    }
}

#[derive(JsLifetime, Trace)]
pub struct ReadableStreamGenericReader<'js> {
    pub(super) closed_promise: ResolveablePromise<'js>,
    pub(super) stream: Option<ReadableStreamClass<'js>>,

    #[qjs(skip_trace)]
    pub(super) promise_primordials: PromisePrimordials<'js>,
    #[qjs(skip_trace)]
    pub(super) constructor_type_error: Constructor<'js>,
    #[qjs(skip_trace)]
    pub(super) constructor_range_error: Constructor<'js>,
    #[qjs(skip_trace)]
    pub(super) function_array_buffer_is_view: Function<'js>,
}

impl<'js> ReadableStreamGenericReader<'js> {
    pub(super) fn readable_stream_reader_generic_initialize(
        ctx: &Ctx<'js>,
        stream: OwnedBorrow<'js, ReadableStream<'js>>,
    ) -> Result<Self> {
        let closed_promise = match stream.state {
            // If stream.[[state]] is "readable",
            ReadableStreamState::Readable => {
                // Set reader.[[closedPromise]] to a new promise.
                ResolveablePromise::new(ctx)?
            },
            // Otherwise, if stream.[[state]] is "closed",
            ReadableStreamState::Closed => {
                // Set reader.[[closedPromise]] to a promise resolved with undefined.
                ResolveablePromise::resolved_with(
                    ctx,
                    &stream.promise_primordials,
                    Ok(Value::new_undefined(ctx.clone())),
                )?
            },
            // Otherwise,
            ReadableStreamState::Errored(ref stored_error) => {
                // Set reader.[[closedPromise]] to a promise rejected with stream.[[storedError]].
                let promise = ResolveablePromise::rejected_with(
                    &stream.promise_primordials,
                    stored_error.clone(),
                )?;

                // Set reader.[[closedPromise]].[[PromiseIsHandled]] to true.
                promise.set_is_handled()?;

                promise
            },
        };

        let promise_primordials = stream.promise_primordials.clone();
        let constructor_type_error = stream.constructor_type_error.clone();
        let constructor_range_error = stream.constructor_range_error.clone();
        let function_array_buffer_is_view = stream.function_array_buffer_is_view.clone();

        Ok(Self {
            // Set reader.[[stream]] to stream.
            stream: Some(stream.into_inner()),
            closed_promise,
            promise_primordials,
            constructor_type_error,
            constructor_range_error,
            function_array_buffer_is_view,
        })
    }

    pub(super) fn readable_stream_reader_generic_release(
        &mut self,

        stream: &mut ReadableStream<'js>,
        controller_release_steps: impl FnOnce(),
    ) -> Result<()> {
        // Let stream be reader.[[stream]].
        // Assert: stream is not undefined.

        // If stream.[[state]] is "readable", reject reader.[[closedPromise]] with a TypeError exception.
        if let ReadableStreamState::Readable = stream.state {
            let e: Value = stream.constructor_type_error.call((
                "Reader was released and can no longer be used to monitor the stream's closedness",
            ))?;
            self.closed_promise.reject(e)?;
        } else {
            // Otherwise, set reader.[[closedPromise]] to a promise rejected with a TypeError exception.
            let e: Value = stream.constructor_type_error.call((
                "Reader was released and can no longer be used to monitor the stream's closedness",
            ))?;
            self.closed_promise =
                ResolveablePromise::rejected_with(&stream.promise_primordials, e)?;
        }

        // Set reader.[[closedPromise]].[[PromiseIsHandled]] to true.
        self.closed_promise.set_is_handled()?;

        // Perform ! stream.[[controller]].[[ReleaseSteps]]().
        controller_release_steps();

        // Set stream.[[reader]] to undefined.
        stream.reader = None;

        // Set reader.[[stream]] to undefined.
        self.stream = None;

        Ok(())
    }

    pub(super) fn readable_stream_reader_generic_cancel<
        C: ReadableStreamController<'js>,
        R: ReadableStreamReader<'js>,
    >(
        ctx: Ctx<'js>,
        // Let stream be reader.[[stream]].
        objects: ReadableStreamObjects<'js, C, R>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, ReadableStreamObjects<'js, C, R>)> {
        // Return ! ReadableStreamCancel(stream, reason).
        ReadableStream::readable_stream_cancel(ctx, objects, reason)
    }
}

impl<'js> ReadableStreamReaderClass<'js> {
    pub(super) fn acquire_readable_stream_default_reader(
        ctx: Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
    ) -> Result<(
        ReadableStreamOwned<'js>,
        ReadableStreamDefaultReaderClass<'js>,
    )> {
        ReadableStreamDefaultReader::set_up_readable_stream_default_reader(&ctx, stream)
    }

    pub(super) fn acquire_readable_stream_byob_reader(
        ctx: Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
    ) -> Result<(ReadableStreamOwned<'js>, ReadableStreamBYOBReaderClass<'js>)> {
        ReadableStreamBYOBReader::set_up_readable_stream_byob_reader(ctx, stream)
    }
}

impl<'js> IntoJs<'js> for ReadableStreamReaderClass<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self {
            Self::ReadableStreamDefaultReader(r) => r.into_js(ctx),
            Self::ReadableStreamBYOBReader(r) => r.into_js(ctx),
        }
    }
}

impl<'js> Trace<'js> for ReadableStreamReaderClass<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        match self {
            Self::ReadableStreamDefaultReader(r) => r.trace(tracer),
            Self::ReadableStreamBYOBReader(r) => r.trace(tracer),
        }
    }
}

impl<'js> FromJs<'js> for ReadableStreamReaderClass<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        if let Ok(default) = obj.into_class() {
            return Ok(Self::ReadableStreamDefaultReader(default));
        }

        if let Ok(default) = obj.into_class() {
            return Ok(Self::ReadableStreamBYOBReader(default));
        }

        Err(Error::new_from_js(ty_name, "ReadableStreamReader"))
    }
}
