use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result,
};
use uuid::Uuid;

use crate::util::export_default;

pub struct UuidModule;

impl ModuleDef for UuidModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("v4")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("v4", Func::from(|| Uuid::new_v4().to_string()))?;
            Ok(())
        })
    }
}
