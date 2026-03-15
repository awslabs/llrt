use rquickjs::{Function, Object, Result};

use crate::utils::ValueOrUndefined;

/// dictionary Transformer {
///   TransformerStartCallback start;
///   TransformerTransformCallback transform;
///   TransformerFlushCallback flush;
///   TransformerCancelCallback cancel;
///   any readableType;
///   any writableType;
/// };
#[derive(Default)]
pub(super) struct Transformer<'js> {
    pub start: Option<Function<'js>>,
    pub transform: Option<Function<'js>>,
    pub flush: Option<Function<'js>>,
    pub cancel: Option<Function<'js>>,
    pub readable_type: bool,
    pub writable_type: bool,
}

impl<'js> Transformer<'js> {
    pub fn from_object(obj: Object<'js>) -> Result<Self> {
        let start = obj.get_value_or_undefined::<_, _>("start")?;
        let transform = obj.get_value_or_undefined::<_, _>("transform")?;
        let flush = obj.get_value_or_undefined::<_, _>("flush")?;
        let cancel = obj.get_value_or_undefined::<_, _>("cancel")?;
        let readable_type: Option<rquickjs::Value<'js>> =
            obj.get_value_or_undefined("readableType")?;
        let writable_type: Option<rquickjs::Value<'js>> =
            obj.get_value_or_undefined("writableType")?;

        Ok(Self {
            start,
            transform,
            flush,
            cancel,
            readable_type: readable_type.is_some(),
            writable_type: writable_type.is_some(),
        })
    }
}
