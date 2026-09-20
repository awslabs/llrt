// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Ctx, Function, Result,
};

pub struct TimersModule;

impl ModuleDef for TimersModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("setTimeout")?;
        declare.declare("clearTimeout")?;
        declare.declare("setInterval")?;
        declare.declare("setImmediate")?;
        declare.declare("clearInterval")?;
        declare.declare("queueMicrotask")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let globals = ctx.globals();

        export_default(ctx, exports, |default| {
            let functions = [
                "setTimeout",
                "clearTimeout",
                "setInterval",
                "clearInterval",
                "setImmediate",
                "queueMicrotask",
            ];
            for func_name in functions {
                let function: Function = globals.get(func_name)?;
                default.set(func_name, function)?;
            }
            Ok(())
        })?;

        Ok(())
    }
}

impl From<TimersModule> for ModuleInfo<TimersModule> {
    fn from(val: TimersModule) -> Self {
        ModuleInfo {
            name: "timers",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    llrt_async_runtime::init(ctx)
}

#[cfg(test)]
mod tests {
    use llrt_async_runtime::cleanup;
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};

    use super::*;

    #[tokio::test]
    async fn test_timers() {
        test_async_with(|ctx| {
            Box::pin(async move {
                init(&ctx).unwrap();

                ModuleEvaluator::eval_rust::<TimersModule>(ctx.clone(), "timers")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setTimeout",
                    r#"
                        import { setTimeout } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setTimeout(() => resolve('timeout'), 100);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "timeout");
                cleanup(&ctx).unwrap();

                init(&ctx).unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setImmediate",
                    r#"
                        import { setImmediate } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setImmediate(() => resolve('immediate'));
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "immediate");
                cleanup(&ctx).unwrap();

                init(&ctx).unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_setInterval",
                    r#"
                        import { setInterval, clearInterval } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                let count = 0;
                                const intervalId = setInterval(() => {
                                    count++;
                                    if (count === 3) {
                                        clearInterval(intervalId);
                                        resolve(count);
                                    }
                                }, 10);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<i32, _>(&ctx, &module, ()).await;
                assert_eq!(result, 3);
                cleanup(&ctx).unwrap();

                init(&ctx).unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_nestedTimers",
                    r#"
                        import { setTimeout, setImmediate } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setTimeout(() => {
                                    setImmediate(() => {
                                        setTimeout(() => {
                                            resolve('nested');
                                        }, 10);
                                    });
                                }, 10);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "nested");
                cleanup(&ctx).unwrap();

                init(&ctx).unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_cancelTimeout",
                    r#"
                        import { setTimeout, clearTimeout } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                const timeoutId = setTimeout(() => {
                                    resolve('should not happen');
                                }, 10);
                                clearTimeout(timeoutId);
                                setTimeout(() => resolve('canceled'), 20);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "canceled");
                cleanup(&ctx).unwrap();

                init(&ctx).unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_invalidTimeoutId",
                    r#"
                        import { setTimeout, clearTimeout } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                setTimeout(() => resolve('fired'), 10);
                                clearTimeout(NaN);
                                clearTimeout(-1);
                                clearTimeout(0.5);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "fired");
                cleanup(&ctx).unwrap();

                init(&ctx).unwrap();
                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_multipleIntervals",
                    r#"
                        import { setInterval, clearInterval } from 'timers';
                        export async function test() {
                            return new Promise((resolve) => {
                                let count1 = 0, count2 = 0;
                                const id1 = setInterval(() => {
                                    count1++;
                                    if (count1 === 2) clearInterval(id1);
                                }, 10);
                                const id2 = setInterval(() => {
                                    count2++;
                                    if (count2 === 3) {
                                        clearInterval(id2);
                                        resolve([count1, count2]);
                                    }
                                }, 20);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<Vec<i32>, _>(&ctx, &module, ()).await;
                assert_eq!(result, vec![2, 3]);
                cleanup(&ctx).unwrap();

                init(&ctx).unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test_timerReinitialize",
                    r#"
                        export async function test() {
                            return new Promise((resolve) => {
                                setTimeout(() => resolve('reinitialized'), 0);
                            });
                        }
                    "#,
                )
                .await
                .unwrap();
                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert_eq!(result, "reinitialized");
                cleanup(&ctx).unwrap();
            })
        })
        .await;
    }
}
