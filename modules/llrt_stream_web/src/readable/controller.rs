use rquickjs::{
    class::{OwnedBorrowMut, Trace},
    Ctx, IntoJs, JsLifetime, Promise, Result, Value,
};

use super::{
    byte_controller::{ReadableByteStreamControllerClass, ReadableByteStreamControllerOwned},
    default_controller::{
        ReadableStreamDefaultControllerClass, ReadableStreamDefaultControllerOwned,
    },
    default_reader::ReadableStreamDefaultReaderOwned,
    objects::ReadableStreamObjects,
    reader::ReadableStreamReader,
    ReadableStreamReadRequest,
};

pub(super) trait ReadableStreamController<'js>: Sized {
    type Class: Clone + Trace<'js>;

    fn with_controller<C, O>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            ReadableStreamDefaultControllerOwned<'js>,
        ) -> Result<(O, ReadableStreamDefaultControllerOwned<'js>)>,
        byte: impl FnOnce(
            C,
            ReadableByteStreamControllerOwned<'js>,
        ) -> Result<(O, ReadableByteStreamControllerOwned<'js>)>,
    ) -> Result<(O, Self)>;

    fn into_inner(self) -> Self::Class;
    fn from_class(class: Self::Class) -> Self;

    fn into_erased(self) -> ReadableStreamControllerOwned<'js>;
    fn try_from_erased(erased: ReadableStreamControllerOwned<'js>) -> Option<Self>;
    fn try_from_erased_class(
        erased_class: <ReadableStreamControllerOwned<'js> as ReadableStreamController<'js>>::Class,
    ) -> Option<Self> {
        Self::try_from_erased(ReadableStreamController::from_class(erased_class))
    }

    fn pull_steps(
        ctx: &Ctx<'js>,
        objects: ReadableStreamObjects<'js, Self, ReadableStreamDefaultReaderOwned<'js>>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) -> Result<ReadableStreamObjects<'js, Self, ReadableStreamDefaultReaderOwned<'js>>>;

    fn cancel_steps<R: ReadableStreamReader<'js>>(
        ctx: &Ctx<'js>,
        objects: ReadableStreamObjects<'js, Self, R>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, ReadableStreamObjects<'js, Self, R>)>;

    fn release_steps(&mut self);
}

#[derive(JsLifetime, Trace, Clone)]
pub(super) enum ReadableStreamControllerClass<'js> {
    ReadableStreamDefaultController(ReadableStreamDefaultControllerClass<'js>),
    ReadableStreamByteController(ReadableByteStreamControllerClass<'js>),
}

impl<'js> IntoJs<'js> for ReadableStreamControllerClass<'js> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self {
            Self::ReadableStreamDefaultController(c) => c.into_js(ctx),
            Self::ReadableStreamByteController(c) => c.into_js(ctx),
        }
    }
}

pub(super) enum ReadableStreamControllerOwned<'js> {
    ReadableStreamDefaultController(ReadableStreamDefaultControllerOwned<'js>),
    ReadableStreamByteController(ReadableByteStreamControllerOwned<'js>),
}

impl<'js> ReadableStreamController<'js> for ReadableStreamControllerOwned<'js> {
    type Class = ReadableStreamControllerClass<'js>;

    fn with_controller<C, O>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            ReadableStreamDefaultControllerOwned<'js>,
        ) -> Result<(O, ReadableStreamDefaultControllerOwned<'js>)>,
        byob: impl FnOnce(
            C,
            ReadableByteStreamControllerOwned<'js>,
        ) -> Result<(O, ReadableByteStreamControllerOwned<'js>)>,
    ) -> Result<(O, Self)> {
        match self {
            ReadableStreamControllerOwned::ReadableStreamDefaultController(r) => {
                let (ctx, r) = default(ctx, r)?;
                Ok((ctx, Self::ReadableStreamDefaultController(r)))
            },
            ReadableStreamControllerOwned::ReadableStreamByteController(r) => {
                let (ctx, r) = byob(ctx, r)?;
                Ok((ctx, Self::ReadableStreamByteController(r)))
            },
        }
    }

    fn into_inner(self) -> Self::Class {
        match self {
            ReadableStreamControllerOwned::ReadableStreamDefaultController(c) => {
                ReadableStreamControllerClass::ReadableStreamDefaultController(c.into_inner())
            },
            ReadableStreamControllerOwned::ReadableStreamByteController(c) => {
                ReadableStreamControllerClass::ReadableStreamByteController(c.into_inner())
            },
        }
    }

    fn from_class(class: Self::Class) -> Self {
        match class {
            ReadableStreamControllerClass::ReadableStreamDefaultController(class) => {
                ReadableStreamControllerOwned::ReadableStreamDefaultController(
                    OwnedBorrowMut::from_class(class),
                )
            },
            ReadableStreamControllerClass::ReadableStreamByteController(class) => {
                ReadableStreamControllerOwned::ReadableStreamByteController(
                    OwnedBorrowMut::from_class(class),
                )
            },
        }
    }

    fn into_erased(self) -> ReadableStreamControllerOwned<'js> {
        self
    }

    fn try_from_erased(erased: ReadableStreamControllerOwned<'js>) -> Option<Self> {
        Some(erased)
    }

    fn pull_steps(
        ctx: &Ctx<'js>,
        objects: ReadableStreamObjects<'js, Self, ReadableStreamDefaultReaderOwned<'js>>,
        read_request: impl ReadableStreamReadRequest<'js> + 'js,
    ) -> Result<ReadableStreamObjects<'js, Self, ReadableStreamDefaultReaderOwned<'js>>> {
        objects
            .with_controller(
                read_request,
                |read_request, objects| {
                    ReadableStreamDefaultControllerOwned::<'js>::pull_steps(
                        ctx,
                        objects,
                        read_request,
                    )
                    .map(|objects| ((), objects))
                },
                |read_request, objects| {
                    ReadableByteStreamControllerOwned::<'js>::pull_steps(ctx, objects, read_request)
                        .map(|objects| ((), objects))
                },
            )
            .map(|((), objects)| objects)
    }

    fn cancel_steps<R: ReadableStreamReader<'js>>(
        ctx: &Ctx<'js>,
        objects: ReadableStreamObjects<'js, Self, R>,
        reason: Value<'js>,
    ) -> Result<(Promise<'js>, ReadableStreamObjects<'js, Self, R>)> {
        objects.with_controller(
            reason,
            |reason, objects| {
                ReadableStreamDefaultControllerOwned::<'js>::cancel_steps(ctx, objects, reason)
            },
            |reason, objects| {
                ReadableByteStreamControllerOwned::<'js>::cancel_steps(ctx, objects, reason)
            },
        )
    }

    fn release_steps(&mut self) {
        match self {
            ReadableStreamControllerOwned::ReadableStreamDefaultController(c) => c.release_steps(),
            ReadableStreamControllerOwned::ReadableStreamByteController(c) => c.release_steps(),
        }
    }
}

impl<'js> From<ReadableStreamDefaultControllerOwned<'js>> for ReadableStreamControllerOwned<'js> {
    fn from(value: ReadableStreamDefaultControllerOwned<'js>) -> Self {
        Self::ReadableStreamDefaultController(value)
    }
}

impl<'js> From<ReadableByteStreamControllerOwned<'js>> for ReadableStreamControllerOwned<'js> {
    fn from(value: ReadableByteStreamControllerOwned<'js>) -> Self {
        Self::ReadableStreamByteController(value)
    }
}
