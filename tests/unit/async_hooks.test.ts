import defaultImport from "node:async_hooks";
import legacyImport from "async_hooks";
import * as legacyNamedImport from "async_hooks";

const modules = {
  "node:async_hooks": defaultImport,
  async_hooks: legacyImport,
  "* as async_hooks": legacyNamedImport,
};

for (const module in modules) {
  const { createHook } = modules[module];
  describe(module, () => {
    let counters = {
      init: 0,
      before: 0,
      after: 0,
      promiseResolve: 0,
      destroy: 0,
    };

    createHook({
      init(asyncId, type, triggerAsyncId) {
        counters.init++;
      },
      before(asyncId) {
        counters.before++;
      },
      after(asyncId) {
        counters.after++;
      },
      promiseResolve(asyncId) {
        counters.promiseResolve++;
      },
      destroy(asyncId) {
        counters.destroy++;
      },
    }).enable();

    it("should track async operations", async () => {
      await new Promise((resolve) => setTimeout(resolve, 10));

      // It detects asynchronous operations in all tests that run simultaneously,
      // making it impossible to test them individually.
      // Therefore, here we only check whether asynchronous operations can be tracked.
      expect(counters.init).toBeGreaterThan(0);
      expect(counters.before).toBeGreaterThan(0);
      expect(counters.after).toBeGreaterThan(0);
      expect(counters.promiseResolve).toBeGreaterThan(0);
      expect(counters.destroy).toBeGreaterThan(0);
    });
  });
}
