use rquickjs::{class::Trace, Result};

use super::default_writer::WritableStreamDefaultWriterOwned;

pub(crate) trait WritableStreamWriter<'js>: Sized + 'js {
    type Class: Clone + Trace<'js>;

    fn with_writer<C>(
        self,
        ctx: C,
        default: impl FnOnce(
            C,
            WritableStreamDefaultWriterOwned<'js>,
        ) -> Result<(C, WritableStreamDefaultWriterOwned<'js>)>,
        none: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)>;

    fn into_inner(self) -> Self::Class;

    fn from_class(class: Self::Class) -> Self;
}

#[derive(Clone, Trace)]
pub(super) struct UndefinedWriter;

impl<'js> WritableStreamWriter<'js> for UndefinedWriter {
    type Class = UndefinedWriter;

    fn with_writer<C>(
        self,
        ctx: C,
        _: impl FnOnce(
            C,
            WritableStreamDefaultWriterOwned<'js>,
        ) -> Result<(C, WritableStreamDefaultWriterOwned<'js>)>,
        none: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)> {
        Ok((none(ctx)?, self))
    }

    fn into_inner(self) -> Self::Class {
        self
    }

    fn from_class(class: Self::Class) -> Self {
        class
    }
}

impl<'js, T: WritableStreamWriter<'js>> WritableStreamWriter<'js> for Option<T> {
    type Class = Option<<T as WritableStreamWriter<'js>>::Class>;

    fn with_writer<C>(
        self,
        mut ctx: C,
        default: impl FnOnce(
            C,
            WritableStreamDefaultWriterOwned<'js>,
        ) -> Result<(C, WritableStreamDefaultWriterOwned<'js>)>,
        none: impl FnOnce(C) -> Result<C>,
    ) -> Result<(C, Self)> {
        match self {
            Some(mut writer) => {
                (ctx, writer) = writer.with_writer(ctx, default, none)?;
                Ok((ctx, Some(writer)))
            },
            None => Ok((none(ctx)?, None)),
        }
    }

    fn into_inner(self) -> Self::Class {
        self.map(WritableStreamWriter::into_inner)
    }

    fn from_class(class: Self::Class) -> Self {
        class.map(WritableStreamWriter::from_class)
    }
}
