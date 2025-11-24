// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use rquickjs::{
    module::{Declarations, Exports, ModuleDef},
    prelude::Func,
    qjs, Ctx, IntoJs, Object, Result, Value,
};

use crate::libs::utils::module::{export_default, ModuleInfo};

// SAFETY:
// - The associated runtime must not be accessed concurrently or destroyed
//   while this function runs (QuickJS is not thread-safe).
// - Undefined behavior may occur if called with an invalid or corrupted runtime.
unsafe fn js_compute_memory_usage(ctx: &Ctx) -> qjs::JSMemoryUsage {
    let mut usage: qjs::JSMemoryUsage = std::mem::zeroed();
    let rt = qjs::JS_GetRuntime(ctx.as_raw().as_ptr());
    qjs::JS_ComputeMemoryUsage(rt, &mut usage);
    usage
}

fn compute_memory_usage(ctx: Ctx) -> Result<Value> {
    let usage = unsafe { js_compute_memory_usage(&ctx) };

    let obj = Object::new(ctx.clone())?;
    obj.set("malloc_size", usage.malloc_size)?;
    obj.set("malloc_limit", usage.malloc_limit)?;
    obj.set("memory_used_size", usage.memory_used_size)?;
    obj.set("malloc_count", usage.malloc_count)?;
    obj.set("memory_used_count", usage.memory_used_count)?;
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

    obj.into_js(&ctx)
}

pub struct LlrtQjsModule;

impl ModuleDef for LlrtQjsModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("ComputeMemoryUsage")?;
        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("ComputeMemoryUsage", Func::from(compute_memory_usage))?;
            Ok(())
        })
    }
}

impl From<LlrtQjsModule> for ModuleInfo<LlrtQjsModule> {
    fn from(val: LlrtQjsModule) -> Self {
        ModuleInfo {
            name: "llrt:qjs",
            module: val,
        }
    }
}
