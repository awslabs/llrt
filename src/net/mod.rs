mod socket;

use rquickjs::{
    cstr,
    module::{Declarations, Exports, ModuleDef},
    Ctx, Result,
};

pub struct NetModule;

impl ModuleDef for NetModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        socket::declare(declare)?;
        declare.declare_static(cstr!("default"))?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        socket::init(ctx.clone(), exports)?;
        Ok(())
    }
}
