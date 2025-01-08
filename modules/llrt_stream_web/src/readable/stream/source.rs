use rquickjs::{Function, Object, Result};

use crate::{readable::stream::ReadableStreamType, utils::ValueOrUndefined};

#[derive(Default)]
pub(crate) struct UnderlyingSource<'js> {
    // callback UnderlyingSourceStartCallback = any (ReadableStreamController controller);
    pub(crate) start: Option<Function<'js>>,
    // callback UnderlyingSourcePullCallback = Promise<undefined> (ReadableStreamController controller);
    pub(crate) pull: Option<Function<'js>>,
    // callback UnderlyingSourceCancelCallback = Promise<undefined> (optional any reason);
    pub(crate) cancel: Option<Function<'js>>,
    pub(super) r#type: Option<ReadableStreamType>,
    // [EnforceRange] unsigned long long autoAllocateChunkSize;
    pub(crate) auto_allocate_chunk_size: Option<usize>,
}

impl<'js> UnderlyingSource<'js> {
    pub(super) fn from_object(obj: Object<'js>) -> Result<Self> {
        let start = obj.get_value_or_undefined::<_, _>("start")?;
        let pull = obj.get_value_or_undefined::<_, _>("pull")?;
        let cancel = obj.get_value_or_undefined::<_, _>("cancel")?;
        let r#type = obj.get_value_or_undefined::<_, _>("type")?;
        let auto_allocate_chunk_size =
            obj.get_value_or_undefined::<_, _>("autoAllocateChunkSize")?;

        Ok(Self {
            start,
            pull,
            cancel,
            r#type,
            auto_allocate_chunk_size,
        })
    }
}
