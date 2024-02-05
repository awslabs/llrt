use rquickjs::{
    atom::PredefinedAtom, class::JsClass, prelude::This, Array, Ctx, Function, Object, Result,
    Value,
};

use super::{object::ObjectExt, result::OptionExt};

pub trait IteratorDef<'js>
where
    Self: 'js + JsClass<'js> + Sized,
{
    fn js_entries(&self, ctx: Ctx<'js>) -> Result<Array<'js>>;

    fn js_iterator(&self, ctx: Ctx<'js>) -> Result<Value<'js>> {
        let value = self.js_entries(ctx)?;
        let obj = value.as_object();
        let values_fn: Function = obj.get(PredefinedAtom::Values)?;
        values_fn.call((This(value),))
    }
}

pub fn get_class_name(value: &Value) -> Result<Option<String>> {
    value
        .get_optional::<_, Object>(PredefinedAtom::Constructor)?
        .and_then_ok(|ctor| ctor.get_optional::<_, String>(PredefinedAtom::Name))
}
