// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use llrt_utils::module::export_default;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Result,
};

#[cfg(unix)]
use self::unix::{get_release, get_type, get_version, EOL};
#[cfg(windows)]
use self::windows::{get_release, get_type, get_version, EOL};
use crate::module_info::ModuleInfo;
use crate::sysinfo::get_platform;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

fn get_tmp_dir() -> String {
    env::temp_dir().to_string_lossy().to_string()
}

fn get_available_parallelism() -> usize {
    num_cpus::get()
}

pub struct OsModule;

impl ModuleDef for OsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("type")?;
        declare.declare("release")?;
        declare.declare("tmpdir")?;
        declare.declare("platform")?;
        declare.declare("version")?;
        declare.declare("EOL")?;
        declare.declare("availableParallelism")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("type", Func::from(get_type))?;
            default.set("release", Func::from(get_release))?;
            default.set("tmpdir", Func::from(get_tmp_dir))?;
            default.set("platform", Func::from(get_platform))?;
            default.set("version", Func::from(get_version))?;
            default.set("EOL", EOL)?;
            default.set(
                "availableParallelism",
                Func::from(get_available_parallelism),
            )?;

            Ok(())
        })
    }
}

impl From<OsModule> for ModuleInfo<OsModule> {
    fn from(val: OsModule) -> Self {
        ModuleInfo {
            name: "os",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{call_test, test_async_with, ModuleEvaluator};

    #[tokio::test]
    async fn test_type() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<OsModule>(ctx.clone(), "os")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { type } from 'os';

                        export async function test() {
                            return type()
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<String, _>(&ctx, &module, ()).await;

                assert!(result == "Linux" || result == "Windows_NT" || result == "Darwin");
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_release() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<OsModule>(ctx.clone(), "os")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { release } from 'os';

                        export async function test() {
                            return release()
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<String, _>(&ctx, &module, ()).await;

                assert!(!result.is_empty()); // Format is platform dependant
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_version() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<OsModule>(ctx.clone(), "os")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { version } from 'os';

                        export async function test() {
                            return version()
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<String, _>(&ctx, &module, ()).await;

                assert!(!result.is_empty()); // Format is platform dependant
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_available_parallelism() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<OsModule>(ctx.clone(), "os")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { availableParallelism } from 'os';

                        export async function test() {
                            return availableParallelism()
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<usize, _>(&ctx, &module, ()).await;

                assert!(result > 0);
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_eol() {
        test_async_with(|ctx| {
            Box::pin(async move {
                ModuleEvaluator::eval_rust::<OsModule>(ctx.clone(), "os")
                    .await
                    .unwrap();

                let module = ModuleEvaluator::eval_js(
                    ctx.clone(),
                    "test",
                    r#"
                        import { EOL } from 'os';

                        export async function test() {
                            return EOL
                        }
                    "#,
                )
                .await
                .unwrap();

                let result = call_test::<String, _>(&ctx, &module, ()).await;
                assert!(result == EOL);
            })
        })
        .await;
    }
}
