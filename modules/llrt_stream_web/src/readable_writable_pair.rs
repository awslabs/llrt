use rquickjs::{Ctx, Error, FromJs, Result, Value};

use crate::{readable::ReadableStreamClass, writable::WritableStreamClass};

/// An object containing a pair of linked streams, one readable and one writable
/// https://streams.spec.whatwg.org/#dictdef-readablewritablepair
pub struct ReadableWritablePair<'js> {
    pub readable: ReadableStreamClass<'js>,
    pub writable: WritableStreamClass<'js>,
}

impl<'js> FromJs<'js> for ReadableWritablePair<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let readable = obj.get::<_, ReadableStreamClass<'js>>("readable")?;
        let writable = obj.get::<_, WritableStreamClass<'js>>("writable")?;

        Ok(Self { readable, writable })
    }
}
