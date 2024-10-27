// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    env,
    net::{Ipv4Addr, Ipv6Addr},
    sync::{Arc, Mutex},
    thread,
};

use llrt_utils::{
    module::{export_default, ModuleInfo},
    result::ResultExt,
    sysinfo::{get_arch, get_platform},
};
use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    Ctx, Exception, Object, Result,
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, RefreshKind, System};

#[cfg(unix)]
use self::unix::{get_priority, get_type, get_version, set_priority, DEV_NULL, EOL};
#[cfg(windows)]
use self::windows::{get_priority, get_type, get_version, set_priority, DEV_NULL, EOL};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

static SYSTEM: Lazy<Arc<Mutex<System>>> = Lazy::new(|| {
    Arc::new(Mutex::new(System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::new().with_cpu_usage().with_frequency())
            .with_memory(MemoryRefreshKind::new().with_ram()),
    )))
});

static NETWORKS: Lazy<Arc<Mutex<Networks>>> =
    Lazy::new(|| Arc::new(Mutex::new(Networks::new_with_refreshed_list())));

fn get_available_parallelism() -> usize {
    num_cpus::get()
}

fn get_cpus(ctx: Ctx<'_>) -> Result<Vec<Object>> {
    let mut vec: Vec<Object> = Vec::new();
    let mut system = SYSTEM.lock().unwrap();

    thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_all();

    for cpu in system.cpus() {
        let obj = Object::new(ctx.clone())?;
        obj.set("model", cpu.brand())?;
        obj.set("speed", cpu.frequency())?;

        // The number of milliseconds spent by the CPU in each mode cannot be obtained at this time.
        let times = Object::new(ctx.clone())?;
        times.set("user", 0)?;
        times.set("nice", 0)?;
        times.set("sys", 0)?;
        times.set("idle", 0)?;
        times.set("irq", 0)?;
        obj.set("times", times)?;

        vec.push(obj);
    }
    Ok(vec)
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

fn get_free_mem() -> u64 {
    let mut system = SYSTEM.lock().unwrap();

    system.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());
    system.free_memory()
}

fn get_home_dir(ctx: Ctx<'_>) -> Result<String> {
    match home::home_dir() {
        Some(val) => Ok(val.to_string_lossy().into_owned()),
        None => Err(Exception::throw_message(
            &ctx,
            "Could not determine home directory",
        )),
    }
}

fn get_host_name(ctx: Ctx<'_>) -> Result<String> {
    match System::host_name() {
        Some(val) => Ok(val),
        None => Err(Exception::throw_reference(&ctx, "System::host_name")),
    }
}

fn get_load_avg() -> Vec<f64> {
    let load_avg = System::load_average();

    vec![load_avg.one, load_avg.five, load_avg.fifteen]
}

fn get_machine(ctx: Ctx<'_>) -> Result<String> {
    match System::cpu_arch() {
        Some(val) => Ok(val),
        None => Err(Exception::throw_reference(&ctx, "System::cpu_arch")),
    }
}

fn get_network_interfaces(ctx: Ctx<'_>) -> Result<HashMap<String, Vec<Object>>> {
    let mut map: HashMap<String, Vec<Object>> = HashMap::new();
    let networks = NETWORKS.lock().unwrap();

    for (interface_name, network_data) in networks.iter() {
        let mut ifs = Vec::new();

        for ip_network in network_data.ip_networks() {
            let addr = &ip_network.addr.to_string();
            let is_ipv4 = addr.contains(".");
            let (is_internal, scope_id) = if is_ipv4 {
                get_attribute_ipv4(&ctx, addr)?
            } else {
                get_attribute_ipv6(&ctx, addr)?
            };

            let obj = Object::new(ctx.clone())?;
            obj.set("address", addr)?;
            obj.set(
                "netmask",
                if is_ipv4 {
                    prefix_to_netmask_ipv4(ip_network.prefix)
                } else {
                    prefix_to_netmask_ipv6(ip_network.prefix)
                }
                .to_string(),
            )?;
            obj.set("family", if is_ipv4 { "IPv4" } else { "IPv6" })?;
            obj.set("mac", network_data.mac_address().to_string())?;
            obj.set("internal", is_internal)?;
            obj.set("cidr", [addr, "/", &ip_network.prefix.to_string()].concat())?;
            if !is_ipv4 {
                obj.set("scopeid", scope_id)?;
            }

            ifs.push(obj);
        }
        if !ifs.is_empty() {
            map.insert(interface_name.to_string(), ifs);
        }
    }
    Ok(map)
}

fn prefix_to_netmask_ipv4(prefix: u8) -> Box<str> {
    let mut prefix = prefix;

    if prefix > 32 {
        return Box::from("");
    }

    let mut mask = [0u8; 4];

    #[allow(clippy::needless_range_loop)]
    for i in 0..4 {
        if prefix >= 8 {
            mask[i] = 255;
            prefix -= 8;
        } else if prefix > 0 {
            mask[i] = 255 << (8 - prefix);
            break;
        }
    }
    Box::from(Ipv4Addr::new(mask[0], mask[1], mask[2], mask[3]).to_string())
}

fn prefix_to_netmask_ipv6(prefix: u8) -> Box<str> {
    let mut prefix = prefix;

    if prefix > 128 {
        return Box::from("");
    }

    let mut mask = [0u16; 8];

    #[allow(clippy::needless_range_loop)]
    for i in 0..8 {
        if prefix >= 16 {
            mask[i] = 0xFFFF;
            prefix -= 16;
        } else if prefix > 0 {
            mask[i] = 0xFFFF << (16 - prefix);
            break;
        }
    }
    Box::from(
        Ipv6Addr::new(
            mask[0], mask[1], mask[2], mask[3], mask[4], mask[5], mask[6], mask[7],
        )
        .to_string(),
    )
}

fn get_attribute_ipv4(ctx: &Ctx<'_>, addr: &str) -> Result<(bool, u8)> {
    let addr = addr.parse::<Ipv4Addr>().or_throw(ctx)?;
    let is_internal = addr.is_broadcast()
        || addr.is_documentation()
        || addr.is_link_local()
        || addr.is_loopback()
        || addr.is_multicast()
        || addr.is_unspecified();
    let scope_id = 0; // For IPv4, ScopeID is a dummy value.

    Ok((is_internal, scope_id))
}

fn get_attribute_ipv6(ctx: &Ctx<'_>, addr: &str) -> Result<(bool, u8)> {
    let addr = addr.parse::<Ipv6Addr>().or_throw(ctx)?;
    let is_internal = addr.is_loopback() || addr.is_multicast() || addr.is_unspecified();
    let scope_id = 0; // ScopeID is not supported at this time.

    Ok((is_internal, scope_id))
}

fn get_release(ctx: Ctx<'_>) -> Result<String> {
    match System::kernel_version() {
        Some(val) => Ok(val),
        None => Err(Exception::throw_reference(&ctx, "System::kernel_version")),
    }
}

fn get_tmp_dir() -> String {
    env::temp_dir().to_string_lossy().to_string()
}

fn get_total_mem() -> u64 {
    let mut system = SYSTEM.lock().unwrap();

    system.refresh_memory_specifics(MemoryRefreshKind::new().with_ram());
    system.total_memory()
}

fn get_uptime() -> u64 {
    System::uptime()
}

pub struct OsModule;

impl ModuleDef for OsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("arch")?;
        declare.declare("availableParallelism")?;
        declare.declare("cpus")?;
        declare.declare("devNull")?;
        declare.declare("endianness")?;
        declare.declare("EOL")?;
        declare.declare("freemem")?;
        declare.declare("getPriority")?;
        declare.declare("homedir")?;
        declare.declare("hostname")?;
        declare.declare("loadavg")?;
        declare.declare("machine")?;
        declare.declare("networkInterfaces")?;
        declare.declare("platform")?;
        declare.declare("release")?;
        declare.declare("setPriority")?;
        declare.declare("tmpdir")?;
        declare.declare("totalmem")?;
        declare.declare("type")?;
        declare.declare("uptime")?;
        // declare.declare("userInfo")?;
        declare.declare("version")?;

        declare.declare("constants")?;

        declare.declare("default")?;

        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("arch", Func::from(get_arch))?;
            default.set(
                "availableParallelism",
                Func::from(get_available_parallelism),
            )?;
            default.set("cpus", Func::from(get_cpus))?;
            default.set("devNull", DEV_NULL)?;
            default.set("endianness", Func::from(get_endianness))?;
            default.set("EOL", EOL)?;
            default.set("freemem", Func::from(get_free_mem))?;
            default.set("getPriority", Func::from(get_priority))?;
            default.set("homedir", Func::from(get_home_dir))?;
            default.set("hostname", Func::from(get_host_name))?;
            default.set("loadavg", Func::from(get_load_avg))?;
            default.set("machine", Func::from(get_machine))?;
            default.set("networkInterfaces", Func::from(get_network_interfaces))?;
            default.set("platform", Func::from(get_platform))?;
            default.set("release", Func::from(get_release))?;
            default.set("setPriority", Func::from(set_priority))?;
            default.set("tmpdir", Func::from(get_tmp_dir))?;
            default.set("totalmem", Func::from(get_total_mem))?;
            default.set("type", Func::from(get_type))?;
            default.set("uptime", Func::from(get_uptime))?;
            default.set("version", Func::from(get_version))?;

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
