import { ComputeMemoryUsage } from "llrt:qjs";

describe("ComputeMemoryUsage", () => {
  it("should exist function and should be available", () => {
    const usage = ComputeMemoryUsage();

    expect(usage.malloc_size).toBeGreaterThanOrEqual(0);
    expect(usage.malloc_limit).toBeGreaterThanOrEqual(0);
    expect(usage.memory_used_size).toBeGreaterThanOrEqual(0);
    expect(usage.malloc_count).toBeGreaterThanOrEqual(0);
    expect(usage.memory_used_count).toBeGreaterThanOrEqual(0);
    expect(usage.atom_count).toBeGreaterThanOrEqual(0);
    expect(usage.atom_size).toBeGreaterThanOrEqual(0);
    expect(usage.str_count).toBeGreaterThanOrEqual(0);
    expect(usage.str_size).toBeGreaterThanOrEqual(0);
    expect(usage.obj_count).toBeGreaterThanOrEqual(0);
    expect(usage.obj_size).toBeGreaterThanOrEqual(0);
    expect(usage.prop_count).toBeGreaterThanOrEqual(0);
    expect(usage.prop_size).toBeGreaterThanOrEqual(0);
    expect(usage.shape_count).toBeGreaterThanOrEqual(0);
    expect(usage.shape_size).toBeGreaterThanOrEqual(0);
    expect(usage.js_func_count).toBeGreaterThanOrEqual(0);
    expect(usage.js_func_size).toBeGreaterThanOrEqual(0);
    expect(usage.js_func_code_size).toBeGreaterThanOrEqual(0);
    expect(usage.js_func_pc2line_count).toBeGreaterThanOrEqual(0);
    expect(usage.js_func_pc2line_size).toBeGreaterThanOrEqual(0);
    expect(usage.c_func_count).toBeGreaterThanOrEqual(0);
    expect(usage.array_count).toBeGreaterThanOrEqual(0);
    expect(usage.fast_array_count).toBeGreaterThanOrEqual(0);
    expect(usage.fast_array_elements).toBeGreaterThanOrEqual(0);
    expect(usage.binary_object_count).toBeGreaterThanOrEqual(0);
    expect(usage.binary_object_size).toBeGreaterThanOrEqual(0);
  });
});
