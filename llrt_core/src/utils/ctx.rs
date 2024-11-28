use rquickjs::{Ctx, Result};

pub trait CtxExt {
    fn get_script_or_module_name(&self) -> Result<String>;
}

impl CtxExt for Ctx<'_> {
    fn get_script_or_module_name(&self) -> Result<String> {
        if let Some(name) = self.script_or_module_name(0) {
            name.to_string()
        } else {
            Ok(String::from("."))
        }
    }
}
