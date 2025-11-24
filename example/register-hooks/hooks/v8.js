import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "v8") {
      return {
        url: "v8",
        shortCircuit: true,
      };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url === "v8") {
      const code = `
        import { ComputeMemoryUsage } from "llrt:qjs";

        export function getHeapStatistics() {
          const usage = ComputeMemoryUsage();

          return {
            total_heap_size: usage.memory_used_size,
            total_heap_size_executable: 0,
            total_physical_size: 0,
            total_available_size: 0,
            used_heap_size: usage.memory_used_size,
            heap_size_limit: usage.malloc_limit,
            malloced_memory: usage.malloc_size,
            peak_malloced_memory: 0,
            does_zap_garbage: 0,
            number_of_native_contexts: 0,
            number_of_detached_contexts: 0,
            total_global_handles_size: 0,
            used_global_handles_size: 0,
            external_memory: 0,
          };
        }
      `;

      return { format: "module", shortCircuit: true, source: code };
    }
    return nextLoad(url, context);
  },
});
