use rquickjs::function::{Func, Rest};
use rquickjs::{
    cstr,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Function, Result, Value,
};
use std::sync::atomic::Ordering;

use crate::modules::console::{format_values_internal, AWS_LAMBDA_MODE, NEWLINE_LOOKUP};
use crate::{module_builder::ModuleInfo, modules::module::export_default};

pub struct UtilModule;

impl ModuleDef for UtilModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare(stringify!(TextDecoder))?;
        declare.declare(stringify!(TextEncoder))?;
        declare.declare(stringify!(format))?;
        declare.declare_c_str(cstr!("default"))?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let encoder: Function = globals.get(stringify!(TextEncoder))?;
            let decoder: Function = globals.get(stringify!(TextDecoder))?;

            default.set(stringify!(TextEncoder), encoder)?;
            default.set(stringify!(TextDecoder), decoder)?;
            default.set("format", Func::from(format))?;

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

fn format<'js>(ctx: Ctx<'js>, args: Rest<Value<'js>>) -> Result<String> {
    let mut result = String::with_capacity(64);
    let newline_char = NEWLINE_LOOKUP[AWS_LAMBDA_MODE.load(Ordering::Relaxed) as usize];
    format_values_internal(&mut result, &ctx, args, false, newline_char)?;

    Ok(result)
}
