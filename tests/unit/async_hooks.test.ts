import defaultImport from "node:async_hooks";
import { executionAsyncResource } from "node:async_hooks";
import legacyImport from "async_hooks";
import { spawnCapture } from "./test-utils";

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

it("should shut down cleanly after disabling a hook with callbacks", async () => {
  const { code } = await spawnCapture(process.argv0, [
    "-e",
    "import { createHook } from 'node:async_hooks'; const hook = createHook({ init() {}, before() {}, after() {}, promiseResolve() {}, destroy() {} }); hook.enable(); hook.disable()",
  ]);

  expect(code).toBe(0);
});
