// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use rquickjs::{Ctx, Object, Result};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

static SYSTEM: Lazy<Arc<Mutex<System>>> = Lazy::new(|| {
    Arc::new(Mutex::new(System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage().with_frequency())
            .with_memory(MemoryRefreshKind::nothing().with_ram()),
    )))
});

pub fn get_cpus(ctx: Ctx<'_>) -> Result<Vec<Object<'_>>> {
    let mut vec: Vec<Object> = Vec::new();
    let system = SYSTEM.lock().unwrap();

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

pub fn get_free_mem() -> u64 {
    let mut system = SYSTEM.lock().unwrap();

    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    system.free_memory()
}

pub fn get_total_mem() -> u64 {
    let mut system = SYSTEM.lock().unwrap();

    system.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    system.total_memory()
}
