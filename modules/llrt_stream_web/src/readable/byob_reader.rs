use std::collections::VecDeque;

use llrt_utils::bytes::ObjectBytes;
use rquickjs::{
    atom::PredefinedAtom,
    class::{JsClass, OwnedBorrowMut, Trace, Tracer},
    function::Constructor,
    methods,
    prelude::{Opt, This},
    ArrayBuffer, Class, Ctx, Error, Exception, FromJs, Function, IntoJs, JsLifetime, Object,
    Promise, Result, Value,
};

use super::{
    byte_controller::ReadableByteStreamController,
    controller::{ReadableStreamController, ReadableStreamControllerClass},
    objects::ReadableStreamObjects,
    promise_rejected_with,
    reader::{ReadableStreamGenericReader, ReadableStreamReader, ReadableStreamReaderOwned},
    ObjectExt, ReadableStreamOwned, ReadableStreamReadResult, ReadableStreamState,
};
use crate::readable::byte_controller::ReadableByteStreamControllerOwned;
use crate::readable::default_reader::ReadableStreamDefaultReaderOwned;
use crate::{downgrade_owned_borrow_mut, ResolveablePromise};

#[derive(Trace)]
#[rquickjs::class]
pub(crate) struct ReadableStreamBYOBReader<'js> {
    pub(super) generic: ReadableStreamGenericReader<'js>,
    pub(super) read_into_requests: VecDeque<Box<dyn ReadableStreamReadIntoRequest<'js> + 'js>>,
}

pub(crate) type ReadableStreamBYOBReaderClass<'js> = Class<'js, ReadableStreamBYOBReader<'js>>;
pub(crate) type ReadableStreamBYOBReaderOwned<'js> =
    OwnedBorrowMut<'js, ReadableStreamBYOBReader<'js>>;

unsafe impl<'js> JsLifetime<'js> for ReadableStreamBYOBReader<'js> {
    type Changed<'to> = ReadableStreamBYOBReader<'to>;
}

impl<'js> ReadableStreamBYOBReader<'js> {
    pub(super) fn readable_stream_byob_reader_error_read_into_requests(
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
        e: Value<'js>,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
    > {
        // Let readIntoRequests be reader.[[readIntoRequests]].
        let read_into_requests = &mut objects.reader.read_into_requests;

        // Set reader.[[readIntoRequests]] to a new empty list.
        let read_into_requests = read_into_requests.split_off(0);
        // For each readIntoRequest of readIntoRequests,
        for read_into_request in read_into_requests {
            // Perform readIntoRequest’s error steps, given e.
            objects = read_into_request.error_steps(objects, e.clone())?;
        }

        Ok(objects)
    }

    pub(super) fn set_up_readable_stream_byob_reader(
        ctx: Ctx<'js>,
        stream: ReadableStreamOwned<'js>,
    ) -> Result<(ReadableStreamOwned<'js>, Class<'js, Self>)> {
        // If ! IsReadableStreamLocked(stream) is true, throw a TypeError exception.
        if stream.is_readable_stream_locked() {
            return Err(Exception::throw_type(
                &ctx,
                "This stream has already been locked for exclusive reading by another reader",
            ));
        }

        // If stream.[[controller]] does not implement ReadableByteStreamController, throw a TypeError exception.
        match stream.controller {
            ReadableStreamControllerClass::ReadableStreamByteController(_) => {},
            _ => {
                return Err(Exception::throw_type(
                    &ctx,
                    "Cannot construct a ReadableStreamBYOBReader for a stream not constructed with a byte source",
                ));
            },
        };

        // Perform ! ReadableStreamReaderGenericInitialize(reader, stream).
        let generic = ReadableStreamGenericReader::readable_stream_reader_generic_initialize(
            &ctx,
            downgrade_owned_borrow_mut(stream),
        )?;

        let mut stream = OwnedBorrowMut::from_class(generic.stream.clone().unwrap());

        let reader = Class::instance(
            ctx.clone(),
            Self {
                generic,
                // Set reader.[[readIntoRequests]] to a new empty list.
                read_into_requests: VecDeque::new(),
            },
        )?;

        stream.reader = Some(reader.clone().into());

        Ok((stream, reader))
    }

    pub(super) fn readable_stream_byob_reader_release(
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
    > {
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
        // Perform ! ReadableStreamBYOBReaderErrorReadIntoRequests(reader, e).
        Self::readable_stream_byob_reader_error_read_into_requests(objects, e)
    }

    pub(super) fn readable_stream_byob_reader_read(
        ctx: &Ctx<'js>,
        // Let stream be reader.[[stream]].
        mut objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
        view: ViewBytes<'js>,
        min: u64,
        read_into_request: impl ReadableStreamReadIntoRequest<'js> + 'js,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
    > {
        // Set stream.[[disturbed]] to true.
        objects.stream.disturbed = true;

        // If stream.[[state]] is "errored", perform readIntoRequest’s error steps given stream.[[storedError]].
        if let ReadableStreamState::Errored(ref stored_error) = objects.stream.state {
            let stored_error = stored_error.clone();
            read_into_request.error_steps(objects, stored_error)
        } else {
            // Otherwise, perform ! ReadableByteStreamControllerPullInto(stream.[[controller]], view, min, readIntoRequest).
            ReadableByteStreamController::readable_byte_stream_controller_pull_into(
                ctx,
                objects,
                view,
                min,
                read_into_request,
            )
        }
    }
}

#[methods(rename_all = "camelCase")]
impl<'js> ReadableStreamBYOBReader<'js> {
    // this is required by web platform tests
    #[qjs(get)]
    pub fn constructor(ctx: Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        <ReadableStreamBYOBReader as JsClass>::constructor(&ctx)
    }

    #[qjs(constructor)]
    pub fn new(ctx: Ctx<'js>, stream: ReadableStreamOwned<'js>) -> Result<Class<'js, Self>> {
        // Perform ? SetUpReadableStreamBYOBReader(this, stream).
        let (_, reader) = Self::set_up_readable_stream_byob_reader(ctx, stream)?;
        Ok(reader)
    }

    fn read(
        ctx: Ctx<'js>,
        reader: This<OwnedBorrowMut<'js, Self>>,
        view: Opt<Value<'js>>,
        options: Opt<Value<'js>>,
    ) -> Result<Promise<'js>> {
        let options = match options.0 {
            None => ReadableStreamBYOBReaderReadOptions { min: 1 },
            Some(value) => match ReadableStreamBYOBReaderReadOptions::from_js(&ctx, value) {
                Ok(value) => value,
                Err(Error::Exception) => {
                    return promise_rejected_with(&reader.generic.promise_primordials, ctx.catch());
                },
                Err(err) => return Err(err),
            },
        };

        let view = view.0.unwrap_or_else(|| Value::new_undefined(ctx.clone()));
        let view =
            match ViewBytes::from_value(&ctx, &reader.generic.function_array_buffer_is_view, &view)
            {
                Ok(view) => view,
                Err(Error::Exception) => {
                    return promise_rejected_with(&reader.generic.promise_primordials, ctx.catch());
                },
                Err(err) => return Err(err),
            };

        let (buffer, byte_length) = match view.get_array_buffer() {
            Ok((buffer, byte_length, _)) => (buffer, byte_length),
            // this can happen if its detached
            Err(Error::Exception) => {
                return promise_rejected_with(&reader.generic.promise_primordials, ctx.catch())
            },
            Err(err) => return Err(err),
        };

        // If view.[[ByteLength]] is 0, return a promise rejected with a TypeError exception.
        if byte_length == 0 {
            let e: Value = reader
                .generic
                .constructor_type_error
                .call(("view must have non-zero byteLength",))?;
            return promise_rejected_with(&reader.generic.promise_primordials, e);
        }

        // If view.[[ViewedArrayBuffer]].[[ArrayBufferByteLength]] is 0, return a promise rejected with a TypeError exception.
        if buffer.is_empty() {
            let e: Value = reader
                .generic
                .constructor_type_error
                .call(("view's buffer must have non-zero byteLength",))?;
            return promise_rejected_with(&reader.generic.promise_primordials, e);
        }

        // If ! IsDetachedBuffer(view.[[ViewedArrayBuffer]]) is true, return a promise rejected with a TypeError exception.
        if buffer.as_bytes().is_none() {
            let e: Value = reader
                .generic
                .constructor_type_error
                .call(("view's buffer has been detached",))?;
            return promise_rejected_with(&reader.generic.promise_primordials, e);
        }

        // If options["min"] is 0, return a promise rejected with a TypeError exception.
        if options.min == 0 {
            let e: Value = reader
                .generic
                .constructor_type_error
                .call(("options.min must be greater than 0",))?;
            return promise_rejected_with(&reader.generic.promise_primordials, e);
        }

        // If view has a [[TypedArrayName]] internal slot,
        let typed_array_len = match &view.0 {
            ObjectBytes::U8Array(a) => Some(a.len()),
            ObjectBytes::I8Array(a) => Some(a.len()),
            ObjectBytes::U16Array(a) => Some(a.len()),
            ObjectBytes::I16Array(a) => Some(a.len()),
            ObjectBytes::U32Array(a) => Some(a.len()),
            ObjectBytes::I32Array(a) => Some(a.len()),
            ObjectBytes::U64Array(a) => Some(a.len()),
            ObjectBytes::I64Array(a) => Some(a.len()),
            ObjectBytes::F32Array(a) => Some(a.len()),
            ObjectBytes::F64Array(a) => Some(a.len()),
            _ => None,
        };
        if let Some(typed_array_len) = typed_array_len {
            // If options["min"] > view.[[ArrayLength]], return a promise rejected with a RangeError exception.
            if options.min > typed_array_len as u64 {
                let e: Value = reader
                    .generic
                    .constructor_range_error
                    .call(("options.min must be less than or equal to views length",))?;
                return promise_rejected_with(&reader.generic.promise_primordials, e);
            }
        } else {
            // Otherwise (i.e., it is a DataView),
            // If options["min"] > view.[[ByteLength]], return a promise rejected with a RangeError exception.
            if options.min > byte_length as u64 {
                let e: Value = reader
                    .generic
                    .constructor_range_error
                    .call(("options.min must be less than or equal to views byteLength",))?;
                return promise_rejected_with(&reader.generic.promise_primordials, e);
            }
        }

        // If this.[[stream]] is undefined, return a promise rejected with a TypeError exception.
        if reader.generic.stream.is_none() {
            let e: Value = reader
                .generic
                .constructor_type_error
                .call(("Cannot read a stream using a released reader",))?;
            return promise_rejected_with(&reader.generic.promise_primordials, e);
        }

        // Let promise be a new promise.
        let promise = ResolveablePromise::new(&ctx)?;
        // Let readIntoRequest be a new read-into request with the following items:
        #[derive(Trace)]
        struct ReadIntoRequest<'js> {
            promise: ResolveablePromise<'js>,
        }

        impl<'js> ReadableStreamReadIntoRequest<'js> for ReadIntoRequest<'js> {
            // chunk steps, given chunk
            // Resolve promise with «[ "value" → chunk, "done" → false ]».
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
                self.promise.resolve(ReadableStreamReadResult {
                    value: Some(chunk),
                    done: false,
                })?;
                Ok(objects)
            }

            // close steps, given chunk
            // Resolve promise with «[ "value" → chunk, "done" → true ]».
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
                self.promise.resolve(ReadableStreamReadResult {
                    value: Some(chunk),
                    done: true,
                })?;
                Ok(objects)
            }

            // error steps, given e
            // Reject promise with e.
            fn error_steps(
                &self,
                objects: ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
                reason: Value<'js>,
            ) -> Result<
                ReadableStreamObjects<
                    'js,
                    ReadableByteStreamControllerOwned<'js>,
                    ReadableStreamBYOBReaderOwned<'js>,
                >,
            > {
                self.promise.reject(reason)?;
                Ok(objects)
            }
        }

        let stream = OwnedBorrowMut::from_class(
            reader
                .generic
                .stream
                .clone()
                .expect("ReadableStreamBYOBReader read called without stream"),
        );

        let controller = ReadableByteStreamControllerOwned::<'js>::try_from_erased_class(
            stream.controller.clone(),
        )
        .expect("releaseLock called on byob reader without byte controller");

        let objects = ReadableStreamObjects {
            stream,
            controller,
            reader: reader.0,
        };

        // Perform ! ReadableStreamBYOBReaderRead(this, view, options["min"], readIntoRequest).
        Self::readable_stream_byob_reader_read(
            &ctx,
            objects,
            view,
            options.min,
            ReadIntoRequest {
                promise: promise.clone(),
            },
        )?;

        // Return promise.
        Ok(promise.promise)
    }

    fn release_lock(reader: This<OwnedBorrowMut<'js, Self>>) -> Result<()> {
        // If this.[[stream]] is undefined, return.
        let stream = match reader.generic.stream.clone() {
            None => {
                return Ok(());
            },
            Some(stream) => OwnedBorrowMut::from_class(stream),
        };

        let controller = ReadableByteStreamControllerOwned::<'js>::try_from_erased_class(
            stream.controller.clone(),
        )
        .expect("releaseLock called on byob reader without byte controller");

        let objects = ReadableStreamObjects {
            stream,
            controller,
            reader: reader.0,
        };

        // Perform ! ReadableStreamBYOBReaderRelease(this).
        Self::readable_stream_byob_reader_release(objects)?;

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
                let e: Value = reader
                    .generic
                    .constructor_type_error
                    .call(("Cannot cancel a stream using a released reader",))?;
                return promise_rejected_with(&reader.generic.promise_primordials, e);
            },
            Some(stream) => OwnedBorrowMut::from_class(stream),
        };

        let controller = ReadableByteStreamControllerOwned::<'js>::try_from_erased_class(
            stream.controller.clone(),
        )
        .expect("releaseLock called on byob reader without byte controller");

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

struct ReadableStreamBYOBReaderReadOptions {
    min: u64,
}

impl<'js> FromJs<'js> for ReadableStreamBYOBReaderReadOptions {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let min = obj.get_optional::<_, f64>("min")?.unwrap_or(1.0);
        if min < u64::MIN as f64 || min > u64::MAX as f64 {
            return Err(Exception::throw_type(
                ctx,
                "min on ReadableStreamBYOBReaderReadOptions must fit into unsigned long long",
            ));
        };

        Ok(Self { min: min as u64 })
    }
}

pub(super) trait ReadableStreamReadIntoRequest<'js>: Trace<'js> {
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
    >;

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
    >;

    fn error_steps(
        &self,
        objects: ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
        reason: Value<'js>,
    ) -> Result<
        ReadableStreamObjects<
            'js,
            ReadableByteStreamControllerOwned<'js>,
            ReadableStreamBYOBReaderOwned<'js>,
        >,
    >;
}

impl<'js> Trace<'js> for Box<dyn ReadableStreamReadIntoRequest<'js> + 'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.as_ref().trace(tracer);
    }
}

#[derive(JsLifetime, Clone)]
pub(super) struct ViewBytes<'js>(ObjectBytes<'js>);

impl<'js> ViewBytes<'js> {
    pub(super) fn from_object(
        ctx: &Ctx<'js>,
        function_array_buffer_is_view: &Function<'js>,
        object: &Object<'js>,
    ) -> Result<Self> {
        if function_array_buffer_is_view.call::<_, bool>((object.clone(),))? {
            if let Some(view) = ObjectBytes::from_array_buffer(object)? {
                return Ok(Self(view));
            }
        }

        Err(Exception::throw_type(
            ctx,
            "view must be an ArrayBufferView",
        ))
    }

    pub(super) fn from_value(
        ctx: &Ctx<'js>,
        function_array_buffer_is_view: &Function<'js>,
        value: &Value<'js>,
    ) -> Result<Self> {
        match value.as_object() {
            None => {
                Err(Exception::throw_type(
                    ctx,
                    "view must be typed DataView, Buffer, ArrayBuffer, or Uint8Array, but is not an object",
                ))
            },
            Some(object) => Self::from_object(ctx, function_array_buffer_is_view, object),
        }
    }

    pub(super) fn get_array_buffer(&self) -> Result<(ArrayBuffer<'js>, usize, usize)> {
        Ok(self
            .0
            .get_array_buffer()?
            .expect("invariant broken; ViewBytes may not contain ObjectBytes::Vec"))
    }

    pub(super) fn element_size(&self) -> usize {
        match self.0 {
            ObjectBytes::U8Array(_) => 1,
            ObjectBytes::I8Array(_) => 1,
            ObjectBytes::U16Array(_) => 2,
            ObjectBytes::I16Array(_) => 2,
            ObjectBytes::U32Array(_) => 4,
            ObjectBytes::I32Array(_) => 4,
            ObjectBytes::U64Array(_) => 8,
            ObjectBytes::I64Array(_) => 8,
            ObjectBytes::F32Array(_) => 4,
            ObjectBytes::F64Array(_) => 8,
            ObjectBytes::DataView(_) => 1,
            ObjectBytes::Vec(_) => {
                panic!("invariant broken; ViewBytes may not contain ObjectBytes::Vec")
            },
        }
    }

    pub(super) fn atom(&self) -> PredefinedAtom {
        match self.0 {
            ObjectBytes::U8Array(_) => PredefinedAtom::Uint8Array,
            ObjectBytes::I8Array(_) => PredefinedAtom::Int8Array,
            ObjectBytes::U16Array(_) => PredefinedAtom::Uint16Array,
            ObjectBytes::I16Array(_) => PredefinedAtom::Int16Array,
            ObjectBytes::U32Array(_) => PredefinedAtom::Uint32Array,
            ObjectBytes::I32Array(_) => PredefinedAtom::Int32Array,
            ObjectBytes::U64Array(_) => PredefinedAtom::BigUint64Array,
            ObjectBytes::I64Array(_) => PredefinedAtom::BigInt64Array,
            ObjectBytes::F32Array(_) => PredefinedAtom::Float32Array,
            ObjectBytes::F64Array(_) => PredefinedAtom::Float64Array,
            ObjectBytes::DataView(_) => PredefinedAtom::DataView,
            ObjectBytes::Vec(_) => {
                panic!("invariant broken; ViewBytes may not contain ObjectBytes::Vec")
            },
        }
    }
}

impl<'js> Trace<'js> for ViewBytes<'js> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.0.trace(tracer);
    }
}

impl<'js> IntoJs<'js> for ViewBytes<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        self.0.into_js(ctx)
    }
}

impl<'js> ReadableStreamReader<'js> for ReadableStreamBYOBReaderOwned<'js> {
    type Class = ReadableStreamBYOBReaderClass<'js>;

    fn with_reader<C>(
        self,
        ctx: C,
        _: impl FnOnce(
            C,
            ReadableStreamDefaultReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamDefaultReaderOwned<'js>)>,
        byob: impl FnOnce(
            C,
            ReadableStreamBYOBReaderOwned<'js>,
        ) -> Result<(C, ReadableStreamBYOBReaderOwned<'js>)>,
        _: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)> {
        byob(ctx, self)
    }

    fn into_inner(self) -> Self::Class {
        self.into_inner()
    }

    fn from_class(class: Self::Class) -> Self {
        OwnedBorrowMut::from_class(class)
    }

    fn try_from_erased(erased: Option<ReadableStreamReaderOwned<'js>>) -> Option<Self> {
        match erased {
            Some(ReadableStreamReaderOwned::ReadableStreamBYOBReader(r)) => Some(r),
            _ => None,
        }
    }
}
