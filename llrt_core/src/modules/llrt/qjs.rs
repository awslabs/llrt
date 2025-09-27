// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::env;

use once_cell::sync::Lazy;
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    qjs::{self},
    Ctx, IntoJs, Object, Result, Value,
};

use crate::libs::utils::module::{export_default, ModuleInfo};
use crate::modules::require::LLRT_PLATFORM;

static LLRT_PSEUDO_V8_STATS: Lazy<String> = Lazy::new(|| {
    env::var(crate::environment::ENV_LLRT_PSEUDO_V8_STATS)
        .ok()
        .filter(|flag| flag == "1")
        .unwrap_or_else(|| "0".to_string())
});

fn is_v8_stats() -> bool {
    LLRT_PLATFORM.as_str() == "node" || LLRT_PSEUDO_V8_STATS.as_str() == "1"
}

fn get_code_statistics(ctx: Ctx<'_>) -> Result<Value<'_>> {
    let usage = unsafe {
        let mut usage: qjs::JSMemoryUsage = std::mem::zeroed();
        let rt = qjs::JS_GetRuntime(ctx.as_raw().as_ptr());
        qjs::JS_ComputeMemoryUsage(rt, &mut usage);
        usage
    };

    let obj: Object<'_> = Object::new(ctx.clone())?;
    if is_v8_stats() {
        obj.set("code_and_metadata_size", 0)?;
        obj.set("bytecode_and_metadata_size", usage.js_func_code_size)?;
        obj.set("external_script_source_size", 0)?;
        obj.set("cpu_profiler_metadata_size", 0)?;
    } else {
        obj.set("atom_count", usage.atom_count)?;
        obj.set("atom_size", usage.atom_size)?;
        obj.set("str_count", usage.str_count)?;
        obj.set("str_size", usage.str_size)?;
        obj.set("obj_count", usage.obj_count)?;
        obj.set("obj_size", usage.obj_size)?;
        obj.set("prop_count", usage.prop_count)?;
        obj.set("prop_size", usage.prop_size)?;
        obj.set("shape_count", usage.shape_count)?;
        obj.set("shape_size", usage.shape_size)?;
        obj.set("js_func_count", usage.js_func_count)?;
        obj.set("js_func_size", usage.js_func_size)?;
        obj.set("js_func_code_size", usage.js_func_code_size)?;
        obj.set("js_func_pc2line_count", usage.js_func_pc2line_count)?;
        obj.set("js_func_pc2line_size", usage.js_func_pc2line_size)?;
        obj.set("c_func_count", usage.c_func_count)?;
        obj.set("array_count", usage.array_count)?;
        obj.set("fast_array_count", usage.fast_array_count)?;
        obj.set("fast_array_elements", usage.fast_array_elements)?;
        obj.set("binary_object_count", usage.binary_object_count)?;
        obj.set("binary_object_size", usage.binary_object_size)?;
    }

    obj.into_js(&ctx)
}

fn get_heap_statistics(ctx: Ctx<'_>) -> Result<Value<'_>> {
    let usage = unsafe {
        let mut usage: qjs::JSMemoryUsage = std::mem::zeroed();
        let rt = qjs::JS_GetRuntime(ctx.as_raw().as_ptr());
        qjs::JS_ComputeMemoryUsage(rt, &mut usage);
        usage
    };

    let obj: Object<'_> = Object::new(ctx.clone())?;
    if is_v8_stats() {
        obj.set("total_heap_size", usage.memory_used_size)?;
        obj.set("total_heap_size_executable", 0)?;
        obj.set("total_physical_size", 0)?;
        obj.set("total_available_size", 0)?;
        obj.set("used_heap_size", usage.memory_used_size)?;
        obj.set("heap_size_limit", usage.malloc_limit)?;
        obj.set("malloced_memory", usage.malloc_size)?;
        obj.set("peak_malloced_memory", 0)?;
        obj.set("does_zap_garbage", 0)?;
        obj.set("number_of_native_contexts", 0)?;
        obj.set("number_of_detached_contexts", 0)?;
        obj.set("total_global_handles_size", 0)?;
        obj.set("used_global_handles_size", 0)?;
        obj.set("external_memory", 0)?;
    } else {
        obj.set("malloc_size", usage.malloc_size)?;
        obj.set("malloc_limit", usage.malloc_limit)?;
        obj.set("memory_used_size", usage.memory_used_size)?;
        obj.set("malloc_count", usage.malloc_count)?;
        obj.set("memory_used_count", usage.memory_used_count)?;
    }

    obj.into_js(&ctx)
}

pub struct LlrtQjsModule;

impl ModuleDef for LlrtQjsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("getCodeStatistics")?;
        declare.declare("getHeapStatistics")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("getCodeStatistics", Func::from(get_code_statistics))?;
            default.set("getHeapStatistics", Func::from(get_heap_statistics))?;
            Ok(())
        })
    }
}

impl From<LlrtQjsModule> for ModuleInfo<LlrtQjsModule> {
    fn from(val: LlrtQjsModule) -> Self {
        ModuleInfo {
            name: if is_v8_stats() { "v8" } else { "llrt:qjs" },
            module: val,
        }
    }
}
