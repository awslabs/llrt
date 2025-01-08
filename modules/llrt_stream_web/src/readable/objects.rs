use rquickjs::{
    class::{OwnedBorrowMut, Trace, Tracer},
    Result,
};

use crate::readable::{
    byob_reader::ReadableStreamBYOBReaderOwned,
    byte_controller::ReadableByteStreamControllerOwned,
    controller::{
        ReadableStreamController, ReadableStreamControllerClass, ReadableStreamControllerOwned,
    },
    default_controller::ReadableStreamDefaultControllerOwned,
    default_reader::{ReadableStreamDefaultReaderOrUndefined, ReadableStreamDefaultReaderOwned},
    reader::{ReadableStreamReader, ReadableStreamReaderOwned, UndefinedReader},
    stream::{ReadableStream, ReadableStreamClass, ReadableStreamOwned},
};

pub(super) struct ReadableStreamObjects<'js, C, R> {
    pub(super) stream: ReadableStreamOwned<'js>,
    pub(super) controller: C,
    pub(super) reader: R,
}

pub(super) type ReadableStreamDefaultControllerObjects<'js, R> =
    ReadableStreamObjects<'js, ReadableStreamDefaultControllerOwned<'js>, R>;
pub(super) type ReadableStreamDefaultReaderObjects<'js, C = ReadableStreamControllerOwned<'js>> =
    ReadableStreamObjects<'js, C, ReadableStreamDefaultReaderOwned<'js>>;
pub(super) type ReadableByteStreamObjects<'js, R> =
    ReadableStreamObjects<'js, ReadableByteStreamControllerOwned<'js>, R>;
pub(super) type ReadableStreamBYOBObjects<'js> = ReadableStreamObjects<
    'js,
    ReadableByteStreamControllerOwned<'js>,
    ReadableStreamBYOBReaderOwned<'js>,
>;

pub(super) struct ReadableStreamClassObjects<
    'js,
    C: ReadableStreamController<'js>,
    R: ReadableStreamReader<'js>,
> {
    pub(super) stream: ReadableStreamClass<'js>,
    pub(super) controller: C::Class,
    pub(super) reader: R::Class,
}

// derive(Clone) isn't clever enough to figure out that C and R don't need to implement Clone, but only C::Class and R::Class.
impl<'js, C: ReadableStreamController<'js>, R: ReadableStreamReader<'js>> Clone
    for ReadableStreamClassObjects<'js, C, R>
{
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            controller: self.controller.clone(),
            reader: self.reader.clone(),
        }
    }
}

// derive(Trace) isn't clever enough to figure out that C and R don't need to implement Trace, but only C::Class and R::Class.
impl<'js, C: ReadableStreamController<'js>, R: ReadableStreamReader<'js>> Trace<'js>
    for ReadableStreamClassObjects<'js, C, R>
{
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.stream.trace(tracer);
        self.controller.trace(tracer);
        self.reader.trace(tracer);
    }
}

impl<'js, C: ReadableStreamController<'js>, R: ReadableStreamReader<'js>>
    ReadableStreamClassObjects<'js, C, R>
{
    pub(super) fn set_reader<RNext: ReadableStreamReader<'js>>(
        self,
        reader: RNext::Class,
    ) -> ReadableStreamClassObjects<'js, C, RNext> {
        drop(self.reader);
        ReadableStreamClassObjects {
            stream: self.stream,
            controller: self.controller,
            reader,
        }
    }
}

impl<'js, C: ReadableStreamController<'js>, R: ReadableStreamReader<'js>>
    ReadableStreamObjects<'js, C, R>
{
    pub(super) fn with_assert_default_controller(
        mut self,
        f: impl FnOnce(
            ReadableStreamDefaultControllerObjects<'js, R>,
        ) -> Result<ReadableStreamDefaultControllerObjects<'js, R>>,
    ) -> Result<Self> {
        ((), self) = self.with_controller(
            (),
            |(), controller| Ok(((), f(controller)?)),
            |_, _| panic!("expected default controller, found byte controller"),
        )?;
        Ok(self)
    }

    pub(super) fn with_assert_byte_controller(
        mut self,
        f: impl FnOnce(ReadableByteStreamObjects<'js, R>) -> Result<ReadableByteStreamObjects<'js, R>>,
    ) -> Result<Self> {
        ((), self) = self.with_controller(
            (),
            |_, _| panic!("expected byte controller, found default controller"),
            |(), controller| Ok(((), f(controller)?)),
        )?;
        Ok(self)
    }

    pub(super) fn with_controller<Ctx, O>(
        self,
        ctx: Ctx,
        default: impl FnOnce(
            Ctx,
            ReadableStreamDefaultControllerObjects<'js, R>,
        ) -> Result<(O, ReadableStreamDefaultControllerObjects<'js, R>)>,
        byte: impl FnOnce(
            Ctx,
            ReadableByteStreamObjects<'js, R>,
        ) -> Result<(O, ReadableByteStreamObjects<'js, R>)>,
    ) -> Result<(O, Self)> {
        let ((out, stream, reader), controller) = self.controller.with_controller(
            (ctx, self.stream, self.reader),
            |(ctx, stream, reader), controller| {
                let (out, objects) = default(
                    ctx,
                    ReadableStreamObjects {
                        stream,
                        controller,
                        reader,
                    },
                )?;

                Ok(((out, objects.stream, objects.reader), objects.controller))
            },
            |(ctx, stream, reader), controller| {
                let (out, objects) = byte(
                    ctx,
                    ReadableStreamObjects {
                        stream,
                        controller,
                        reader,
                    },
                )?;

                Ok(((out, objects.stream, objects.reader), objects.controller))
            },
        )?;

        Ok((
            out,
            Self {
                stream,
                controller,
                reader,
            },
        ))
    }

    pub(super) fn with_assert_byob_reader(
        self,
        f: impl FnOnce(ReadableStreamBYOBObjects<'js>) -> Result<ReadableStreamBYOBObjects<'js>>,
    ) -> Result<Self> {
        self.with_reader(
            |_| panic!("expected byob reader, found default reader"),
            f,
            |_| panic!("expected byob reader, found no reader"),
        )
    }

    pub(super) fn with_assert_default_reader(
        self,
        f: impl FnOnce(
            ReadableStreamDefaultReaderObjects<'js, C>,
        ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>>,
    ) -> Result<Self> {
        self.with_reader(
            f,
            |_| panic!("expected default reader, found byob reader"),
            |_| panic!("expected default reader, found no reader"),
        )
    }

    pub(super) fn with_reader(
        mut self,
        default: impl FnOnce(
            ReadableStreamDefaultReaderObjects<'js, C>,
        ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>>,
        byob: impl FnOnce(ReadableStreamBYOBObjects<'js>) -> Result<ReadableStreamBYOBObjects<'js>>,
        none: impl FnOnce(
            ReadableStreamObjects<'js, C, UndefinedReader>,
        ) -> Result<ReadableStreamObjects<'js, C, UndefinedReader>>,
    ) -> Result<Self> {
        ((self.stream, self.controller), self.reader) = self.reader.with_reader(
            (self.stream, self.controller),
            |(stream, controller), reader| {
                let objects = default(ReadableStreamObjects {
                    stream,
                    controller,
                    reader,
                })?;

                Ok(((objects.stream, objects.controller), objects.reader))
            },
            |(mut stream, mut controller), mut reader| {
                ((stream, reader), controller) = controller.with_controller(
                    (stream, reader),
                    |_, _| panic!("byob reader must have a byte controller"),
                    |(stream, reader), controller| {
                        let objects = byob(ReadableStreamObjects {
                            stream,
                            controller,
                            reader,
                        })?;

                        Ok(((objects.stream, objects.reader), objects.controller))
                    },
                )?;

                Ok(((stream, controller), reader))
            },
            |(stream, controller)| {
                let objects = none(ReadableStreamObjects {
                    stream,
                    controller,
                    reader: UndefinedReader,
                })?;

                Ok((objects.stream, objects.controller))
            },
        )?;

        Ok(self)
    }

    pub(super) fn into_inner(self) -> ReadableStreamClassObjects<'js, C, R> {
        ReadableStreamClassObjects {
            stream: self.stream.into_inner(),
            controller: self.controller.into_inner(),
            reader: self.reader.into_inner(),
        }
    }

    pub(super) fn from_class(objects_class: ReadableStreamClassObjects<'js, C, R>) -> Self {
        Self {
            stream: OwnedBorrowMut::from_class(objects_class.stream),
            controller: C::from_class(objects_class.controller),
            reader: R::from_class(objects_class.reader),
        }
    }

    pub(super) fn from_class_no_reader(
        objects_class: ReadableStreamClassObjects<'js, C, R>,
    ) -> ReadableStreamObjects<'js, C, UndefinedReader> {
        ReadableStreamObjects {
            stream: OwnedBorrowMut::from_class(objects_class.stream),
            controller: C::from_class(objects_class.controller),
            reader: UndefinedReader,
        }
    }

    pub(super) fn clear_reader(self) -> ReadableStreamObjects<'js, C, UndefinedReader> {
        drop(self.reader);
        ReadableStreamObjects {
            stream: self.stream,
            controller: self.controller,
            reader: UndefinedReader,
        }
    }
}

impl<'js>
    ReadableStreamDefaultControllerObjects<'js, Option<ReadableStreamDefaultReaderOwned<'js>>>
{
    pub(super) fn from_default_controller(
        controller: ReadableStreamDefaultControllerOwned<'js>,
    ) -> Self {
        Self::new_default(
            OwnedBorrowMut::from_class(controller.stream.clone()),
            controller,
        )
    }

    pub(super) fn new_default(
        stream: ReadableStreamOwned<'js>,
        controller: ReadableStreamDefaultControllerOwned<'js>,
    ) -> Self {
        ReadableStreamObjects {
            stream,
            controller,
            reader: UndefinedReader,
        }
        .refresh_reader()
    }
}

impl<'js, R: ReadableStreamReader<'js>> ReadableStreamDefaultControllerObjects<'js, R> {
    pub(super) fn refresh_reader(
        mut self,
    ) -> ReadableStreamDefaultControllerObjects<'js, Option<ReadableStreamDefaultReaderOwned<'js>>>
    {
        drop(self.reader);
        let reader = self.stream.reader_mut();
        ReadableStreamObjects {
            stream: self.stream,
            controller: self.controller,
            reader: ReadableStreamReader::try_from_erased(reader)
                .expect("default controller must have default reader or no reader"),
        }
    }
}

impl<'js> ReadableByteStreamObjects<'js, UndefinedReader> {
    pub(super) fn from_byte_controller(controller: ReadableByteStreamControllerOwned<'js>) -> Self {
        Self::new_byte(
            OwnedBorrowMut::from_class(controller.stream.clone()),
            controller,
        )
    }

    pub(super) fn new_byte(
        stream: ReadableStreamOwned<'js>,
        controller: ReadableByteStreamControllerOwned<'js>,
    ) -> Self {
        ReadableStreamObjects {
            stream,
            controller,
            reader: UndefinedReader,
        }
    }

    pub(super) fn set_reader<RNext: ReadableStreamReader<'js>>(
        self,
        reader: RNext,
    ) -> ReadableByteStreamObjects<'js, RNext> {
        ReadableStreamObjects {
            stream: self.stream,
            controller: self.controller,
            reader,
        }
    }
}

impl<'js> ReadableStreamBYOBObjects<'js> {
    pub(super) fn from_byob_reader(reader: ReadableStreamBYOBReaderOwned<'js>) -> Self {
        let stream = OwnedBorrowMut::from_class(
            reader
                .generic
                .stream
                .clone()
                .expect("ReadableStreamBYOBReader must have a stream"),
        );
        let controller = match &stream.controller {
            ReadableStreamControllerClass::ReadableStreamByteController(c) => c.clone(),
            _ => panic!("ReadableStreamBYOBReader stream must have byte controller"),
        };
        Self {
            stream,
            controller: OwnedBorrowMut::from_class(controller),
            reader,
        }
    }
}

impl<'js, R: ReadableStreamReader<'js>> ReadableByteStreamObjects<'js, R> {
    pub(super) fn refresh_reader(
        mut self,
    ) -> ReadableByteStreamObjects<'js, Option<ReadableStreamReaderOwned<'js>>> {
        drop(self.reader);
        let reader = self.stream.reader_mut();
        ReadableStreamObjects {
            stream: self.stream,
            controller: self.controller,
            reader,
        }
    }
}

impl<'js, C: ReadableStreamController<'js>, R: ReadableStreamDefaultReaderOrUndefined<'js>>
    ReadableStreamObjects<'js, C, R>
{
    pub(super) fn with_some_reader(
        self,
        default: impl FnOnce(
            ReadableStreamDefaultReaderObjects<'js, C>,
        ) -> Result<ReadableStreamDefaultReaderObjects<'js, C>>,
        none: impl FnOnce(
            ReadableStreamObjects<'js, C, UndefinedReader>,
        ) -> Result<ReadableStreamObjects<'js, C, UndefinedReader>>,
    ) -> Result<Self> {
        self.with_reader(
            default,
            |_| panic!("byob reader cannot implement DefaultReaderOrUndefined"),
            none,
        )
    }
}

impl<'js> ReadableStreamObjects<'js, ReadableStreamControllerOwned<'js>, UndefinedReader> {
    pub(super) fn from_stream(stream: ReadableStreamOwned<'js>) -> Self {
        let controller = ReadableStreamControllerOwned::from_class(stream.controller.clone());
        Self::new(stream, controller)
    }

    fn new(
        stream: OwnedBorrowMut<'js, ReadableStream<'js>>,
        controller: ReadableStreamControllerOwned<'js>,
    ) -> Self {
        ReadableStreamObjects {
            stream,
            controller,
            reader: UndefinedReader,
        }
    }
}

impl<'js, R: ReadableStreamReader<'js>>
    ReadableStreamObjects<'js, ReadableStreamControllerOwned<'js>, R>
{
    pub(super) fn refresh_reader(
        mut self,
    ) -> ReadableStreamObjects<
        'js,
        ReadableStreamControllerOwned<'js>,
        Option<ReadableStreamReaderOwned<'js>>,
    > {
        drop(self.reader);
        let reader = self.stream.reader_mut();
        ReadableStreamObjects {
            stream: self.stream,
            controller: self.controller,
            reader,
        }
    }
}

impl<'js> ReadableStreamDefaultReaderObjects<'js> {
    pub(super) fn from_default_reader(reader: ReadableStreamDefaultReaderOwned<'js>) -> Self {
        let stream = OwnedBorrowMut::from_class(
            reader
                .generic
                .stream
                .clone()
                .expect("ReadableStreamDefaultReader must have a stream"),
        );
        let controller = ReadableStreamControllerOwned::from_class(stream.controller.clone());
        Self {
            stream,
            controller,
            reader,
        }
    }
}
