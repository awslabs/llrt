// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use llrt_utils::object::ObjectExt;
use llrt_utils::result::ResultExt;
use rquickjs::{
    loader::Loader,
    module::ModuleDef,
    prelude::{Func, Opt},
    Ctx, Error, Module, Object, Result, Value,
};
use tracing::trace;

use super::{Hook, ModuleHookState};

type LoadFn = for<'js> fn(Ctx<'js>, Vec<u8>) -> Result<Module<'js>>;

#[derive(Debug, Default)]
pub struct ModuleLoader {
    modules: HashMap<String, LoadFn>,
}

impl ModuleLoader {
    fn load_func<'js, D: ModuleDef>(ctx: Ctx<'js>, name: Vec<u8>) -> Result<Module<'js>> {
        Module::declare_def::<D, _>(ctx, name)
    }

    pub fn add_module<N: Into<String>, M: ModuleDef>(&mut self, name: N, _module: M) -> &mut Self {
        self.modules.insert(name.into(), Self::load_func::<M>);
        self
    }

    #[must_use]
    pub fn with_module<N: Into<String>, M: ModuleDef>(mut self, name: N, module: M) -> Self {
        self.add_module(name, module);
        self
    }
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js>> {
        trace!("Try load '{}'", name);
        let (short_circuit, next_load, source) = module_hook_load(ctx, name)?;

        if short_circuit {
            trace!("+- Loading module via ShortCircuit: {}\n", name);
            return Module::declare(ctx.clone(), name, source);
        };

        let load = self
            .modules
            .remove(name)
            .ok_or_else(|| Error::new_loading(name))?;

        if next_load {
            trace!("|  Determined as `nextResolve`: {}", name);
        } else {
            trace!("|  Determined as `NormalCircuit`: {}", name);
        }

        trace!("+- Loading module: {}\n", name);
        (load)(ctx.clone(), Vec::from(name))
    }
}

pub fn module_hook_load<'js>(ctx: &Ctx<'js>, name: &str) -> Result<(bool, bool, String)> {
    let bind_state = ctx.userdata::<RefCell<ModuleHookState>>().or_throw(ctx)?;
    let state = bind_state.borrow();

    if state.hooks.is_empty() {
        return Ok((false, false, name.into()));
    }

    let result = call_load_hooks(ctx, &state.hooks, 0, name.into())?;

    let short_circuit = result
        .get_optional::<_, bool>("shortCircuit")?
        .unwrap_or(false);

    let next_load = result
        .get_optional::<_, bool>("__nextLoad")?
        .unwrap_or(false);

    let source = result
        .get_optional::<_, String>("source")?
        .unwrap_or_default();

    Ok((short_circuit, next_load, source))
}

#[allow(dependency_on_unit_never_type_fallback)]
fn call_load_hooks<'js>(
    ctx: &Ctx<'js>,
    hooks: &[Hook<'js>],
    index: usize,
    x: String,
) -> Result<Object<'js>> {
    if index >= hooks.len() {
        let obj = Object::new(ctx.clone())?;
        obj.set("url", x)?;
        obj.set("shortCircuit", false)?;
        obj.set("__nextLoad", false)?;
        return Ok(obj);
    }

    let hook = &hooks[index];
    let context = Object::new(ctx.clone())?;

    let called_next = Rc::new(Cell::new(false));
    let called_next_ref = Rc::clone(&called_next);

    let next_func = {
        let hooks = hooks.to_vec();
        Func::new(
            move |ctx: Ctx<'js>, spec: String, _opt_ctx: Opt<Value<'js>>| {
                called_next_ref.set(true);
                call_load_hooks(&ctx, &hooks, index + 1, spec)
            },
        )
    };

    let Some(load_fn) = &hook.load else {
        return call_load_hooks(ctx, hooks, index + 1, x);
    };

    let result = load_fn.call::<_, Object>((x.clone(), context, next_func))?;
    result.set("__nextLoad", called_next.get())?;

    Ok(result)
}
