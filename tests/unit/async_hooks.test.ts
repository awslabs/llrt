import defaultImport from "node:async_hooks";
import { executionAsyncId, triggerAsyncId } from "node:async_hooks";
import legacyImport from "async_hooks";
import { spawnCapture } from "./test-utils";

it("node:async_hooks should be the same as async_hooks", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

it("should use the current execution ID as a native trigger ID", async () => {
  const storage = new defaultImport.AsyncLocalStorage();
  const [parentTimerId, nestedTriggerId] = await storage.run(
    "test",
    () =>
      new Promise<[number, number]>((resolve) => {
        setTimeout(() => {
          const parentTimerId = executionAsyncId();
          setTimeout(() => resolve([parentTimerId, triggerAsyncId()]), 0);
        }, 0);
      })
  );

  expect(parentTimerId).toBeGreaterThan(1);
  expect(nestedTriggerId).toBe(parentTimerId);
});

it("should shut down cleanly after calling legacy hook methods", async () => {
  const { code } = await spawnCapture(process.argv0, [
    "-e",
    "import { createHook } from 'node:async_hooks'; const hook = createHook({}); hook.enable(); hook.disable()",
  ]);

  expect(code).toBe(0);
});

it("should shut down cleanly after using AsyncLocalStorage.run", async () => {
  const { code } = await spawnCapture(process.argv0, [
    "-e",
    "import { AsyncLocalStorage } from 'node:async_hooks'; const storage = new AsyncLocalStorage(); storage.run('store', () => 1);",
  ]);

  expect(code).toBe(0);
});
