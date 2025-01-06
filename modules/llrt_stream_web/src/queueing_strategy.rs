use rquickjs::{
    class::{JsCell, JsClass, Readable, Trace},
    function::{Constructor, Params},
    methods, Class, Ctx, Error, Exception, FromJs, Function, JsLifetime, Object, Result, Value,
};

use super::ValueOrUndefined;

pub(super) struct QueuingStrategy<'js> {
    // unrestricted double highWaterMark;
    high_water_mark: Option<Value<'js>>,
    // callback QueuingStrategySize = unrestricted double (any chunk);
    pub(super) size: Option<SizeFunction<'js>>,
}

impl<'js> FromJs<'js> for QueuingStrategy<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or(Error::new_from_js(ty_name, "Object"))?;

        let high_water_mark = obj.get_value_or_undefined::<_, Value>("highWaterMark")?;
        let size = obj.get_value_or_undefined::<_, _>("size")?;

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
    SizeFunction(SizeFunction<'js>),
}

impl<'js> SizeAlgorithm<'js> {
    pub(super) fn call(&self, ctx: Ctx<'js>, chunk: Value<'js>) -> Result<SizeValue<'js>> {
        match self {
            Self::AlwaysOne
            | Self::SizeFunction(SizeFunction::Native(NativeSizeFunction::Count)) => {
                Ok(SizeValue::Native(1.0))
            },
            Self::SizeFunction(SizeFunction::Js(ref f)) => f.call((chunk.clone(),)),
            Self::SizeFunction(SizeFunction::Native(NativeSizeFunction::ByteLength)) => {
                let size = byte_length_queueing_strategy_size_function(&ctx, &chunk)?;
                SizeValue::from_js(&ctx, size)
            },
        }
    }
}

pub(super) enum SizeValue<'js> {
    Value(Value<'js>),
    Native(f64),
}

impl SizeValue<'_> {
    pub(super) fn as_number(&self) -> Option<f64> {
        match self {
            Self::Value(value) => value.as_number(),
            Self::Native(size) => Some(*size),
        }
    }
}

impl<'js> FromJs<'js> for SizeValue<'js> {
    fn from_js(_: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if let Some(size) = value.as_number() {
            return Ok(Self::Native(size));
        }

        Ok(Self::Value(value))
    }
}

#[derive(JsLifetime, Trace, Clone)]
pub(super) enum SizeFunction<'js> {
    Js(Function<'js>),
    Native(NativeSizeFunction),
}

impl<'js> FromJs<'js> for SizeFunction<'js> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if let Ok(nsf) = Class::<NativeSizeFunction>::from_value(&value) {
            return Ok(SizeFunction::Native(*nsf.borrow()));
        }

        Ok(SizeFunction::Js(Function::from_js(ctx, value)?))
    }
}

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct CountQueuingStrategy<'js> {
    high_water_mark: f64,
    size: Class<'js, NativeSizeFunction>,
}

#[methods(rename_all = "camelCase")]
impl<'js> CountQueuingStrategy<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>, init: QueueingStrategyInit) -> Result<Self> {
        // Set this.[[highWaterMark]] to init["highWaterMark"].
        Ok(Self {
            high_water_mark: init.high_water_mark,
            size: Class::instance(ctx, NativeSizeFunction::Count)?,
        })
    }

    #[qjs(get)]
    fn size(&self) -> Class<'js, NativeSizeFunction> {
        self.size.clone()
    }

    #[qjs(get)]
    fn high_water_mark(&self) -> f64 {
        self.high_water_mark
    }
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
            .get_value_or_undefined("highWaterMark")?
            .ok_or(Error::new_from_js(ty_name, "QueueingStrategyInit"))?;

        Ok(Self { high_water_mark })
    }
}

#[derive(JsLifetime, Trace)]
#[rquickjs::class]
pub(crate) struct ByteLengthQueuingStrategy<'js> {
    high_water_mark: f64,
    size: Class<'js, NativeSizeFunction>,
}

#[methods(rename_all = "camelCase")]
impl<'js> ByteLengthQueuingStrategy<'js> {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'js>, init: QueueingStrategyInit) -> Result<Self> {
        // Set this.[[highWaterMark]] to init["highWaterMark"].
        Ok(Self {
            high_water_mark: init.high_water_mark,
            size: Class::instance(ctx, NativeSizeFunction::ByteLength)?,
        })
    }

    #[qjs(get)]
    fn size(&self) -> Class<'js, NativeSizeFunction> {
        self.size.clone()
    }

    #[qjs(get)]
    fn high_water_mark(&self) -> f64 {
        self.high_water_mark
    }
}

#[derive(JsLifetime, Trace, Clone, Copy)]
pub(super) enum NativeSizeFunction {
    ByteLength,
    Count,
}

impl<'js> JsClass<'js> for NativeSizeFunction {
    const NAME: &'static str = "NativeSizeFunction";

    const CALLABLE: bool = true;

    type Mutable = Readable;

    fn prototype(ctx: &Ctx<'js>) -> Result<Option<Object<'js>>> {
        Ok(Some(Function::prototype(ctx.clone())))
    }

    fn constructor(_ctx: &Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        Ok(None)
    }

    fn call<'a>(this: &JsCell<'js, Self>, params: Params<'a, 'js>) -> Result<Value<'js>> {
        match &*this.borrow() {
            NativeSizeFunction::Count => Ok(Value::new_int(params.ctx().clone(), 1)),
            NativeSizeFunction::ByteLength => {
                let Some(chunk) = params.arg(0) else {
                    return Err(Exception::throw_type(
                        params.ctx(),
                        "ByteLengthQueuingStrategy expects an argument 'chunk'",
                    ));
                };

                byte_length_queueing_strategy_size_function(params.ctx(), &chunk)
            },
        }
    }
}

fn byte_length_queueing_strategy_size_function<'js>(
    ctx: &Ctx<'js>,
    chunk: &Value<'js>,
) -> Result<Value<'js>> {
    if let Some(chunk) = chunk.as_object() {
        chunk.get("byteLength")
    } else {
        Err(Exception::throw_type(
            ctx,
            "ByteLengthQueuingStrategy argument 'chunk' must be an object",
        ))
    }
}
