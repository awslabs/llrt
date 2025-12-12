import defaultImport from "node:async_hooks";
import legacyImport from "async_hooks";

it("node:async_hooks should be the same as async_hooks", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { createHook } = defaultImport;

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

  // destroy callbacks require GC + event loop tick to fire reliably
  __gc();
  await new Promise((resolve) => setTimeout(resolve, 1));
  expect(counters.destroy).toBeGreaterThan(0);
});
