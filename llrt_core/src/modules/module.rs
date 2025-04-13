// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs,
    rc::Rc,
    sync::Mutex,
};

use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    object::Accessor,
    prelude::Func,
    qjs, Ctx, Error, Filter, JsLifetime, Module, Object, Result, Value,
};
use tokio::time::Instant;
use tracing::trace;

use crate::bytecode::BYTECODE_FILE_EXT;
use crate::libs::{
    json::parse::json_parse,
    utils::module::{export_default, ModuleInfo},
};
use crate::modules::{
    path::resolve_path,
    require::{resolver::require_resolve, CJS_IMPORT_PREFIX},
    timers::poll_timers,
};
use crate::utils::ctx::CtxExt;

#[derive(Default)]
pub struct RequireState<'js> {
    cache: HashMap<Rc<str>, Value<'js>>,
    exports: HashMap<Rc<str>, Value<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for RequireState<'js> {
    type Changed<'to> = RequireState<'to>;
}

pub struct ModuleModule;

fn create_require(ctx: Ctx<'_>) -> Result<Value<'_>> {
    ctx.globals().get("require")
}

impl ModuleDef for ModuleModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("createRequire")?;
        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("createRequire", Func::from(create_require))?;

            Ok(())
        })?;

        Ok(())
    }
}

impl From<ModuleModule> for ModuleInfo<ModuleModule> {
    fn from(val: ModuleModule) -> Self {
        ModuleInfo {
            name: "module",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx, module_names: HashSet<&'static str>) -> Result<()> {
    let globals = ctx.globals();

    let require_in_progress: Rc<Mutex<HashMap<Rc<str>, Object>>> =
        Rc::new(Mutex::new(HashMap::new()));

    let _ = ctx.store_userdata(RefCell::new(RequireState::default()));

    let exports_accessor = Accessor::new(
        |ctx| {
            struct Args<'js>(Ctx<'js>);
            let Args(ctx) = Args(ctx);
            let name = ctx.get_script_or_module_name()?;
            let name = name.trim_start_matches(CJS_IMPORT_PREFIX);

            let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
            let mut state = binding.borrow_mut();

            if let Some(value) = state.exports.get(name) {
                Ok::<_, Error>(value.clone())
            } else {
                let obj = Object::new(ctx.clone())?.into_value();
                state.exports.insert(name.into(), obj.clone());
                Ok::<_, Error>(obj)
            }
        },
        |ctx, exports| {
            struct Args<'js>(Ctx<'js>, Value<'js>);
            let Args(ctx, exports) = Args(ctx, exports);
            let name = ctx.get_script_or_module_name()?;
            let name = name.trim_start_matches(CJS_IMPORT_PREFIX);
            let binding = ctx.userdata::<RefCell<RequireState>>().unwrap();
            let mut state = binding.borrow_mut();
            state.exports.insert(name.into(), exports);
            Ok::<_, Error>(())
        },
    )
    .configurable()
    .enumerable();

    let module = Object::new(ctx.clone())?;
    module.prop("exports", exports_accessor)?;

    globals.prop("module", module)?;
    globals.prop("exports", exports_accessor)?;

    globals.set(
        "require",
        Func::from(move |ctx, specifier: String| -> Result<Value> {
            struct Args<'js>(Ctx<'js>);
            let Args(ctx) = Args(ctx);

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

                if module_names.contains(specifier.as_str()) {
                    import_name = specifier.into();
                    import_name.clone()
                } else {
                    let module_name = ctx.get_script_or_module_name()?;
                    let module_name = module_name.trim_start_matches(CJS_IMPORT_PREFIX);
                    let abs_path = resolve_path([module_name].iter())?;

                    let resolved_path =
                        require_resolve(&ctx, &specifier, &abs_path, false)?.into_owned();
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

            let mut require_in_progress_map = require_in_progress.lock().unwrap();
            if let Some(obj) = require_in_progress_map.get(&import_name) {
                let value = obj.clone().into_value();
                return Ok(value);
            }

            trace!("Require: {}", import_specifier);

            let obj = Object::new(ctx.clone())?;
            require_in_progress_map.insert(import_name.clone(), obj.clone());
            drop(require_in_progress_map);
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

            require_in_progress
                .lock()
                .unwrap()
                .remove(import_name.as_ref());

            if let Some(exports_obj) = exports_obj {
                if exports_obj.type_of() == rquickjs::Type::Object {
                    drop(state);
                    let exports = unsafe { exports_obj.as_object().unwrap_unchecked() };

                    for prop in
                        exports.own_props::<Value, Value>(Filter::new().private().string().symbol())
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
        }),
    )?;

    Ok(())
}
