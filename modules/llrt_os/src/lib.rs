// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]

use std::env;

use llrt_utils::{
    module::{export_default, ModuleInfo},
    sysinfo::{ARCH, PLATFORM},
};
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Exception, Result,
};

#[cfg(feature = "system")]
use sysinfo::System;

#[cfg(unix)]
use self::unix::{
    get_priority, get_release, get_type, get_user_info, get_version, set_priority, DEV_NULL, EOL,
};
#[cfg(windows)]
use self::windows::{
    get_priority, get_release, get_type, get_user_info, get_version, set_priority, DEV_NULL, EOL,
};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(feature = "network")]
use self::network::get_network_interfaces;
#[cfg(feature = "statistics")]
use self::statistics::{get_cpus, get_free_mem, get_total_mem};

#[cfg(feature = "network")]
mod network;
#[cfg(feature = "statistics")]
mod statistics;

fn get_available_parallelism() -> usize {
    num_cpus::get()
}

fn get_endianness() -> &'static str {
    #[cfg(target_endian = "little")]
    {
        "LE"
    }
    #[cfg(target_endian = "big")]
    {
        "BE"
    }
}

fn get_home_dir(ctx: Ctx<'_>) -> Result<String> {
    home::home_dir()
        .map(|val| val.to_string_lossy().into_owned())
        .ok_or_else(|| Exception::throw_message(&ctx, "Could not determine home directory"))
}

#[cfg(feature = "system")]
fn get_host_name(ctx: Ctx<'_>) -> Result<String> {
    System::host_name().ok_or_else(|| Exception::throw_reference(&ctx, "System::host_name"))
}

#[cfg(feature = "system")]
fn get_load_avg() -> Vec<f64> {
    let load_avg = System::load_average();

    vec![load_avg.one, load_avg.five, load_avg.fifteen]
}

#[cfg(feature = "system")]
fn get_machine() -> String {
    System::cpu_arch()
}

fn get_tmp_dir() -> String {
    env::temp_dir().to_string_lossy().to_string()
}

#[cfg(feature = "system")]
fn get_uptime() -> u64 {
    System::uptime()
}

pub struct OsModule;

impl ModuleDef for OsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("arch")?;
        declare.declare("availableParallelism")?;
        declare.declare("devNull")?;
        declare.declare("endianness")?;
        declare.declare("EOL")?;
        declare.declare("getPriority")?;
        declare.declare("homedir")?;
        declare.declare("platform")?;
        declare.declare("release")?;
        declare.declare("setPriority")?;
        declare.declare("tmpdir")?;
        declare.declare("type")?;
        declare.declare("userInfo")?;
        declare.declare("version")?;

        #[cfg(feature = "network")]
        {
            declare.declare("networkInterfaces")?;
        }

        #[cfg(feature = "statistics")]
        {
            declare.declare("cpus")?;
            declare.declare("freemem")?;
            declare.declare("totalmem")?;
        }
        #[cfg(feature = "system")]
        {
            declare.declare("hostname")?;
            declare.declare("loadavg")?;
            declare.declare("machine")?;
            declare.declare("uptime")?;
        }
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("arch", Func::from(|| ARCH))?;
            default.set(
                "availableParallelism",
                Func::from(get_available_parallelism),
            )?;
            default.set("devNull", DEV_NULL)?;
            default.set("endianness", Func::from(get_endianness))?;
            default.set("EOL", EOL)?;
            default.set("getPriority", Func::from(get_priority))?;
            default.set("homedir", Func::from(get_home_dir))?;
            default.set("platform", Func::from(|| PLATFORM))?;
            default.set("release", Func::from(get_release))?;
            default.set("setPriority", Func::from(set_priority))?;
            default.set("tmpdir", Func::from(get_tmp_dir))?;
            default.set("type", Func::from(get_type))?;
            default.set("userInfo", Func::from(get_user_info))?;
            default.set("version", Func::from(get_version))?;
            #[cfg(feature = "network")]
            {
                default.set("networkInterfaces", Func::from(get_network_interfaces))?;
            }

            #[cfg(feature = "statistics")]
            {
                default.set("cpus", Func::from(get_cpus))?;
                default.set("freemem", Func::from(get_free_mem))?;
                default.set("totalmem", Func::from(get_total_mem))?;
            }
            #[cfg(feature = "system")]
            {
                default.set("hostname", Func::from(get_host_name))?;
                default.set("loadavg", Func::from(get_load_avg))?;
                default.set("machine", Func::from(get_machine))?;
                default.set("uptime", Func::from(get_uptime))?;
            }
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
    use llrt_test::{call_test, test_async_with, ModuleEvaluator};
    use rquickjs::{Ctx, Value};

    use super::*;

    async fn run_test_return_string(
        ctx: &Ctx<'_>,
        name: &str,
        is_function: bool,
        expected_assertion: impl Fn(String),
    ) {
        ModuleEvaluator::eval_rust::<OsModule>(ctx.clone(), "os")
            .await
            .unwrap();

        let brackets = if is_function { "()" } else { "" };
        let module = ModuleEvaluator::eval_js(
            ctx.clone(),
            "test",
            &format!(
                r#"
                    import {{ {} }} from 'os';

                    export async function test() {{
                        return {}{}
                    }}
                "#,
                name, name, brackets
            ),
        )
        .await
        .unwrap();

        let result = call_test::<String, _>(ctx, &module, ()).await;
        expected_assertion(result);
    }

    async fn run_test_return_number(ctx: &Ctx<'_>, name: &str, expected_assertion: impl Fn(Value)) {
        ModuleEvaluator::eval_rust::<OsModule>(ctx.clone(), "os")
            .await
            .unwrap();

        let module = ModuleEvaluator::eval_js(
            ctx.clone(),
            "test",
            &format!(
                r#"
                    import {{ {} }} from 'os';

                    export async function test() {{
                        return {}()
                    }}
                "#,
                name, name
            ),
        )
        .await
        .unwrap();

        let result = call_test::<Value, _>(ctx, &module, ()).await;
        expected_assertion(result);
    }

    #[tokio::test]
    async fn test_available_parallelism() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_number(&ctx, "availableParallelism", |result| {
                    assert!(result.is_number()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_arch() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "type", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_devnull() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "devNull", false, |result| {
                    assert_eq!(result, DEV_NULL);
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_endianness() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "endianness", true, |result| {
                    let endianness = if cfg!(target_endian = "little") {
                        "LE".to_string()
                    } else {
                        "BE".to_string()
                    };
                    assert_eq!(result, endianness);
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_eol() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "EOL", false, |result| {
                    assert_eq!(result, EOL);
                })
                .await;
            })
        })
        .await;
    }

    #[cfg(feature = "statistics")]
    #[tokio::test]
    async fn test_freemem() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_number(&ctx, "freemem", |result| {
                    assert!(result.is_number()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_homedir() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "homedir", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[cfg(feature = "system")]
    #[tokio::test]
    async fn test_hostname() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "hostname", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[cfg(feature = "system")]
    #[tokio::test]
    async fn test_machine() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "machine", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_platform() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "platform", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_release() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "release", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_tmpdir() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "tmpdir", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[cfg(feature = "statistics")]
    #[tokio::test]
    async fn test_totalmem() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_number(&ctx, "totalmem", |result| {
                    assert!(result.is_number()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_type() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "type", true, |result| {
                    assert!(result == "Linux" || result == "Windows_NT" || result == "Darwin");
                })
                .await;
            })
        })
        .await;
    }

    #[cfg(feature = "system")]
    #[tokio::test]
    async fn test_uptime() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_number(&ctx, "uptime", |result| {
                    assert!(result.is_number()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }

    #[tokio::test]
    async fn test_version() {
        test_async_with(|ctx| {
            Box::pin(async move {
                run_test_return_string(&ctx, "version", true, |result| {
                    assert!(!result.is_empty()); // platform dependant
                })
                .await;
            })
        })
        .await;
    }
}
