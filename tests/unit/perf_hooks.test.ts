import { performance } from "perf_hooks";

describe("perf_hooks", () => {
  it("perf_hooks.performance should be the same as globalThis.performance", () => {
    expect(performance).toBe(globalThis.performance);
  });
});
