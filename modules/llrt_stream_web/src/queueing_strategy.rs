use rquickjs::{
    class::Trace, methods, Ctx, Error, Exception, FromJs, Function, JsLifetime, Object, Result,
    Value,
};

use super::ObjectExt;

pub(super) struct QueuingStrategy<'js> {
    // unrestricted double highWaterMark;
    high_water_mark: Option<Value<'js>>,
    // callback QueuingStrategySize = unrestricted double (any chunk);
    pub(super) size: Option<Function<'js>>,
}

impl<'js> FromJs<'js> for QueuingStrategy<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let high_water_mark = obj.get_optional::<_, Value>("highWaterMark")?;
        let size = obj.get_optional::<_, _>("size")?;

        Ok(Self {
            high_water_mark,
            size,
        })
    }
}

impl<'js> QueuingStrategy<'js> {
    // https://streams.spec.whatwg.org/#validate-and-normalize-high-water-mark
    pub(super) fn extract_high_water_mark(
        ctx: &Ctx<'js>,
        this: Option<QueuingStrategy<'js>>,
        default_hwm: f64,
    ) -> Result<f64> {
        match this {
            // If strategy["highWaterMark"] does not exist, return defaultHWM.
            None => Ok(default_hwm),
            Some(this) => {
                // Let highWaterMark be strategy["highWaterMark"].
                if let Some(high_water_mark) = &this.high_water_mark {
                    let high_water_mark = high_water_mark.as_number().unwrap_or(f64::NAN);
                    // If highWaterMark is NaN or highWaterMark < 0, throw a RangeError exception.
                    if high_water_mark.is_nan() || high_water_mark < 0.0 {
                        Err(Exception::throw_range(ctx, "Invalid highWaterMark"))
                    } else {
                        // Return highWaterMark.
                        Ok(high_water_mark)
                    }
                } else {
                    // If strategy["highWaterMark"] does not exist, return defaultHWM.
                    Ok(default_hwm)
                }
            },
        }
    }

    // https://streams.spec.whatwg.org/#make-size-algorithm-from-size-function
    pub(super) fn extract_size_algorithm(
        this: Option<&QueuingStrategy<'js>>,
    ) -> SizeAlgorithm<'js> {
        // If strategy["size"] does not exist, return an algorithm that returns 1.
        match this.as_ref().and_then(|t| t.size.as_ref()) {
            None => SizeAlgorithm::AlwaysOne,
            Some(size) => SizeAlgorithm::SizeFunction(size.clone()),
        }
    }
}

#[derive(JsLifetime, Trace, Clone)]
pub(super) enum SizeAlgorithm<'js> {
    AlwaysOne,
    SizeFunction(Function<'js>),
}

impl<'js> SizeAlgorithm<'js> {
    pub(super) fn call(&self, ctx: Ctx<'js>, chunk: Value<'js>) -> Result<Value<'js>> {
        match self {
            Self::AlwaysOne => Ok(Value::new_number(ctx, 1.0)),
            Self::SizeFunction(ref f) => f.call((chunk.clone(),)),
        }
    }
}

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct CountQueuingStrategy<'js> {
    high_water_mark: f64,
    size: Function<'js>,
}

#[methods(rename_all = "camelCase")]
impl<'js> CountQueuingStrategy<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>, init: QueueingStrategyInit) -> Result<Self> {
        // Set this.[[highWaterMark]] to init["highWaterMark"].
        Ok(Self {
            high_water_mark: init.high_water_mark,
            size: Function::new(ctx, count_queueing_strategy_size_function)?,
        })
    }

    #[qjs(get)]
    fn size(&self) -> Function<'js> {
        self.size.clone()
    }

    #[qjs(get)]
    fn high_water_mark(&self) -> f64 {
        self.high_water_mark
    }
}

fn count_queueing_strategy_size_function() -> f64 {
    // Return 1.
    1.0
}

struct QueueingStrategyInit {
    high_water_mark: f64,
}

impl<'js> FromJs<'js> for QueueingStrategyInit {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let high_water_mark = obj
            .get_optional("highWaterMark")?
            .ok_or(Error::new_from_js(ty_name, "QueueingStrategyInit"))?;

        Ok(Self { high_water_mark })
    }
}

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct ByteLengthQueuingStrategy<'js> {
    high_water_mark: f64,
    size: Function<'js>,
}

#[methods(rename_all = "camelCase")]
impl<'js> ByteLengthQueuingStrategy<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>, init: QueueingStrategyInit) -> Result<Self> {
        // Set this.[[highWaterMark]] to init["highWaterMark"].
        Ok(Self {
            high_water_mark: init.high_water_mark,
            size: Function::new(ctx, byte_length_queueing_strategy_size_function)?,
        })
    }

    #[qjs(get)]
    fn size(&self) -> Function<'js> {
        self.size.clone()
    }

    #[qjs(get)]
    fn high_water_mark(&self) -> f64 {
        self.high_water_mark
    }
}

fn byte_length_queueing_strategy_size_function(chunk: Object<'_>) -> Result<f64> {
    chunk.get("byteLength")
}
