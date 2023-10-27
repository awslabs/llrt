use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result, Value,
};

use crate::util::export_default;

pub struct ModuleModule;

fn create_require(ctx: Ctx<'_>) -> Result<Value<'_>> {
    ctx.globals().get("require")
}

impl ModuleDef for ModuleModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("createRequire")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("createRequire", Func::from(create_require))?;

            Ok(())
        })?;

        Ok(())
    }
}
