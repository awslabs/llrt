// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    class::{Trace, Tracer},
    Ctx, FromJs, IntoJs, JsLifetime, Result, Value,
};

macro_rules! define_any {
    ($name:ident, $($variant:ident),+) => {
        #[derive(Debug, Clone)]
        pub enum $name<$($variant),+> {
            $(
                $variant($variant),
            )+
        }

        define_any_from_js!($name, $($variant),+);

        impl<'js, $($variant: IntoJs<'js>),+> IntoJs<'js> for $name<$($variant),+> {
            fn into_js(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
                match self {
                    $(
                        Self::$variant(val) => val.into_js(ctx),
                    )+
                }
            }
        }

        unsafe impl<'js, $($variant: JsLifetime<'js>),+> JsLifetime<'js> for $name<$($variant),+> {
            type Changed<'to> = $name<$($variant::Changed<'to>),+>;
        }

        impl<'js, $($variant: Trace<'js>),+> Trace<'js> for $name<$($variant),+> {
            fn trace<'a>(&self, tracer: Tracer<'a, 'js>) {
                match self {
                    $(
                        Self::$variant(val) => val.trace(tracer),
                    )+
                }
            }
        }

        define_any_methods!($name, $($variant),+);
    };
}

macro_rules! define_any_from_js {
    ($name:ident, $first:ident, $($rest:ident),+) => {
        impl<'js, $first: FromJs<'js>, $($rest: FromJs<'js>),+> FromJs<'js> for $name<$first, $($rest),+> {
            fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> Result<Self> {
                define_any_from_js_impl!($name, ctx, value, $first, $($rest),+)
            }
        }
    };
}

macro_rules! define_any_from_js_impl {
    ($name:ident, $ctx:ident, $value:ident, $first:ident) => {
        $first::from_js($ctx, $value).map($name::$first)
    };

    ($name:ident, $ctx:ident, $value:ident, $first:ident, $($rest:ident),+) => {
        $first::from_js($ctx, $value.clone()).map($name::$first).or_else(|error| {
            if error.is_from_js() {
                define_any_from_js_impl!($name, $ctx, $value, $($rest),+)
            } else {
                Err(error)
            }
        })
    };
}

macro_rules! define_any_variant_methods {
    ($variant:ident, $is_fn:ident, $as_fn:ident, $as_mut_fn:ident, $into_fn:ident) => {
        #[allow(dead_code)]
        pub fn $is_fn(&self) -> bool {
            matches!(self, Self::$variant(_))
        }

        #[allow(dead_code)]
        pub fn $as_fn(&self) -> Option<&$variant> {
            match self {
                Self::$variant(val) => Some(val),
                _ => None,
            }
        }

        #[allow(dead_code)]
        pub fn $as_mut_fn(&mut self) -> Option<&mut $variant> {
            match self {
                Self::$variant(val) => Some(val),
                _ => None,
            }
        }

        #[allow(dead_code)]
        pub fn $into_fn(self) -> std::result::Result<$variant, Self> {
            match self {
                Self::$variant(val) => Ok(val),
                other => Err(other),
            }
        }
    };

    (A) => {
        define_any_variant_methods!(A, is_a, as_a, as_a_mut, into_a);
    };
    (B) => {
        define_any_variant_methods!(B, is_b, as_b, as_b_mut, into_b);
    };
    (C) => {
        define_any_variant_methods!(C, is_c, as_c, as_c_mut, into_c);
    };
    (D) => {
        define_any_variant_methods!(D, is_d, as_d, as_d_mut, into_d);
    };
    (E) => {
        define_any_variant_methods!(E, is_e, as_e, as_e_mut, into_e);
    };
    (F) => {
        define_any_variant_methods!(F, is_f, as_f, as_f_mut, into_f);
    };
    (G) => {
        define_any_variant_methods!(G, is_g, as_g, as_g_mut, into_g);
    };
    (H) => {
        define_any_variant_methods!(H, is_h, as_h, as_h_mut, into_h);
    };
}

macro_rules! define_any_methods {
    ($name:ident, $($variant:ident),+) => {
        impl<$($variant),+> $name<$($variant),+> {
            $(
                define_any_variant_methods!($variant);
            )+
        }
    };
}

define_any!(Any2, A, B);
define_any!(Any3, A, B, C);
define_any!(Any4, A, B, C, D);
define_any!(Any5, A, B, C, D, E);
define_any!(Any6, A, B, C, D, E, F);
define_any!(Any7, A, B, C, D, E, F, G);
define_any!(Any8, A, B, C, D, E, F, G, H);

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context, Runtime};

    #[test]
    fn test_any2_string_number() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            // Test string conversion
            let val: Value = ctx.eval("'hello'").unwrap();
            let any: Any2<String, i32> = Any2::from_js(&ctx, val).unwrap();
            assert!(any.is_a());
            assert_eq!(any.as_a().unwrap(), "hello");
            assert!(!any.is_b());
            assert!(any.as_b().is_none());

            // Test number conversion
            let val: Value = ctx.eval("42").unwrap();
            let any: Any2<String, i32> = Any2::from_js(&ctx, val).unwrap();
            assert!(!any.is_a());
            assert!(any.is_b());
            assert_eq!(*any.as_b().unwrap(), 42);
        });
    }

    #[test]
    fn test_any3_fallback() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            // Test that it tries in order
            let val: Value = ctx.eval("true").unwrap();
            let any: Any3<String, i32, bool> = Any3::from_js(&ctx, val).unwrap();
            assert!(any.is_c());
            assert!(*any.as_c().unwrap());
            assert!(!any.is_a());
            assert!(!any.is_b());
        });
    }

    #[test]
    fn test_any2_into_js() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let any: Any2<String, i32> = Any2::A("test".to_string());
            let val: Value = any.into_js(&ctx).unwrap();
            let result: String = val.get().unwrap();
            assert_eq!(result, "test");

            let any: Any2<String, i32> = Any2::B(99);
            let val: Value = any.into_js(&ctx).unwrap();
            let result: i32 = val.get().unwrap();
            assert_eq!(result, 99);
        });
    }

    #[test]
    fn test_any3_methods() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            // Test all methods for variant A
            let val: Value = ctx.eval("'test'").unwrap();
            let any: Any3<String, i32, bool> = Any3::from_js(&ctx, val).unwrap();
            assert!(any.is_a());
            assert_eq!(any.as_a().unwrap(), "test");
            assert_eq!(any.into_a().unwrap(), "test");

            // Test all methods for variant B
            let val: Value = ctx.eval("42").unwrap();
            let any: Any3<String, i32, bool> = Any3::from_js(&ctx, val).unwrap();
            assert!(any.is_b());
            assert_eq!(*any.as_b().unwrap(), 42);
            assert_eq!(any.into_b().unwrap(), 42);

            // Test all methods for variant C
            let val: Value = ctx.eval("true").unwrap();
            let any: Any3<String, i32, bool> = Any3::from_js(&ctx, val).unwrap();
            assert!(any.is_c());
            assert!(*any.as_c().unwrap());
            assert!(any.into_c().unwrap());
        });
    }

    #[test]
    fn test_any4_mutable_methods() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            let val: Value = ctx.eval("42").unwrap();
            let mut any: Any4<String, i32, bool, f64> = Any4::from_js(&ctx, val).unwrap();

            if let Some(n) = any.as_b_mut() {
                *n = 100;
            }

            assert_eq!(any.into_b().unwrap(), 100);
        });
    }

    #[test]
    fn test_any2_error_propagation() {
        use rquickjs::{Array, Object};

        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            // Test that conversion errors cause fallback to next type
            let val: Value = ctx.eval("42").unwrap();
            let any: Any2<String, i32> = Any2::from_js(&ctx, val).unwrap();
            assert!(any.is_b());

            // Test that all types fail results in an error
            let val: Value = ctx.eval("null").unwrap();
            let result: Result<Any2<Object, Array>> = Any2::from_js(&ctx, val);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_any_conversion_order() {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            // Test that conversion happens in order A, B, C, D, E
            // Since 42 can be converted to f64, i32, etc., but String comes first and fails,
            // it should try the next successful conversion
            let val: Value = ctx.eval("42").unwrap();

            // String should fail, so it tries i32 which succeeds
            let any: Any3<String, i32, f64> = Any3::from_js(&ctx, val).unwrap();
            assert!(any.is_b());

            // If we flip the order, f64 would be tried first (but both work)
            let val: Value = ctx.eval("3.14").unwrap();
            let any: Any3<String, f64, i32> = Any3::from_js(&ctx, val).unwrap();
            assert!(any.is_b()); // f64 should succeed first
        });
    }
}
