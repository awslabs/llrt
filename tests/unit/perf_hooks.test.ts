import defaultImport from "node:perf_hooks";
import legacyImport from "perf_hooks";

it("node:perf_hooks should be the same as perf_hooks", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { performance } = defaultImport;

it("performance should be the same as globalThis.performance", () => {
  expect(performance).toStrictEqual(globalThis.performance);
});
