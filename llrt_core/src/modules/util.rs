use rquickjs::function::Func;
use rquickjs::{
    cstr,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Function, Result,
};

use crate::{module_builder::ModuleInfo, modules::module::export_default};

use super::console::format_plain;

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare(stringify!(format))?;
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
            default.set("format", Func::from(format_plain))?;

            Ok(())
        })
    }
}

impl From<UtilModule> for ModuleInfo<UtilModule> {
    fn from(val: UtilModule) -> Self {
        ModuleInfo {
            name: "util",
            module: val,
        }
    }
}
