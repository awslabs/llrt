use rquickjs::{
    class::{Trace, Tracer},
    Ctx, FromJs, IntoJs, JsLifetime, Result, Type, Value,
};

/// Helper type for treating an undefined value as None, without treating null as None
#[derive(Clone)]
pub struct Undefined<T>(pub Option<T>);

impl<'js, T: FromJs<'js>> FromJs<'js> for Undefined<T> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.type_of() == Type::Undefined {
            Ok(Self(None))
        } else {
            Ok(Self(Some(FromJs::from_js(ctx, value)?)))
        }
    }
}

impl<'js, T: IntoJs<'js>> IntoJs<'js> for Undefined<T> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self.0 {
            None => Ok(Value::new_undefined(ctx.clone())),
            Some(val) => val.into_js(ctx),
        }
    }
}

impl<T> Default for Undefined<T> {
    fn default() -> Self {
        Self(None)
    }
}

unsafe impl<'js, T: JsLifetime<'js>> JsLifetime<'js> for Undefined<T> {
    type Changed<'to> = Undefined<T::Changed<'to>>;
}

impl<'js, T: Trace<'js>> Trace<'js> for Undefined<T> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.0.trace(tracer)
    }
}

/// Helper type for converting an None into null instead of undefined.
/// Needed while rquickjs::function::Null has no IntoJs implementation
#[derive(Clone)]
pub struct Null<T>(pub Option<T>);

impl<'js, T: FromJs<'js>> FromJs<'js> for Null<T> {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        if value.type_of() == Type::Null {
            Ok(Self(None))
        } else {
            Ok(Self(Some(FromJs::from_js(ctx, value)?)))
        }
    }
}

impl<'js, T: IntoJs<'js>> IntoJs<'js> for Null<T> {
    fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        match self.0 {
            None => Ok(Value::new_null(ctx.clone())),
            Some(val) => val.into_js(ctx),
        }
    }
}

unsafe impl<'js, T: JsLifetime<'js>> JsLifetime<'js> for Null<T> {
    type Changed<'to> = Null<T::Changed<'to>>;
}

impl<'js, T: Trace<'js>> Trace<'js> for Null<T> {
    fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
        self.0.trace(tracer)
    }
}
