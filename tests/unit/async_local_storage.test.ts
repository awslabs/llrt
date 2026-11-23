import asyncHooks from "node:async_hooks";
import childProcess from "node:child_process";
import crypto from "node:crypto";
import dgram from "node:dgram";
import net from "node:net";
import zlib from "node:zlib";

const { AsyncLocalStorage } = asyncHooks;

test("propagates stores through promise continuations", async () => {
  const storage = new AsyncLocalStorage();
  const store = { requestId: "request-1" };

  const result = storage.run(store, async () => {
    expect(storage.getStore()).toBe(store);
    await Promise.resolve();
    return storage.getStore();
  });

  const afterRun = storage.getStore();
  const value = await result;
  const afterPromise = storage.getStore();

  expect(value).toBe(store);
  expect(afterRun).toBeUndefined();
  expect(afterPromise).toBeUndefined();
});

test("propagates ALS through Web Streams tee and pipe jobs", async () => {
  const storage = new AsyncLocalStorage<string>();
  const writeStores: Array<string | undefined> = [];
  const pipe = storage.run("pipe", async () => {
    const source = new ReadableStream({
      start(controller) {
        controller.enqueue("chunk");
        controller.close();
      },
    });
    const [first, second] = source.tee();
    const createSink = () =>
      new WritableStream({
        write() {
          writeStores.push(storage.getStore());
        },
      });

    await Promise.all([
      first.pipeTo(createSink()),
      second.pipeTo(createSink()),
    ]);
  });

  await pipe;
  expect(writeStores).toEqual(["pipe", "pipe"]);
  storage.disable();
});

test("restores the caller store immediately after an async run returns", async () => {
  const storage = new AsyncLocalStorage<string>();
  storage.enterWith("caller");

  let inside: string | undefined;
  let afterAwait: string | undefined;
  const result = storage.run("async", async () => {
    inside = storage.getStore();
    await Promise.resolve();
    afterAwait = storage.getStore();
  });

  const afterRun = storage.getStore();
  await result;
  const afterPromise = storage.getStore();

  expect([inside, afterRun, afterAwait, afterPromise]).toEqual([
    "async",
    "caller",
    "async",
    "caller",
  ]);
});

test("isolates stores across concurrent timer continuations", async () => {
  const storage = new AsyncLocalStorage<{ requestId: string }>();

  const handleRequest = (requestId: string) =>
    storage.run({ requestId }, async () => {
      await new Promise((resolve) => setTimeout(resolve, 1));
      return storage.getStore()?.requestId;
    });

  await expect(
    Promise.all([handleRequest("request-1"), handleRequest("request-2")])
  ).resolves.toEqual(["request-1", "request-2"]);
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

test("keeps other storage tracking active after disable", async () => {
  const firstStorage = new AsyncLocalStorage();
  const secondStorage = new AsyncLocalStorage();

  firstStorage.enterWith("first");
  secondStorage.enterWith("second");
  firstStorage.disable();

  await Promise.resolve();
  expect(firstStorage.getStore()).toBeUndefined();
  expect(secondStorage.getStore()).toBe("second");
  secondStorage.disable();
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

test("propagates ALS through Node net callbacks", async () => {
  const storage = new AsyncLocalStorage<{ side: string }>();
  const createdStore = { side: "created" };
  const serverStore = { side: "server" };
  const clientCreatedStore = { side: "client-created" };
  const clientStore = { side: "client" };
  let serverConnectionStore: { side: string } | undefined;
  let serverDataStore: { side: string } | undefined;
  let serverWriteStore: { side: string } | undefined;
  let clientConnectStore: { side: string } | undefined;
  let clientWriteStore: { side: string } | undefined;
  let clientDataStore: { side: string } | undefined;

  const server = storage.run(createdStore, () =>
    net.createServer((socket) => {
      serverConnectionStore = storage.getStore();
      socket.on("data", (data) => {
        serverDataStore = storage.getStore();
        socket.write(data, () => {
          serverWriteStore = storage.getStore();
        });
      });
    })
  );

  const finished = new Promise<void>((resolve, reject) => {
    storage.run(serverStore, () => {
      server.listen(() => {
        const client = storage.run(clientCreatedStore, () => new net.Socket());
        client.on("error", reject);
        client.on("data", (data) => {
          clientDataStore = storage.getStore();
          expect(data.toString()).toBe("als");
          client.end(() => server.close(resolve));
        });
        storage.run(clientStore, () =>
          client.connect((server.address() as any).port, "127.0.0.1", () => {
            clientConnectStore = storage.getStore();
            client.write("als", () => {
              clientWriteStore = storage.getStore();
            });
          })
        );
      });
    });
  });

  await finished;
  expect(serverConnectionStore).toBe(serverStore);
  expect(serverDataStore).toBe(serverStore);
  expect(serverWriteStore).toBe(serverStore);
  expect(clientConnectStore).toBe(clientStore);
  expect(clientWriteStore).toBe(clientStore);
  expect(clientDataStore).toBe(clientStore);
  storage.disable();
});

test("unwinds native callback scopes when ALS is disabled inside the callback", async () => {
  const storage = new AsyncLocalStorage<string>();
  const timer = new Promise<void>((resolve) => {
    storage.run("timer", () =>
      setTimeout(() => {
        expect(storage.getStore()).toBe("timer");
        storage.disable();
        resolve();
      }, 0)
    );
  });

  await timer;
  expect(storage.getStore()).toBeUndefined();
  await expect(
    storage.run("restored", async () => {
      await Promise.resolve();
      return storage.getStore();
    })
  ).resolves.toBe("restored");
});

test("propagates ALS through crypto randomFill callbacks", async () => {
  const storage = new AsyncLocalStorage<{ requestId: string }>();
  const store = { requestId: "crypto" };
  const callbackStore = await new Promise<typeof store>((resolve, reject) => {
    storage.run(store, () =>
      crypto.randomFill(Buffer.alloc(8), (error) => {
        if (error) reject(error);
        else resolve(storage.getStore() as typeof store);
      })
    );
  });

  expect(callbackStore).toBe(store);
  storage.disable();
});

test("propagates ALS through dgram send callbacks", async () => {
  const storage = new AsyncLocalStorage<{ requestId: string }>();
  const store = { requestId: "dgram-send" };
  const server = dgram.createSocket("udp4");
  const client = dgram.createSocket("udp4");

  try {
    const port = await new Promise<number>((resolve, reject) => {
      server.once("error", reject);
      server.bind(0, () => resolve(server.address().port));
    });
    const received = new Promise<void>((resolve, reject) => {
      server.once("error", reject);
      server.once("message", (message) => {
        try {
          expect(message.toString()).toBe("als");
          resolve();
        } catch (error) {
          reject(error);
        }
      });
    });
    const callbackStore = await new Promise<typeof store>((resolve, reject) => {
      storage.run(store, () =>
        client.send("als", port, "127.0.0.1", (error) => {
          if (error) reject(error);
          else resolve(storage.getStore() as typeof store);
        })
      );
    });

    await received;
    expect(callbackStore).toBe(store);
  } finally {
    storage.disable();
    client.close();
    server.close();
  }
});

test("propagates ALS through child-process exit events", async () => {
  const storage = new AsyncLocalStorage<{ requestId: string }>();
  const store = { requestId: "child-process" };
  const callbackStore = await new Promise<typeof store>((resolve, reject) => {
    storage.run(store, () => {
      const child = childProcess.spawn("echo", ["als"]);
      child.once("error", reject);
      child.once("exit", () => resolve(storage.getStore() as typeof store));
    });
  });

  expect(callbackStore).toBe(store);
  storage.disable();
});

test("propagates ALS through nested zlib callbacks", async () => {
  const storage = new AsyncLocalStorage<{ requestId: string }>();
  const store = { requestId: "zlib" };

  try {
    await storage.run(store, async () => {
      const compressed = await new Promise<any>((resolve, reject) => {
        zlib.deflate("Hello LLRT!!", (error, result) => {
          if (error) reject(error);
          else resolve(result);
        });
      });
      expect(storage.getStore()).toBe(store);

      const decompressed = await new Promise<any>((resolve, reject) => {
        zlib.inflate(compressed, (error, result) => {
          if (error) reject(error);
          else resolve(result);
        });
      });

      expect(decompressed.toString()).toBe("Hello LLRT!!");
      expect(storage.getStore()).toBe(store);
    });
  } finally {
    storage.disable();
  }
});
