use rquickjs::{module::{Declarations, Exports, ModuleDef}, Ctx, Result};
pub trait GenericModule {
    fn declare(&self, declare: &mut Declarations) -> Result<()>;
    fn evaluate<'js>(&self, ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()>;
}

// Implement this trait for any type that implements ModuleDef
impl<T: ModuleDef + 'static> GenericModule for T {
    fn declare(&self, declare: &mut Declarations) -> Result<()> {
        T::declare(declare)
    }

    fn evaluate<'js>(&self, ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        T::evaluate(ctx, exports)
    }
}