import defaultImport from "node:async_hooks";
import { executionAsyncResource } from "node:async_hooks";
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

it("should assign distinct async IDs to timers sharing a callback", async () => {
  const executionIds: number[] = [];
  const hook = createHook({
    before() {
      executionIds.push(defaultImport.executionAsyncId());
    },
  });
  hook.enable();

  await new Promise<void>((resolve) => {
    let completed = 0;
    const callback = () => {
      completed++;
      if (completed === 2) resolve();
    };
    setTimeout(callback, 0);
    setTimeout(callback, 0);
  });
  hook.disable();

  expect(executionIds.length).toBeGreaterThanOrEqual(2);
  expect(executionIds[executionIds.length - 2]).not.toBe(
    executionIds[executionIds.length - 1]
  );
});

it("should use the current execution ID as a native trigger ID", async () => {
  let parentTimerId = 0;
  let nestedTriggerId = 0;
  const hook = createHook({
    init(asyncId, type, triggerAsyncId) {
      if (type !== "Timeout") return;
      if (parentTimerId === 0) {
        parentTimerId = asyncId;
      } else {
        nestedTriggerId = triggerAsyncId;
      }
    },
  });
  hook.enable();

  await new Promise<void>((resolve) => {
    setTimeout(() => setTimeout(resolve, 0), 0);
  });
  hook.disable();

  expect(nestedTriggerId).toBe(parentTimerId);
});

it("should expose the current async resource", async () => {
  let resource: object | undefined;
  const hook = createHook({
    before() {
      resource = executionAsyncResource();
    },
  });
  hook.enable();
  await new Promise((resolve) => setTimeout(resolve, 1));
  hook.disable();

  expect(resource).toBeDefined();
  expect(typeof resource).toBe("object");
});

it("should allow hook callbacks to register another hook", async () => {
  let nestedHookCreated = false;
  const hook = createHook({
    init() {
      if (!nestedHookCreated) {
        nestedHookCreated = true;
        createHook({}).enable();
      }
    },
  });
  hook.enable();

  await Promise.resolve();
  hook.disable();
  expect(nestedHookCreated).toBe(true);
});

it("should not retry a hook callback after it throws", () => {
  let calls = 0;
  const hook = createHook({
    init() {
      calls++;
      throw new Error("hook failure");
    },
  });
  hook.enable();

  Promise.resolve();
  hook.disable();
  expect(calls).toBe(1);
});

it("should manage hook lifecycle without duplicate callbacks", async () => {
  let calls = 0;
  const hook = createHook({
    init() {
      calls++;
    },
  });

  expect(hook.enable()).toBe(hook);
  expect(hook.enable()).toBe(hook);
  await Promise.resolve();
  const callsWhileEnabled = calls;

  expect(hook.disable()).toBe(hook);
  await Promise.resolve();
  expect(calls).toBe(callsWhileEnabled);

  expect(hook.enable()).toBe(hook);
  await Promise.resolve();
  expect(calls).toBeGreaterThan(callsWhileEnabled);
  expect(hook.disable()).toBe(hook);
});

it("should keep tracking while another hook is enabled", async () => {
  let firstCalls = 0;
  let secondCalls = 0;
  const firstHook = createHook({
    init() {
      firstCalls++;
    },
  });
  const secondHook = createHook({
    init() {
      secondCalls++;
    },
  });
  firstHook.enable();
  secondHook.enable();

  await Promise.resolve();
  firstHook.disable();
  const firstCallsAfterDisable = firstCalls;
  const secondCallsAfterDisable = secondCalls;
  await Promise.resolve();

  expect(firstCalls).toBe(firstCallsAfterDisable);
  expect(secondCalls).toBeGreaterThan(secondCallsAfterDisable);
  secondHook.disable();
});

it("should enter the promise execution context before callbacks", async () => {
  let observedExecutionId = 0;
  const hook = createHook({
    before() {
      observedExecutionId = defaultImport.executionAsyncId();
    },
  });
  hook.enable();
  await new Promise((resolve) => setTimeout(resolve, 1)).then(() => undefined);
  hook.disable();
  expect(observedExecutionId).toBeGreaterThan(1);
});
