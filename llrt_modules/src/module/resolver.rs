// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet, rc::Rc};

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
    let hooks = Rc::new(bind_state.borrow().hooks.clone());

    if hooks.is_empty() {
        return Ok((false, false, x.into()));
    }

    let result = call_resolve_hooks(ctx, &hooks, x, y)?;

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
    hooks: &Rc<Vec<Hook<'js>>>,
    spec: &str,
    parent_url: &str,
) -> Result<Object<'js>> {
    call_resolve_hooks_from(ctx, hooks, 0, spec, parent_url)
}

fn call_resolve_hooks_from<'js>(
    ctx: &Ctx<'js>,
    hooks: &Rc<Vec<Hook<'js>>>,
    start_index: usize,
    spec: &str,
    parent_url: &str,
) -> Result<Object<'js>> {
    for index in start_index..hooks.len() {
        let Some(resolve_fn) = &hooks[index].resolve else {
            continue;
        };

        let context = Object::new(ctx.clone())?;
        context.set("parentURL", parent_url)?;

        let spec_clone = spec.to_string();
        let hooks_clone = Rc::clone(hooks);

        let next_func = Func::new(
            move |ctx: Ctx<'js>,
                  new_spec: String,
                  opt_ctx: Opt<Value<'js>>|
                  -> Result<Object<'js>> {
                let new_parent = if let Some(val) = opt_ctx.0 {
                    if let Some(ctx_obj) = val.as_object() {
                        ctx_obj
                            .get::<_, String>("parentURL")
                            .unwrap_or_else(|_| spec_clone.clone())
                    } else {
                        spec_clone.clone()
                    }
                } else {
                    spec_clone.clone()
                };
                call_resolve_hooks_from(&ctx, &hooks_clone, index + 1, &new_spec, &new_parent)
            },
        );

        return resolve_fn.call::<_, Object>((spec, context, next_func));
    }

    let obj = Object::new(ctx.clone())?;
    obj.set("url", spec)?;
    obj.set("shortCircuit", false)?;
    obj.set("__nextResolve", false)?;
    Ok(obj)
}
