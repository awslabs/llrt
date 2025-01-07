use llrt_utils::option::Undefined;
use rquickjs::{
    class::{JsClass, OwnedBorrowMut},
    Class, Ctx, FromJs, IntoAtom, Object, Result, Value,
};

pub mod promise;
pub mod queue;

// the trait used elsewhere in this repo accepts null values as 'None', which causes many web platform tests to fail as they
// like to check that undefined is accepted and null isn't.
pub trait ValueOrUndefined<'js> {
    fn get_value_or_undefined<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(
        &self,
        k: K,
    ) -> Result<Option<V>>;
}

impl<'js> ValueOrUndefined<'js> for Object<'js> {
    fn get_value_or_undefined<K: IntoAtom<'js> + Clone, V: FromJs<'js> + Sized>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        let value = self.get::<K, Value<'js>>(k)?;
        Ok(Undefined::from_js(self.ctx(), value)?.0)
    }
}

impl<'js> ValueOrUndefined<'js> for Value<'js> {
    fn get_value_or_undefined<K: IntoAtom<'js> + Clone, V: FromJs<'js>>(
        &self,
        k: K,
    ) -> Result<Option<V>> {
        if let Some(obj) = self.as_object() {
            return obj.get_value_or_undefined(k);
        }
        Ok(None)
    }
}

pub trait UnwrapOrUndefined<'js> {
    fn unwrap_or_undefined(self, ctx: &Ctx<'js>) -> Value<'js>;
}

impl<'js> UnwrapOrUndefined<'js> for Option<Value<'js>> {
    fn unwrap_or_undefined(self, ctx: &Ctx<'js>) -> Value<'js> {
        self.unwrap_or_else(|| Value::new_undefined(ctx.clone()))
    }
}

pub fn class_from_owned_borrow_mut<'js, T: JsClass<'js>>(
    borrow: OwnedBorrowMut<'js, T>,
) -> (Class<'js, T>, OwnedBorrowMut<'js, T>) {
    let class = borrow.into_inner();
    let borrow = OwnedBorrowMut::from_class(class.clone());
    (class, borrow)
}
