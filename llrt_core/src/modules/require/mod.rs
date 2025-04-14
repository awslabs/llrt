// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    env, fs,
    marker::PhantomData,
    rc::Rc,
};

use once_cell::sync::Lazy;
use rquickjs::{atom::PredefinedAtom, qjs, Ctx, Filter, JsLifetime, Module, Object, Result, Value};
use tokio::time::Instant;
use tracing::trace;

use crate::bytecode::BYTECODE_FILE_EXT;
use crate::environment;
use crate::libs::json::parse::json_parse;
use crate::modules::{path::resolve_path, timers::poll_timers};
use crate::utils::ctx::CtxExt;

use self::resolver::require_resolve;

pub mod loader;
pub mod resolver;

// added when .cjs files are imported
pub const CJS_IMPORT_PREFIX: &str = "__cjs:";
// added to force CJS imports in loader
pub const CJS_LOADER_PREFIX: &str = "__cjsm:";

pub static LLRT_PLATFORM: Lazy<String> = Lazy::new(|| {
    env::var(environment::ENV_LLRT_PLATFORM)
        .ok()
        .filter(|platform| platform == "node")
        .unwrap_or_else(|| "browser".to_string())
});

pub static COMPRESSION_DICT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compression.dict"));

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));

#[derive(Default)]
pub struct RequireState<'js> {
    pub cache: HashMap<Rc<str>, Value<'js>>,
    pub exports: HashMap<Rc<str>, Value<'js>>,
    pub progress: HashMap<Rc<str>, Object<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for RequireState<'js> {
    type Changed<'to> = RequireState<'to>;
}

pub struct ModuleNames<'js> {
    pub list: HashSet<String>,
    _marker: PhantomData<&'js ()>,
}

unsafe impl<'js> JsLifetime<'js> for ModuleNames<'js> {
    type Changed<'to> = ModuleNames<'to>;
}

impl ModuleNames<'_> {
    pub fn new(names: HashSet<String>) -> Self {
        Self {
            list: names,
            _marker: PhantomData,
        }
    }
}

pub fn require(ctx: Ctx<'_>, specifier: String) -> Result<Value<'_>> {
    struct Args<'js>(Ctx<'js>);
    let Args(ctx) = Args(ctx);

    let binding = ctx.userdata::<RefCell<ModuleNames>>().unwrap();
    let module_names = binding.borrow();

    let is_cjs_import = specifier.starts_with(CJS_IMPORT_PREFIX);

    let import_name: Rc<str>;

    let is_json = specifier.ends_with(".json");

    trace!("Before specifier: {}", specifier);

    let import_specifier: Rc<str> = if !is_cjs_import {
        let is_bytecode = specifier.ends_with(BYTECODE_FILE_EXT);
        let is_bytecode_or_json = is_json || is_bytecode;
        let specifier = if is_bytecode_or_json {
            specifier
        } else {
            specifier.trim_start_matches("node:").to_string()
        };

        if module_names.list.contains(specifier.as_str()) {
            import_name = specifier.into();
            import_name.clone()
        } else {
            let module_name = ctx.get_script_or_module_name()?;
            let module_name = module_name.trim_start_matches(CJS_IMPORT_PREFIX);
            let abs_path = resolve_path([module_name].iter())?;

            let resolved_path = require_resolve(&ctx, &specifier, &abs_path, false)?.into_owned();
            import_name = resolved_path.into();
            if is_bytecode_or_json {
                import_name.clone()
            } else {
                [CJS_IMPORT_PREFIX, &import_name].concat().into()
            }
        }
    } else {
        import_name = specifier[CJS_IMPORT_PREFIX.len()..].into();
        specifier.into()
    };

    trace!("After specifier: {}", import_specifier);

    let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
    let mut state = binding.borrow_mut();

    if let Some(cached_value) = state.cache.get(import_name.as_ref()) {
        return Ok(cached_value.clone());
    }

    if is_json {
        let json = fs::read_to_string(import_name.as_ref())?;
        let json = json_parse(&ctx, json)?;
        state.cache.insert(import_name, json.clone());
        return Ok(json);
    }

    if let Some(obj) = state.progress.get(&import_name) {
        let value = obj.clone().into_value();
        return Ok(value);
    }

    trace!("Require: {}", import_specifier);

    let obj = Object::new(ctx.clone())?;
    state.progress.insert(import_name.clone(), obj.clone());
    drop(state);

    let import_promise = Module::import(&ctx, import_specifier.as_bytes())?;

    let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

    let mut deadline = Instant::now();

    let mut executing_timers = Vec::new();

    let imported_object = loop {
        if let Some(x) = import_promise.result::<Object>() {
            break x?;
        }

        if deadline < Instant::now() {
            poll_timers(rt, &mut executing_timers, None, Some(&mut deadline))?;
        }

        ctx.execute_pending_job();
    };

    let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
    let mut state = binding.borrow_mut();

    let exports_obj = state.exports.get(&import_name).cloned();

    state.progress.remove(import_name.as_ref());

    if let Some(exports_obj) = exports_obj {
        if exports_obj.type_of() == rquickjs::Type::Object {
            drop(state);
            let exports = unsafe { exports_obj.as_object().unwrap_unchecked() };

            for prop in exports.own_props::<Value, Value>(Filter::new().private().string().symbol())
            {
                let (key, value) = prop?;
                obj.set(key, value)?;
            }
        } else {
            //we have explicitly set it
            state.cache.insert(import_name, exports_obj.clone());
            return Ok(exports_obj);
        }
    } else {
        drop(state);
    }

    let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
    let mut state = binding.borrow_mut();

    let props = imported_object.props::<String, Value>();

    let default_export: Option<Value> = imported_object.get(PredefinedAtom::Default)?;
    if let Some(default_export) = default_export {
        //if default export is object attach all named exports to
        if let Some(default_object) = default_export.as_object() {
            for prop in props {
                let (key, value) = prop?;
                if !default_object.contains_key(&key)? {
                    default_object.set(key, value)?;
                }
            }
            let default_object = default_object.clone().into_value();
            state.cache.insert(import_name, default_object.clone());
            return Ok(default_object);
        }
    }

    for prop in props {
        let (key, value) = prop?;
        obj.set(key, value)?;
    }

    let value = obj.into_value();

    state.cache.insert(import_name, value.clone());
    Ok(value)
}
