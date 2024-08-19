use rquickjs::{class::OwnedBorrowMut, Class, Result};

use super::{
    default_controller::{
        WritableStreamDefaultControllerClass, WritableStreamDefaultControllerOwned,
    },
    default_writer::WritableStreamDefaultWriterOwned,
    writer::{UndefinedWriter, WritableStreamWriter},
    WritableStream, WritableStreamOwned,
};

pub(crate) struct WritableStreamObjects<'js, W> {
    pub(crate) stream: WritableStreamOwned<'js>,
    pub(crate) controller: WritableStreamDefaultControllerOwned<'js>,
    pub(crate) writer: W,
}

pub(crate) struct WritableStreamClassObjects<'js, W: WritableStreamWriter<'js>> {
    pub(crate) stream: Class<'js, WritableStream<'js>>,
    pub(crate) controller: WritableStreamDefaultControllerClass<'js>,
    pub(crate) writer: W::Class,
}

impl<'js, W: WritableStreamWriter<'js>> Clone for WritableStreamClassObjects<'js, W> {
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            controller: self.controller.clone(),
            writer: self.writer.clone(),
        }
    }
}

impl<'js, W: WritableStreamWriter<'js>> WritableStreamObjects<'js, W> {
    pub(super) fn into_inner(self) -> WritableStreamClassObjects<'js, W> {
        WritableStreamClassObjects {
            stream: self.stream.into_inner(),
            controller: self.controller.into_inner(),
            writer: self.writer.into_inner(),
        }
    }

    pub(crate) fn from_class(objects_class: WritableStreamClassObjects<'js, W>) -> Self {
        Self {
            stream: OwnedBorrowMut::from_class(objects_class.stream),
            controller: OwnedBorrowMut::from_class(objects_class.controller),
            writer: W::from_class(objects_class.writer),
        }
    }

    pub(super) fn from_class_no_writer(
        objects_class: WritableStreamClassObjects<'js, W>,
    ) -> WritableStreamObjects<'js, UndefinedWriter> {
        WritableStreamObjects {
            stream: OwnedBorrowMut::from_class(objects_class.stream),
            controller: OwnedBorrowMut::from_class(objects_class.controller),
            writer: UndefinedWriter,
        }
    }

    pub(super) fn with_writer(
        mut self,
        default: impl FnOnce(
            WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
        ) -> Result<
            WritableStreamObjects<'js, WritableStreamDefaultWriterOwned<'js>>,
        >,
        none: impl FnOnce(
            WritableStreamObjects<'js, UndefinedWriter>,
        ) -> Result<WritableStreamObjects<'js, UndefinedWriter>>,
    ) -> Result<Self> {
        ((self.stream, self.controller), self.writer) = self.writer.with_writer(
            (self.stream, self.controller),
            |(stream, controller), writer| {
                let objects = default(WritableStreamObjects {
                    stream,
                    controller,
                    writer,
                })?;

                Ok(((objects.stream, objects.controller), objects.writer))
            },
            |(stream, controller)| {
                let objects = none(WritableStreamObjects {
                    stream,
                    controller,
                    writer: UndefinedWriter,
                })?;

                Ok((objects.stream, objects.controller))
            },
        )?;

        Ok(self)
    }
}

impl<'js, W: WritableStreamWriter<'js>> WritableStreamObjects<'js, W> {
    pub(super) fn refresh_writer(
        mut self,
    ) -> WritableStreamObjects<'js, Option<WritableStreamDefaultWriterOwned<'js>>> {
        drop(self.writer);
        let writer = self.stream.writer_mut();
        WritableStreamObjects {
            stream: self.stream,
            controller: self.controller,
            writer,
        }
    }
}
