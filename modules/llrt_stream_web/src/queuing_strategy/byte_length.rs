use rquickjs::{class::Trace, methods, Class, Ctx, JsLifetime, Result};

use super::{NativeSizeFunction, QueueingStrategyInit};

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct ByteLengthQueuingStrategy<'js> {
    high_water_mark: f64,
    size: Class<'js, NativeSizeFunction>,
}

#[methods(rename_all = "camelCase")]
impl<'js> ByteLengthQueuingStrategy<'js> {
    #[qjs(constructor)]
    pub(crate) fn new(ctx: Ctx<'js>, init: QueueingStrategyInit) -> Result<Self> {
        // Set this.[[highWaterMark]] to init["highWaterMark"].
        Ok(Self {
            high_water_mark: init.high_water_mark,
            size: Class::instance(ctx, NativeSizeFunction::ByteLength)?,
        })
    }

    // readonly attribute Function size;
    // size is an attribute, not a method, so this function is not itself the size function, but instead returns one
    #[qjs(get)]
    pub(crate) fn size(&self) -> Class<'js, NativeSizeFunction> {
        self.size.clone()
    }

    // readonly attribute unrestricted double highWaterMark;
    #[qjs(get)]
    pub(crate) fn high_water_mark(&self) -> f64 {
        self.high_water_mark
    }
}
