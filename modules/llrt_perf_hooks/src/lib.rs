// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_events::Emitter;
use llrt_utils::module::{export_default, ModuleInfo};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    Class, Ctx, Object, Result,
};

use crate::performance::Performance;

mod performance;

pub struct PerfHooksModule;

impl ModuleDef for PerfHooksModule {
    fn declare(declare: &Declarations<'_>) -> Result<()> {
        declare.declare("performance")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let globals = ctx.globals();

            let performance: Object = globals.get("performance")?;
            default.set("performance", performance)?;

            Ok(())
        })
    }
}

impl From<PerfHooksModule> for ModuleInfo<PerfHooksModule> {
    fn from(val: PerfHooksModule) -> Self {
        ModuleInfo {
            name: "perf_hooks",
            module: val,
        }
    }
}

pub fn init(ctx: &Ctx<'_>) -> Result<()> {
    let globals = ctx.globals();

    let instance = Class::instance(ctx.clone(), Performance::new())?;
    Performance::add_event_target_prototype(ctx)?;

    globals.set("performance", instance)?;
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use llrt_test::{call_test, test_async_with, ModuleEvaluator};
//     use rquickjs::CatchResultExt;

//     use super::*;

//     #[tokio::test]
//     async fn test_now() {
//         time::init();
//         test_async_with(|ctx| {
//             Box::pin(async move {
//                 ModuleEvaluator::eval_rust::<PerfHooksModule>(ctx.clone(), "perf_hooks")
//                     .await
//                     .catch(&ctx)
//                     .unwrap();

//                 let module = ModuleEvaluator::eval_js(
//                     ctx.clone(),
//                     "test",
//                     r#"
//                         import { performance } from 'perf_hooks';

//                         export async function test() {
//                             const now = performance.now()
//                             // TODO: Delaying with setTimeout
//                             for(let i=0; i < (1<<20); i++){}

//                             return performance.now() - now
//                         }
//                     "#,
//                 )
//                 .await
//                 .unwrap();
//                 let result = call_test::<u32, _>(&ctx, &module, ()).await;
//                 assert!(result > 0)
//             })
//         })
//         .await;
//     }

//     #[tokio::test]
//     async fn test_time_origin() {
//         time::init();
//         test_async_with(|ctx| {
//             Box::pin(async move {
//                 ModuleEvaluator::eval_rust::<PerfHooksModule>(ctx.clone(), "perf_hooks")
//                     .await
//                     .catch(&ctx)
//                     .unwrap();

//                 let module = ModuleEvaluator::eval_js(
//                     ctx.clone(),
//                     "test",
//                     r#"
//                         import { performance } from 'perf_hooks';

//                         export async function test() {
//                             return performance.timeOrigin
//                         }
//                     "#,
//                 )
//                 .await
//                 .catch(&ctx)
//                 .unwrap();
//                 let result = call_test::<f64, _>(&ctx, &module, ()).await;
//                 assert!(result > 0.0);
//             })
//         })
//         .await;
//     }

//     #[tokio::test]
//     async fn test_to_json() {
//         time::init();
//         test_async_with(|ctx| {
//             Box::pin(async move {
//                 ModuleEvaluator::eval_rust::<PerfHooksModule>(ctx.clone(), "perf_hooks")
//                     .await
//                     .unwrap();

//                 let module = ModuleEvaluator::eval_js(
//                     ctx.clone(),
//                     "test",
//                     r#"
//                         import { performance } from 'perf_hooks';

//                         export async function test() {
//                             return performance.toJSON()
//                         }
//                     "#,
//                 )
//                 .await
//                 .unwrap();
//                 let result = call_test::<Object, _>(&ctx, &module, ()).await;
//                 let time_origin = result.get::<_, f64>("timeOrigin").unwrap();
//                 assert!(time_origin > 0.);
//             })
//         })
//         .await;
//     }
// }
