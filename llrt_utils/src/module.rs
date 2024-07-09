use rquickjs::{module::Exports, Ctx, Object, Result, Value};

pub fn export_default<'js, F>(ctx: &Ctx<'js>, exports: &Exports<'js>, f: F) -> Result<()>
where
    F: FnOnce(&Object<'js>) -> Result<()>,
{
    let default = Object::new(ctx.clone())?;
    f(&default)?;

    for name in default.keys::<String>() {
        let name = name?;
        let value: Value = default.get(&name)?;
        exports.export(name, value)?;
    }

    exports.export("default", default)?;

    Ok(())
}
