// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, fs::File, io::Read};

use llrt_json::escape::escape_json_string;
use llrt_utils::result::ResultExt;
use rquickjs::{loader::Loader, Ctx, Function, Module, Object, Result, Value};
use tracing::trace;

use crate::modules::path;
use crate::{module::ModuleCache, CJS_IMPORT_PREFIX, CJS_LOADER_PREFIX};

#[derive(Debug, Default)]
pub struct PackageLoader;

impl PackageLoader {
    fn load_cjs_module<'js>(name: &str, ctx: Ctx<'js>) -> Result<Module<'js>> {
        let cjs_specifier = [CJS_IMPORT_PREFIX, name].concat();
        let require: Function = ctx.globals().get("require")?;
        let export_object: Value = require.call((&cjs_specifier,))?;
        let mut module = String::with_capacity(name.len() + 512);
        module.push_str("const value = require(\"");

        module.push_str(name);
        module.push_str("\");export default value.default||value;");
        if let Some(obj) = export_object.as_object() {
            let keys: Result<Vec<String>> = obj.keys().collect();
            let keys = keys?;

            if !keys.is_empty() {
                module.push_str("const{");

                for p in keys.iter() {
                    if p == "default" {
                        continue;
                    }
                    module.push_str(p);
                    module.push(',');
                }
                module.truncate(module.len() - 1);
                module.push_str("}=value;");
                module.push_str("export{");
                for p in keys.iter() {
                    if p == "default" {
                        continue;
                    }
                    module.push_str(p);
                    module.push(',');
                }
                module.truncate(module.len() - 1);
                module.push_str("};");
            }
        }
        Module::declare(ctx, name, module)
    }

    fn normalize_name(name: &str) -> (bool, bool, &str, &str) {
        if !name.starts_with("__") {
            // If name doesn’t start with "__", return defaults
            return (false, false, name, name);
        }

        if let Some(cjs_path) = name.strip_prefix(CJS_IMPORT_PREFIX) {
            // If it starts with CJS_IMPORT_PREFIX, mark as from_cjs_import
            return (true, false, name, cjs_path);
        }

        if let Some(cjs_path) = name.strip_prefix(CJS_LOADER_PREFIX) {
            // If it starts with CJS_LOADER_PREFIX, mark as is_cjs
            return (false, true, cjs_path, cjs_path);
        }

        // Default return if no prefixes match
        (false, false, name, name)
    }

    fn load_module<'js>(ctx: &Ctx<'js>, name: &str) -> Result<(Module<'js>, Option<String>)> {
        let ctx = ctx.clone();

        let (from_cjs_import, is_cjs, normalized_name, path) = Self::normalize_name(name);

        //json files can never be from CJS imports as they are handled by require
        if !from_cjs_import {
            if normalized_name.ends_with(".json") {
                let mut file = File::open(path)?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                let mut json = String::with_capacity(contents.len() + 40);
                json.push_str("export default JSON.parse(\"");
                escape_json_string(&mut json, &contents);
                json.push_str("\");");
                trace!("+- Loading module as json: {}\n", normalized_name);
                return Ok((Module::declare(ctx, path, json)?, None));
            }
            if is_cjs || normalized_name.ends_with(".cjs") {
                let url = ["file://", path].concat();
                trace!("+- Loading cjs module: {}\n", normalized_name);
                return Ok((Self::load_cjs_module(path, ctx)?, Some(url)));
            }
        }

        let path = path::resolve_path([path].iter())?;
        let path = path.as_str();

        let binding = ctx.userdata::<RefCell<ModuleCache>>().or_throw(&ctx)?;

        let module = if let Some(module) = {
            let cache = binding.borrow();
            cache.esm.get(path).cloned()
        } {
            trace!("+- Loading esm module via cache: {}\n", path);
            (module, None)
        } else {
            let bytes = std::fs::read(path)?;
            let mut bytes: &[u8] = &bytes;

            if !from_cjs_import && bytes.starts_with(b"#!") {
                bytes = bytes.splitn(2, |&c| c == b'\n').nth(1).unwrap_or(bytes);
            }

            let module = Module::declare(ctx.clone(), normalized_name, bytes)?;
            {
                let mut cache = binding.borrow_mut();
                cache.esm.insert(path.into(), module.clone());
            }

            trace!("+- Loading esm module: {}\n", path);
            (module, Some(["file://", path].concat()))
        };

        Ok(module)
    }
}

impl Loader for PackageLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js>> {
        trace!("Try load '{}'", name);
        let (module, url) = Self::load_module(ctx, name)?;
        if let Some(url) = url {
            let meta: Object = module.meta()?;
            meta.prop("url", url)?;
        }

        Ok(module)
    }
}
