use llrt_utils::primordials::Primordial;
use rquickjs::{class::Trace, methods, Ctx, Function, JsLifetime, Result};

use super::{NativeSizeFunctionPrimordials, QueueingStrategyInit};

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct CountQueuingStrategy<'js> {
    high_water_mark: f64,
    size: Function<'js>,
}

#[methods(rename_all = "camelCase")]
impl<'js> CountQueuingStrategy<'js> {
    #[qjs(constructor)]
    pub(crate) fn new(ctx: Ctx<'js>, init: QueueingStrategyInit) -> Result<Self> {
        // Set this.[[highWaterMark]] to init["highWaterMark"].
        Ok(Self {
            high_water_mark: init.high_water_mark,
            size: NativeSizeFunctionPrimordials::get(&ctx)?.count.clone(),
        })
    }

    // readonly attribute Function size;
    // size is an attribute, not a method, so this function is not itself the size function, but instead returns one
    #[qjs(get)]
    pub(crate) fn size(&self) -> Function<'js> {
        self.size.clone()
    }

    // readonly attribute unrestricted double highWaterMark;
    #[qjs(get)]
    pub(crate) fn high_water_mark(&self) -> f64 {
        self.high_water_mark
    }
}
