import asyncHooks from "node:async_hooks";

const { AsyncLocalStorage } = asyncHooks;

test("propagates stores through promise continuations", async () => {
  const storage = new AsyncLocalStorage();
  const store = { requestId: "request-1" };

  const result = storage.run(store, async () => {
    expect(storage.getStore()).toBe(store);
    await Promise.resolve();
    return storage.getStore();
  });

  expect(await result).toBe(store);
});

test("restores the previous store after a synchronous run", () => {
  const storage = new AsyncLocalStorage();
  const store = "request-1";

  expect(
    storage.run(store, (prefix, suffix) => `${prefix}-${suffix}`, "request", 1)
  ).toBe("request-1");
  expect(storage.getStore()).toBeUndefined();
});

test("restores the store when run throws", () => {
  const storage = new AsyncLocalStorage();
  storage.enterWith("outer");

  expect(() =>
    storage.run("inner", () => {
      expect(storage.getStore()).toBe("inner");
      throw new Error("callback failure");
    })
  ).toThrow("callback failure");
  expect(storage.getStore()).toBe("outer");
});

test("restores nested run contexts", () => {
  const storage = new AsyncLocalStorage();
  storage.enterWith("outer");

  expect(
    storage.run("inner", () => {
      expect(storage.getStore()).toBe("inner");
      return storage.run("nested", () => storage.getStore());
    })
  ).toBe("nested");
  expect(storage.getStore()).toBe("outer");
});

test("supports enterWith and disable", async () => {
  const storage = new AsyncLocalStorage();
  const store = "active";

  expect(storage.enterWith(store)).toBeUndefined();
  expect(storage.getStore()).toBe(store);
  await Promise.resolve();
  expect(storage.getStore()).toBe(store);

  expect(storage.disable()).toBe(storage);
  expect(storage.getStore()).toBeUndefined();

  const reactivated = storage.run("reactivated", async () => {
    await Promise.resolve();
    return storage.getStore();
  });
  expect(await reactivated).toBe("reactivated");
  storage.enterWith("active-again");
  expect(storage.getStore()).toBe("active-again");
});

test("temporarily exits and restores the current store", () => {
  const storage = new AsyncLocalStorage();
  storage.enterWith("outer");

  expect(storage.exit(() => storage.getStore())).toBeUndefined();
  expect(
    storage.exit((value) => `${storage.getStore()}-${value}`, "value")
  ).toBe("undefined-value");
  expect(storage.getStore()).toBe("outer");
});

test("restores an empty context after exit changes the store", () => {
  const storage = new AsyncLocalStorage();

  expect(
    storage.exit(() => {
      storage.enterWith("temporary");
      return storage.getStore();
    })
  ).toBe("temporary");
  expect(storage.getStore()).toBeUndefined();
});

test("supports defaultValue and name options", () => {
  const storage = new AsyncLocalStorage({
    defaultValue: "default",
    name: "requests",
  });

  expect(storage.name).toBe("requests");
  expect(storage.getStore()).toBe("default");
  expect(storage.run("active", () => storage.getStore())).toBe("active");
  expect(storage.getStore()).toBe("default");
});

test("supports bind and snapshot", () => {
  const storage = new AsyncLocalStorage();
  storage.enterWith("captured");

  const bound = AsyncLocalStorage.bind(function (value: string) {
    return `${this.prefix}-${value}-${storage.getStore()}`;
  });
  const runInCaptured = AsyncLocalStorage.snapshot();
  storage.enterWith("changed");

  expect(bound.call({ prefix: "bound" }, "value")).toBe("bound-value-captured");
  expect(
    runInCaptured.call(
      { prefix: "snapshot" },
      function (value: string) {
        return `${this.prefix}-${value}-${storage.getStore()}`;
      },
      "value"
    )
  ).toBe("snapshot-value-captured");
});

test("does not retain discarded storage registrations", async () => {
  for (let index = 0; index < 100; index++) {
    const storage = new AsyncLocalStorage();
    await storage.run(index, async () => {
      await Promise.resolve();
    });
  }

  __gc();
  await new Promise((resolve) => setTimeout(resolve, 1));

  const storage = new AsyncLocalStorage();
  await storage.run("live", async () => {
    await Promise.resolve();
    expect(storage.getStore()).toBe("live");
  });
});
