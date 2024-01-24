// Shared with build.rs and compiler.rs

use rquickjs::{
    loader::{Loader, Resolver},
    module::ModuleData,
    Ctx,
};

pub struct DummyLoader;

impl Loader for DummyLoader {
    fn load(&mut self, _ctx: &Ctx<'_>, name: &str) -> rquickjs::Result<ModuleData> {
        Ok(ModuleData::source(name, ""))
    }
}

pub struct DummyResolver;

impl Resolver for DummyResolver {
    fn resolve(&mut self, _ctx: &Ctx<'_>, _base: &str, name: &str) -> rquickjs::Result<String> {
        Ok(name.into())
    }
}

pub fn human_file_size(size: usize) -> String {
    let fsize = size as f64;
    let i = if size == 0 {
        0
    } else {
        (fsize.log2() / 1024f64.log2()).floor() as i32
    };
    let size = fsize / 1024f64.powi(i);
    let units = ["B", "kB", "MB", "GB", "TB", "PB"];
    format!("{:.3} {}", size, units[i as usize])
}
