// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use llrt_utils::{any_of::AnyOf2, bytes::ObjectBytes, object::ObjectExt, result::ResultExt};
use rquickjs::{
    loader::Loader,
    module::ModuleDef,
    prelude::{Func, Opt},
    Ctx, Error, Module, Object, Result, Value,
};
use tracing::trace;

use super::{Hook, ModuleHookState};

type LoadFn = for<'js> fn(Ctx<'js>, Vec<u8>) -> Result<Module<'js>>;
type Source<'js> = AnyOf2<String, ObjectBytes<'js>>;

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
            return match source {
                AnyOf2::A(s) => Module::declare(ctx.clone(), name, s),
                AnyOf2::B(b) => Module::declare(ctx.clone(), name, b.as_bytes(ctx)?),
            };
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

pub fn module_hook_load<'js>(ctx: &Ctx<'js>, name: &str) -> Result<(bool, bool, Source<'js>)> {
    let bind_state = ctx.userdata::<RefCell<ModuleHookState>>().or_throw(ctx)?;
    let hooks = Rc::new(bind_state.borrow().hooks.clone());

    if hooks.is_empty() {
        return Ok((false, false, AnyOf2::A("".into())));
    }

    let result = call_load_hooks(ctx, &hooks, name)?;

    let short_circuit = result
        .get_optional::<_, bool>("shortCircuit")?
        .unwrap_or(false);

    let next_load = result
        .get_optional::<_, bool>("__nextLoad")?
        .unwrap_or(false);

    let source = result
        .get_optional::<_, Source>("source")?
        .unwrap_or(AnyOf2::A("".into()));

    Ok((short_circuit, next_load, source))
}

#[allow(dependency_on_unit_never_type_fallback)]
fn call_load_hooks<'js>(
    ctx: &Ctx<'js>,
    hooks: &Rc<Vec<Hook<'js>>>,
    url: &str,
) -> Result<Object<'js>> {
    call_load_hooks_from(ctx, hooks, 0, url)
}

fn call_load_hooks_from<'js>(
    ctx: &Ctx<'js>,
    hooks: &Rc<Vec<Hook<'js>>>,
    start_index: usize,
    url: &str,
) -> Result<Object<'js>> {
    for index in start_index..hooks.len() {
        let Some(load_fn) = &hooks[index].load else {
            continue;
        };

        let context = Object::new(ctx.clone())?;
        let hooks_clone = Rc::clone(hooks);

        let next_func = Func::new(
            move |ctx: Ctx<'js>,
                  new_url: String,
                  _opt_ctx: Opt<Value<'js>>|
                  -> Result<Object<'js>> {
                call_load_hooks_from(&ctx, &hooks_clone, index + 1, &new_url)
            },
        );

        return load_fn.call::<_, Object>((url, context, next_func));
    }

    let obj = Object::new(ctx.clone())?;
    obj.set("url", url)?;
    obj.set("shortCircuit", false)?;
    obj.set("__nextLoad", false)?;
    Ok(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use llrt_test::test_async_with;
    use rquickjs::Function;

    #[tokio::test]
    async fn test_hook_override_import() {
        use llrt_test::ModuleEvaluator;

        test_async_with(|ctx| {
            Box::pin(async move {
                let _ = ctx.store_userdata(RefCell::new(ModuleHookState::default()));

                let hook_code = r#"
                    globalThis.hookCalled = false;
                    globalThis.nextLoadCalled = false;
                    
                    export function load(url, context, nextLoad) {
                        globalThis.hookCalled = true;
                        if (url === "math") {
                            return {
                                format: "module",
                                shortCircuit: true,
                                source: "export function add(a, b) { return a + b + 1; }"
                            };
                        }
                        globalThis.nextLoadCalled = true;
                        return nextLoad(url, context);
                    }
                "#;

                let hook_module = ModuleEvaluator::eval_js(ctx.clone(), "hook", hook_code)
                    .await
                    .unwrap();

                let load_fn: Function = hook_module.get("load").unwrap();
                let hook = Hook {
                    resolve: None,
                    load: Some(load_fn),
                };

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                binding.borrow_mut().hooks.push(hook);

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                let hooks = Rc::new(binding.borrow().hooks.clone());

                let result = call_load_hooks(&ctx, &hooks, "math").unwrap();

                let globals = ctx.globals();
                assert_eq!(globals.get::<_, bool>("hookCalled").unwrap(), true);
                assert_eq!(result.get::<_, bool>("shortCircuit").unwrap(), true);
                assert_eq!(
                    result.get::<_, String>("source").unwrap(),
                    "export function add(a, b) { return a + b + 1; }"
                );

                let result2 = call_load_hooks(&ctx, &hooks, "other").unwrap();
                assert_eq!(globals.get::<_, bool>("nextLoadCalled").unwrap(), true);
                assert_eq!(result2.get::<_, bool>("shortCircuit").unwrap(), false);
                assert_eq!(result2.get::<_, String>("url").unwrap(), "other");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_multiple_hooks_chain() {
        use llrt_test::ModuleEvaluator;

        test_async_with(|ctx| {
            Box::pin(async move {
                let _ = ctx.store_userdata(RefCell::new(ModuleHookState::default()));

                let hook1_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.hook1Called = true;
                        if (url === "skip") {
                            return nextLoad(url, context);
                        }
                        return nextLoad("modified-" + url, context);
                    }
                "#;

                let hook2_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.hook2Called = true;
                        globalThis.finalUrl = url;
                        return {
                            shortCircuit: true,
                            source: "export const value = 42;"
                        };
                    }
                "#;

                let hook1 = ModuleEvaluator::eval_js(ctx.clone(), "hook1", hook1_code)
                    .await
                    .unwrap();
                let hook2 = ModuleEvaluator::eval_js(ctx.clone(), "hook2", hook2_code)
                    .await
                    .unwrap();

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook1.get("load").unwrap()),
                });
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook2.get("load").unwrap()),
                });

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                let hooks = Rc::new(binding.borrow().hooks.clone());

                let result = call_load_hooks(&ctx, &hooks, "test").unwrap();

                let globals = ctx.globals();
                assert_eq!(globals.get::<_, bool>("hook1Called").unwrap(), true);
                assert_eq!(globals.get::<_, bool>("hook2Called").unwrap(), true);
                assert_eq!(
                    globals.get::<_, String>("finalUrl").unwrap(),
                    "modified-test"
                );
                assert_eq!(result.get::<_, bool>("shortCircuit").unwrap(), true);
                assert_eq!(
                    result.get::<_, String>("source").unwrap(),
                    "export const value = 42;"
                );
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_hook_without_nextload() {
        use llrt_test::ModuleEvaluator;

        test_async_with(|ctx| {
            Box::pin(async move {
                let _ = ctx.store_userdata(RefCell::new(ModuleHookState::default()));

                let hook_code = r#"
                    export function load(url, context, nextLoad) {
                        return {
                            shortCircuit: true,
                            source: "export const intercepted = true;"
                        };
                    }
                "#;

                let hook_module = ModuleEvaluator::eval_js(ctx.clone(), "hook", hook_code)
                    .await
                    .unwrap();

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook_module.get("load").unwrap()),
                });

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                let hooks = Rc::new(binding.borrow().hooks.clone());

                let result = call_load_hooks(&ctx, &hooks, "any").unwrap();

                assert_eq!(result.get::<_, bool>("shortCircuit").unwrap(), true);
                assert_eq!(
                    result.get::<_, String>("source").unwrap(),
                    "export const intercepted = true;"
                );
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_hook_passthrough_all() {
        use llrt_test::ModuleEvaluator;

        test_async_with(|ctx| {
            Box::pin(async move {
                let _ = ctx.store_userdata(RefCell::new(ModuleHookState::default()));

                let hook_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.passedThrough = url;
                        return nextLoad(url, context);
                    }
                "#;

                let hook_module = ModuleEvaluator::eval_js(ctx.clone(), "hook", hook_code)
                    .await
                    .unwrap();

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook_module.get("load").unwrap()),
                });

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                let hooks = Rc::new(binding.borrow().hooks.clone());

                let result = call_load_hooks(&ctx, &hooks, "passthrough").unwrap();

                let globals = ctx.globals();
                assert_eq!(
                    globals.get::<_, String>("passedThrough").unwrap(),
                    "passthrough"
                );
                assert_eq!(result.get::<_, bool>("shortCircuit").unwrap(), false);
                assert_eq!(result.get::<_, String>("url").unwrap(), "passthrough");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_hook_conditional_intercept() {
        use llrt_test::ModuleEvaluator;

        test_async_with(|ctx| {
            Box::pin(async move {
                let _ = ctx.store_userdata(RefCell::new(ModuleHookState::default()));

                let hook_code = r#"
                    export function load(url, context, nextLoad) {
                        if (url.startsWith("internal:")) {
                            return {
                                shortCircuit: true,
                                source: "export const internal = true;"
                            };
                        }
                        return nextLoad(url, context);
                    }
                "#;

                let hook_module = ModuleEvaluator::eval_js(ctx.clone(), "hook", hook_code)
                    .await
                    .unwrap();

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook_module.get("load").unwrap()),
                });

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                let hooks = Rc::new(binding.borrow().hooks.clone());

                let result1 = call_load_hooks(&ctx, &hooks, "internal:test").unwrap();
                assert_eq!(result1.get::<_, bool>("shortCircuit").unwrap(), true);
                assert_eq!(
                    result1.get::<_, String>("source").unwrap(),
                    "export const internal = true;"
                );

                let result2 = call_load_hooks(&ctx, &hooks, "external:test").unwrap();
                assert_eq!(result2.get::<_, bool>("shortCircuit").unwrap(), false);
                assert_eq!(result2.get::<_, String>("url").unwrap(), "external:test");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_three_hooks_selective_intercept() {
        use llrt_test::ModuleEvaluator;

        test_async_with(|ctx| {
            Box::pin(async move {
                let _ = ctx.store_userdata(RefCell::new(ModuleHookState::default()));

                let hook1_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.order = ["hook1"];
                        return nextLoad(url, context);
                    }
                "#;

                let hook2_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.order.push("hook2");
                        if (url === "intercept-here") {
                            return {
                                shortCircuit: true,
                                source: "export const from = 'hook2';"
                            };
                        }
                        return nextLoad(url, context);
                    }
                "#;

                let hook3_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.order.push("hook3");
                        return {
                            shortCircuit: true,
                            source: "export const from = 'hook3';"
                        };
                    }
                "#;

                let hook1 = ModuleEvaluator::eval_js(ctx.clone(), "hook1", hook1_code)
                    .await
                    .unwrap();
                let hook2 = ModuleEvaluator::eval_js(ctx.clone(), "hook2", hook2_code)
                    .await
                    .unwrap();
                let hook3 = ModuleEvaluator::eval_js(ctx.clone(), "hook3", hook3_code)
                    .await
                    .unwrap();

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook1.get("load").unwrap()),
                });
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook2.get("load").unwrap()),
                });
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook3.get("load").unwrap()),
                });

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                let hooks = Rc::new(binding.borrow().hooks.clone());

                let result1 = call_load_hooks(&ctx, &hooks, "intercept-here").unwrap();
                let globals = ctx.globals();
                let order: Vec<String> = globals.get("order").unwrap();
                assert_eq!(order, vec!["hook1", "hook2"]);
                assert_eq!(
                    result1.get::<_, String>("source").unwrap(),
                    "export const from = 'hook2';"
                );
                assert_eq!(result1.get::<_, bool>("shortCircuit").unwrap(), true);

                let result2 = call_load_hooks(&ctx, &hooks, "other").unwrap();
                let order2: Vec<String> = globals.get("order").unwrap();
                assert_eq!(order2, vec!["hook1", "hook2", "hook3"]);
                assert_eq!(
                    result2.get::<_, String>("source").unwrap(),
                    "export const from = 'hook3';"
                );
                assert_eq!(result2.get::<_, bool>("shortCircuit").unwrap(), true);
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_hook_url_transformation_chain() {
        use llrt_test::ModuleEvaluator;

        test_async_with(|ctx| {
            Box::pin(async move {
                let _ = ctx.store_userdata(RefCell::new(ModuleHookState::default()));

                let hook1_code = r#"
                    export function load(url, context, nextLoad) {
                        return nextLoad(url.replace("@", "node_modules/"), context);
                    }
                "#;

                let hook2_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.transformedUrl = url;
                        return nextLoad(url + "/index.js", context);
                    }
                "#;

                let hook3_code = r#"
                    export function load(url, context, nextLoad) {
                        globalThis.finalUrl = url;
                        return {
                            shortCircuit: true,
                            source: "export default {};"
                        };
                    }
                "#;

                let hook1 = ModuleEvaluator::eval_js(ctx.clone(), "hook1", hook1_code)
                    .await
                    .unwrap();
                let hook2 = ModuleEvaluator::eval_js(ctx.clone(), "hook2", hook2_code)
                    .await
                    .unwrap();
                let hook3 = ModuleEvaluator::eval_js(ctx.clone(), "hook3", hook3_code)
                    .await
                    .unwrap();

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook1.get("load").unwrap()),
                });
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook2.get("load").unwrap()),
                });
                binding.borrow_mut().hooks.push(Hook {
                    resolve: None,
                    load: Some(hook3.get("load").unwrap()),
                });

                let binding = ctx.userdata::<RefCell<ModuleHookState>>().unwrap();
                let hooks = Rc::new(binding.borrow().hooks.clone());

                let result = call_load_hooks(&ctx, &hooks, "@pkg/module").unwrap();

                let globals = ctx.globals();
                assert_eq!(
                    globals.get::<_, String>("transformedUrl").unwrap(),
                    "node_modules/pkg/module"
                );
                assert_eq!(
                    globals.get::<_, String>("finalUrl").unwrap(),
                    "node_modules/pkg/module/index.js"
                );
                assert_eq!(result.get::<_, bool>("shortCircuit").unwrap(), true);
                assert_eq!(
                    result.get::<_, String>("source").unwrap(),
                    "export default {};"
                );
            })
        })
        .await;
    }
}
