use rquickjs::{Function, Object, Result, Value};

use crate::utils::ValueOrUndefined;

#[derive(Default)]
pub struct UnderlyingSink<'js> {
    // callback UnderlyingSinkStartCallback = any (WritableStreamDefaultController controller);
    pub start: Option<Function<'js>>,
    // callback UnderlyingSinkWriteCallback = Promise<undefined> (any chunk, WritableStreamDefaultController controller);
    pub write: Option<Function<'js>>,
    // callback UnderlyingSinkCloseCallback = Promise<undefined> ();
    pub close: Option<Function<'js>>,
    // callback UnderlyingSinkAbortCallback = Promise<undefined> (optional any reason);
    pub abort: Option<Function<'js>>,
    pub r#type: Option<Value<'js>>,
}

impl<'js> UnderlyingSink<'js> {
    pub fn from_object(obj: Object<'js>) -> Result<Self> {
        let start = obj.get_value_or_undefined::<_, _>("start")?;
        let write = obj.get_value_or_undefined::<_, _>("write")?;
        let close = obj.get_value_or_undefined::<_, _>("close")?;
        let abort = obj.get_value_or_undefined::<_, _>("abort")?;
        let r#type = obj.get_value_or_undefined::<_, _>("type")?;

        Ok(Self {
            start,
            write,
            close,
            abort,
            r#type,
        })
    }
}
