import defaultImport from "node:async_hooks";
import {
  AsyncLocalStorage,
  AsyncResource,
  executionAsyncId,
  executionAsyncResource,
  triggerAsyncId,
} from "node:async_hooks";
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

it("supports AsyncResource IDs and runInAsyncScope", async () => {
  const storage = new AsyncLocalStorage<string>();
  const { resource, expectedTriggerId } = storage.run("resource-store", () => ({
    resource: new AsyncResource("test"),
    expectedTriggerId: executionAsyncId(),
  }));

  expect(resource.asyncId()).toBeGreaterThan(1);
  expect(resource.triggerAsyncId()).toBeGreaterThan(1);
  expect(
    resource.runInAsyncScope(() => [storage.getStore(), executionAsyncId()])
  ).toEqual(["resource-store", resource.asyncId()]);
  await expect(
    resource.runInAsyncScope(async () => {
      await Promise.resolve();
      return storage.getStore();
    })
  ).resolves.toBe("resource-store");
  expect(resource.triggerAsyncId()).toBe(expectedTriggerId);
});

it("supports an explicit AsyncResource trigger ID and thisArg", () => {
  const parent = new AsyncResource("parent");
  const resource = new AsyncResource("child", {
    triggerAsyncId: parent.asyncId(),
  });

  expect(resource.triggerAsyncId()).toBe(parent.asyncId());
  expect(
    resource.runInAsyncScope(
      function (this: { value: string }, suffix: string) {
        return [
          `${this.value}-${suffix}`,
          executionAsyncId(),
          executionAsyncResource(),
        ];
      },
      { value: "bound" },
      "value"
    )
  ).toEqual(["bound-value", resource.asyncId(), resource]);
});

it("binds callbacks to instance and static AsyncResource scopes", () => {
  const storage = new AsyncLocalStorage<string>();
  const instanceBound = storage.run("instance-store", () => {
    const resource = new AsyncResource("instance-bound");
    return resource.bind(
      function (this: { prefix: string }, value: string) {
        return [this.prefix, value, storage.getStore(), executionAsyncId()];
      },
      { prefix: "instance" }
    );
  });
  const staticBound = storage.run("static-store", () =>
    AsyncResource.bind(
      function (this: { prefix: string }, value: string) {
        return [this.prefix, value, storage.getStore(), executionAsyncId()];
      },
      "static-bound",
      { prefix: "static" }
    )
  );

  expect(storage.run("outside", () => instanceBound("call"))).toEqual([
    "instance",
    "call",
    "instance-store",
    expect.any(Number),
  ]);
  expect(storage.run("outside", () => staticBound("call"))).toEqual([
    "static",
    "call",
    "static-store",
    expect.any(Number),
  ]);
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
