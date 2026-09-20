import { AsyncLocalStorage } from "node:async_hooks";

// Ported from Node.js v22.x test/parallel/test-async-local-storage-contexts.js.
it("preserves the store across await", async () => {
  const storage = new AsyncLocalStorage<{ test: string }>();

  await storage.run({ test: "main context" }, async () => {
    expect(storage.getStore()?.test).toBe("main context");
    await 42;
    expect(storage.getStore()?.test).toBe("main context");
  });
});

// Ported from Node.js v22.x test/parallel/test-async-local-storage-deep-stack.js.
it("supports deeply nested run calls", () => {
  const storage = new AsyncLocalStorage<Record<string, never>>();

  const run = (count: number): void => {
    if (count !== 0) {
      storage.run({}, () => run(count - 1));
      return;
    }

    expect(storage.getStore()).toEqual({});
  };

  run(100);
});

// Ported from Node.js v22.x test/parallel/test-async-local-storage-exit-does-not-leak.js.
it("does not leak a store through nested exit calls", () => {
  const storage = new AsyncLocalStorage<boolean>();
  const data = true;

  const run = (count: number): void => {
    if (count === 0) {
      expect(storage.getStore()).not.toBe(data);
      return;
    }

    storage.run(data, () => {
      storage.exit(run, count - 1);
    });
  };

  run(100);
});

// Ported from Node.js v22.x test/parallel/test-async-local-storage-isolation.js.
it("keeps multiple AsyncLocalStorage instances isolated", () => {
  const first = new AsyncLocalStorage<string>();
  const second = new AsyncLocalStorage<string>();
  const third = new AsyncLocalStorage<string>();

  expect(first.getStore()).toBeUndefined();
  expect(second.getStore()).toBeUndefined();

  first.run("store1", () => {
    expect(first.getStore()).toBe("store1");
    expect(second.getStore()).toBeUndefined();

    second.run("store2", () => {
      expect(first.getStore()).toBe("store1");
      expect(second.getStore()).toBe("store2");
    });

    expect(first.getStore()).toBe("store1");
    expect(second.getStore()).toBeUndefined();

    second.enterWith("store3");
    expect(first.getStore()).toBe("store1");
    expect(second.getStore()).toBe("store3");
  });

  expect(first.getStore()).toBeUndefined();
  expect(second.getStore()).toBe("store3");

  third.enterWith("store3");
  first.run("store1", () => {
    expect(first.getStore()).toBe("store1");
    expect(second.getStore()).toBe("store3");
    expect(third.getStore()).toBe("store3");

    second.run("store2", () => {
      expect(first.getStore()).toBe("store1");
      expect(second.getStore()).toBe("store2");
      expect(third.getStore()).toBe("store3");

      first.disable();
      expect(first.getStore()).toBeUndefined();
      expect(second.getStore()).toBe("store2");
      expect(third.getStore()).toBe("store3");
    });

    expect(first.getStore()).toBeUndefined();
    expect(second.getStore()).toBe("store3");
    expect(third.getStore()).toBe("store3");
  });

  expect(first.getStore()).toBeUndefined();
  expect(second.getStore()).toBe("store3");
  expect(third.getStore()).toBe("store3");
});
