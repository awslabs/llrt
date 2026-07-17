// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use core::fmt;
use std::fmt::Debug;

use llrt_utils::{
    option::Undefined,
    primordials::{BasePrimordials, Primordial},
};
use rquickjs::{
    atom::PredefinedAtom,
    class::{
        impl_::{CloneTrait, CloneWrapper},
        JsClass, Trace,
    },
    function::{Constructor, Opt},
    prelude::This,
    qjs, Class, Coerced, Ctx, Error, Exception, FromJs, IntoJs, JsLifetime, Object, Result, Value,
};

#[derive(Trace, JsLifetime, Debug)]
pub struct QuotaExceededError {
    message: String,
    stack: String,
    requested: Option<u64>,
    quota: Option<u64>,
}

impl fmt::Display for QuotaExceededError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuotaExceededError")
            .field("name", &self.name())
            .field("message", &self.message())
            .finish()
    }
}

impl<'js> JsClass<'js> for QuotaExceededError {
    const NAME: &'static str = "QuotaExceededError";
    type Mutable = rquickjs::class::Writable;
    fn prototype(ctx: &Ctx<'js>) -> rquickjs::Result<Option<Object<'js>>> {
        use rquickjs::class::impl_::{MethodImpl, MethodImplementor};
        let proto = Object::new(ctx.clone())?;
        let implementor = MethodImpl::<Self>::new();
        implementor.implement(&proto)?;

        Ok(Some(proto))
    }
    fn constructor(ctx: &Ctx<'js>) -> Result<Option<Constructor<'js>>> {
        use rquickjs::class::impl_::{ConstructorCreate, ConstructorCreator};
        let implementor = ConstructorCreate::<Self>::new();
        let constructor = implementor
            .create_constructor(ctx)?
            .expect("QuotaExceededError must have a constructor");

        Ok(Some(constructor))
    }
}

impl<'js> IntoJs<'js> for QuotaExceededError {
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> Result<Value<'js>> {
        let cls = Class::<Self>::instance(ctx.clone(), self)?;
        IntoJs::into_js(cls, ctx)
    }
}

impl<'js> FromJs<'js> for QuotaExceededError
where
    for<'a> CloneWrapper<'a, Self>: CloneTrait<Self>,
{
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
        let value = Class::<Self>::from_js(ctx, value)?;
        let borrow = value.try_borrow()?;
        Ok(CloneWrapper(&*borrow).wrap_clone())
    }
}

#[rquickjs::methods]
impl QuotaExceededError {
    #[qjs(constructor)]
    pub fn new<'js>(
        ctx: Ctx<'js>,
        this: This<Value<'js>>,
        message: Opt<Undefined<Coerced<String>>>,
    ) -> Result<Self> {
        // When called with `new`, rquickjs passes the constructor function
        // as `this`. Without `new` this is undefined or the global object.
        if this.0.as_function().is_none() {
            return Err(Exception::throw_type(
                &ctx,
                "Cannot call the QuotaExceededError constructor without 'new'",
            ));
        }

        let message = match message.0 {
            Some(Undefined(Some(v))) => v.0,
            _ => String::new(),
        };

        let error: Object = BasePrimordials::get(&ctx)?
            .constructor_error
            .construct((message.clone(),))?;

        Ok(Self {
            message,
            stack: error.get(PredefinedAtom::Stack)?,
            requested: None,
            quota: None,
        })
    }

    #[qjs(get)]
    fn name(&self) -> &'static str {
        "QuotaExceededError"
    }

    #[qjs(get)]
    fn code(&self) -> u8 {
        22
    }

    #[qjs(get)]
    fn message(&self) -> &str {
        &self.message
    }

    #[qjs(get)]
    fn requested(&self) -> Option<u64> {
        self.requested
    }

    #[qjs(get)]
    fn quota(&self) -> Option<u64> {
        self.quota
    }

    #[qjs(prop, rename = PredefinedAtom::SymbolToStringTag, configurable)]
    pub fn to_string_tag() -> &'static str {
        stringify!(QuotaExceededError)
    }
}

impl QuotaExceededError {
    pub fn quota_exceeded_error(ctx: &Ctx<'_>, message: &str) -> Error {
        let ctor: Constructor = ctx
            .globals()
            .get("QuotaExceededError")
            .expect("failed to create QuotaExceededError");
        let value: Value = ctor
            .construct((message,))
            .expect("failed to create QuotaExceededError");
        unsafe {
            let dup = qjs::JS_DupValue(ctx.as_raw().as_ptr(), value.as_raw());
            qjs::JS_Throw(ctx.as_raw().as_ptr(), dup);
        }
        Error::Exception
    }
}
