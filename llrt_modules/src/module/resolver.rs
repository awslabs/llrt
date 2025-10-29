// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

use llrt_utils::{object::ObjectExt, result::ResultExt};
use rquickjs::{
    loader::Resolver,
    prelude::{Func, Opt},
    Ctx, Error, Object, Result, Value,
};
use tracing::trace;

use crate::CJS_IMPORT_PREFIX;

use super::{Hook, ModuleHookState};

#[derive(Debug, Default)]
pub struct ModuleResolver {
    modules: HashSet<String>,
}

impl ModuleResolver {
    #[must_use]
    pub fn add_name<P: Into<String>>(mut self, path: P) -> Self {
        self.modules.insert(path.into());
        self
    }
}

impl Resolver for ModuleResolver {
    fn resolve(&mut self, ctx: &Ctx<'_>, base: &str, name: &str) -> Result<String> {
        let name = name.trim_start_matches(CJS_IMPORT_PREFIX);
        let name = name.trim_start_matches("node:").trim_end_matches("/");

        let base = base.trim_start_matches(CJS_IMPORT_PREFIX);

        trace!("Try resolve '{}' from '{}'", name, base);

        let (short_circuit, next_resolve, x) = module_hook_resolve(ctx, name, base)?;

        if short_circuit {
            trace!("+- Resolved by `ShortCircuit`: {}", x);
            return Ok(x);
        }

        if next_resolve {
            trace!("|  Determined as `nextResolve`: {}", x);
        } else {
            trace!("|  Determined as `NormalCircuit`: {}", x);
        }

        if self.modules.contains(&x) {
            trace!("+- Resolved by `NativeModule`: {}", x);
            Ok(x)
        } else {
            Err(Error::new_resolving(base, x))
        }
    }
}

pub fn module_hook_resolve<'js>(ctx: &Ctx<'js>, x: &str, y: &str) -> Result<(bool, bool, String)> {
    trace!("|  module_hook_resolve(x, y):({}, {})", x, y);

    let bind_state = ctx.userdata::<RefCell<ModuleHookState>>().or_throw(ctx)?;
    let state = bind_state.borrow();

    if state.hooks.is_empty() {
        return Ok((false, false, x.into()));
    }

    let result = call_resolve_hooks(ctx, &state.hooks, 0, x.into(), y.into())?;

    let short_circuit = result
        .get_optional::<_, bool>("shortCircuit")?
        .unwrap_or(false);

    let next_resolve = result
        .get_optional::<_, bool>("__nextResolve")?
        .unwrap_or(false);

    let url = result.get::<_, String>("url")?;

    Ok((short_circuit, next_resolve, url))
}

#[allow(dependency_on_unit_never_type_fallback)]
fn call_resolve_hooks<'js>(
    ctx: &Ctx<'js>,
    hooks: &[Hook<'js>],
    index: usize,
    x: String,
    y: String,
) -> Result<Object<'js>> {
    if index >= hooks.len() {
        let obj = Object::new(ctx.clone())?;
        obj.set("url", x)?;
        obj.set("shortCircuit", false)?;
        obj.set("__nextResolve", false)?;
        return Ok(obj);
    }

    let hook = &hooks[index];
    let context = Object::new(ctx.clone())?;
    context.set("parentURL", y.clone())?;

    let called_next = Rc::new(Cell::new(false));
    let called_next_ref = Rc::clone(&called_next);

    let next_func = {
        let hooks = hooks.to_vec();
        let parent_url = y.clone();
        Func::new(
            move |ctx: Ctx<'js>, spec: String, opt_ctx: Opt<Value<'js>>| {
                called_next_ref.set(true);
                let parent_url = get_parent_url(&opt_ctx, &parent_url);
                call_resolve_hooks(&ctx, &hooks, index + 1, spec, parent_url)
            },
        )
    };

    let Some(resolve_fn) = &hook.resolve else {
        return call_resolve_hooks(ctx, hooks, index + 1, x, y);
    };

    let result = resolve_fn.call::<_, Object>((x.clone(), context, next_func))?;
    result.set("__nextResolve", called_next.get())?;

    Ok(result)
}

fn get_parent_url<'js>(opt_ctx: &Opt<Value<'js>>, default: &str) -> String {
    if let Some(val) = &opt_ctx.0 {
        if let Some(obj) = val.as_object() {
            if let Ok(url) = obj.get::<_, String>("parentURL") {
                return url;
            }
        }
    }
    default.into()
}
