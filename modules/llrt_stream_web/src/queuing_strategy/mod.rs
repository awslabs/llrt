use llrt_utils::primordials::Primordial;
use rquickjs::{
    class::Trace, convert::Coerced, prelude::This, Ctx, Error, Exception, FromJs, Function,
    JsLifetime, Result, Value,
};

pub(crate) use byte_length::ByteLengthQueuingStrategy;
pub(crate) use count::CountQueuingStrategy;

use crate::utils::ValueOrUndefined;

mod byte_length;
mod count;
#[cfg(test)]
mod tests;

/// QueuingStrategy is the structure of a user-provided object describing how backpressure should be signalled.
/// https://streams.spec.whatwg.org/#qs-api
pub(super) struct QueuingStrategy<'js> {
    // unrestricted double highWaterMark;
    high_water_mark: Option<f64>,
    // callback QueuingStrategySize = unrestricted double (any chunk);
    pub(super) size: Option<SizeFunction<'js>>,
}

impl<'js> FromJs<'js> for QueuingStrategy<'js> {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or_else(|| Error::new_from_js(ty_name, "Object"))?;

        let high_water_mark = obj
            .get_value_or_undefined::<_, Coerced<f64>>("highWaterMark")?
            .map(|value| value.0);
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
        this: Option<Self>,
        default_hwm: f64,
    ) -> Result<f64> {
        match this {
            // If strategy["highWaterMark"] does not exist, return defaultHWM.
            None => Ok(default_hwm),
            // Let highWaterMark be strategy["highWaterMark"].
            Some(Self {
                high_water_mark: Some(high_water_mark),
                ..
            }) => {
                // If highWaterMark is NaN or highWaterMark < 0, throw a RangeError exception.
                if high_water_mark.is_nan() || high_water_mark < 0.0 {
                    Err(Exception::throw_range(ctx, "Invalid highWaterMark"))
                } else {
                    // Return highWaterMark.
                    Ok(high_water_mark)
                }
            },

            // If strategy["highWaterMark"] does not exist, return defaultHWM.
            _ => Ok(default_hwm),
        }
    }

    // https://streams.spec.whatwg.org/#make-size-algorithm-from-size-function
    pub(super) fn extract_size_algorithm(this: Option<&Self>) -> SizeAlgorithm<'js> {
        // If strategy["size"] does not exist, return an algorithm that returns 1.
        match this.as_ref().and_then(|t| t.size.as_ref()) {
            None => SizeAlgorithm::AlwaysOne,
            Some(size) => SizeAlgorithm::SizeFunction(size.clone()),
        }
    }
}

/// SizeAlgorithm represents the two ways we might generate sizes - by calling a function or by simply returning 1.0 (the default)
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
            Self::SizeFunction(SizeFunction::Js(ref f)) => {
                f.call((This(Value::new_undefined(ctx.clone())), chunk.clone()))
            },
            Self::SizeFunction(SizeFunction::Native(NativeSizeFunction::ByteLength)) => {
                let size = byte_length_queueing_strategy_size_function(&ctx, &chunk)?;
                SizeValue::from_js(&ctx, size)
            },
        }
    }
}

/// SizeValue abstracts over the sources of size values - they can either come from user-provided size functions, in which case they might
/// be any Value, or (more often) they come from a NativeSizeFunction or the default AlwaysOne algorithm and we can pass around a native Rust type.
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

/// SizeFunction abstracts over user-provided size functions (from their own queuing strategy implementations) and ones provided by us.
/// We want to be able to recognise the ones that we have provided so we can short-circuit expensive JS calls
#[derive(JsLifetime, Trace, Clone)]
pub(super) enum SizeFunction<'js> {
    Js(Function<'js>),
    Native(NativeSizeFunction),
}

impl<'js> FromJs<'js> for SizeFunction<'js> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let primordials = NativeSizeFunctionPrimordials::get(ctx)?;
        if value == primordials.count.clone().into_value() {
            return Ok(SizeFunction::Native(NativeSizeFunction::Count));
        }
        if value == primordials.byte_length.clone().into_value() {
            return Ok(SizeFunction::Native(NativeSizeFunction::ByteLength));
        }

        Ok(SizeFunction::Js(Function::from_js(ctx, value)?))
    }
}

/// QueueingStrategyInit is the dictionary of input parameters for both native queuing strategies
/// https://streams.spec.whatwg.org/#dictdef-queuingstrategyinit
pub(crate) struct QueueingStrategyInit {
    high_water_mark: f64,
}

impl<'js> FromJs<'js> for QueueingStrategyInit {
    fn from_js(_ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let ty_name = value.type_name();
        let obj = value
            .as_object()
            .ok_or_else(|| Error::new_from_js(ty_name, "Object"))?;

        let high_water_mark = obj
            .get_value_or_undefined::<_, Coerced<f64>>("highWaterMark")?
            .ok_or_else(|| Error::new_from_js(ty_name, "QueueingStrategyInit"))?;

        Ok(Self {
            high_water_mark: high_water_mark.0,
        })
    }
}

/// Identifies a built-in queuing-strategy size algorithm so it can be handled
/// without calling the JavaScript function.
#[derive(JsLifetime, Trace, Clone, Copy)]
pub(super) enum NativeSizeFunction {
    ByteLength,
    Count,
}

#[derive(Clone, JsLifetime, Trace)]
pub(super) struct NativeSizeFunctionPrimordials<'js> {
    pub(super) count: Function<'js>,
    pub(super) byte_length: Function<'js>,
}

impl<'js> Primordial<'js> for NativeSizeFunctionPrimordials<'js> {
    fn new(ctx: &Ctx<'js>) -> Result<Self> {
        let count: Function = Function::new(ctx.clone(), || 1)?;
        let byte_length: Function = Function::new(ctx.clone(), native_byte_length_size)?;
        count.set_name("size")?;
        byte_length.set_name("size")?;

        Ok(Self { count, byte_length })
    }
}

fn byte_length_queueing_strategy_size_function<'js>(
    ctx: &Ctx<'js>,
    chunk: &Value<'js>,
) -> Result<Value<'js>> {
    if chunk.is_null() || chunk.is_undefined() {
        return Err(Exception::throw_type(
            ctx,
            "ByteLengthQueuingStrategy argument 'chunk' must be an object",
        ));
    }

    if let Some(chunk) = chunk.as_object() {
        chunk.get("byteLength")
    } else {
        Ok(Value::new_undefined(ctx.clone()))
    }
}

fn native_byte_length_size<'js>(chunk: Value<'js>) -> Result<Value<'js>> {
    let ctx = chunk.ctx().clone();
    byte_length_queueing_strategy_size_function(&ctx, &chunk)
}
