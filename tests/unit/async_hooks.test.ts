const async_hooks = require("async_hooks");

describe("async_hooks", () => {
  let counters = {
    init: 0,
    before: 0,
    after: 0,
    promiseResolve: 0,
    destroy: 0,
  };

  async_hooks
    .createHook({
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
    })
    .enable();

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
