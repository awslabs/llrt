use rquickjs::{loader::Loader, Ctx, Error, Module, Result};

#[derive(Debug)]
pub struct JSONLoader;

impl Loader for JSONLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, path: &str) -> Result<Module<'js>> {
        if !path.ends_with(".json") {
            return Err(Error::new_loading(path));
        }
        let source = std::fs::read_to_string(path)?;
        Module::declare(ctx.clone(), path, ["export default ", &source].concat())
    }
}
