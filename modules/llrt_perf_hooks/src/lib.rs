// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use llrt_utils::{
    module::{export_default, ModuleInfo},
    time,
};
use rquickjs::{
    atom::PredefinedAtom,
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    runtime::UserData,
    Ctx, Object, Result,
};

fn get_time_origin() -> f64 {
    let time_origin = time::origin_nanos() as f64;

    time_origin / 1e6
}

fn now() -> f64 {
    let now = time::now_nanos();
    let started = time::origin_nanos();
    let elapsed = now.checked_sub(started).unwrap_or_default();

    (elapsed as f64) / 1e6
}

fn to_json(ctx: Ctx<'_>) -> Result<Object<'_>> {
    let obj = Object::new(ctx.clone())?;

    obj.set("timeOrigin", get_time_origin())?;

    Ok(obj)
}

struct PerfInitedUserData;

unsafe impl<'js> UserData<'js> for PerfInitedUserData {
    type Static = PerfInitedUserData;
}

fn new_performance(ctx: Ctx<'_>) -> Result<Object<'_>> {
    let global = ctx.globals();

    //let inited = global.get::<_, Option<bool>>("__perfInit")?.is_some();

    let inited = ctx.userdata::<PerfInitedUserData>().is_some();
    if !inited {
        //global.set("__perfInit", true)?;
        ctx.store_userdata(PerfInitedUserData)?;
        let performance = Object::new(ctx)?;
        performance.set("timeOrigin", get_time_origin())?;
        performance.set("now", Func::from(now))?;
        performance.set(PredefinedAtom::ToJSON, Func::from(to_json))?;
        global.set("performance", performance.clone())?;
        return Ok(performance);
    }
    global.get("performance")
}

pub struct PerfHooksModule;

impl ModuleDef for PerfHooksModule {
    fn declare(declare: &Declarations<'_>) -> Result<()> {
        declare.declare("performance")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            let performance = new_performance(ctx.clone())?;
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
    new_performance(ctx.clone())?;
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
