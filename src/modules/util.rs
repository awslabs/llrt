
use rquickjs::{
    cstr,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Function, Result,
};

use crate::{module_builder::ModuleInfo, modules::module::export_default};

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare_static(cstr!("default"))?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;

            Ok(())
        })
    }
}

impl Into<ModuleInfo<UtilModule>> for UtilModule {
    fn into(self) -> ModuleInfo<UtilModule> {
        ModuleInfo {
            name: "util",
            module: self,
        }
    }
}