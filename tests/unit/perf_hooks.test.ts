import defaultImport from "node:perf_hooks";
import legacyImport from "perf_hooks";
import * as legacyNamedImport from "perf_hooks";

const modules = {
  "node:perf_hooks": defaultImport,
  perf_hooks: legacyImport,
  "* as perf_hooks": legacyNamedImport,
};
for (const module in modules) {
  const { performance } = modules[module];

  describe(module, () => {
    it("performance should be the same as globalThis.performance", () => {
      expect(performance).toBe(globalThis.performance);
    });
  });
}
