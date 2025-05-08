const CWD = process.cwd();

async function waitAndExpectSequentialNumbers(tracked, delayMs) {
  await new Promise((resolve) => {
    setTimeout(() => {
      expect(tracked).toEqual(
        Array.from({ length: tracked.length }, (_, i) => i + 1)
      );
      resolve();
    }, delayMs);
  });
}

describe("Execution order of synchronous and asynchronous processes", () => {
  it("Synchronous operations have priority over microtasks.", async () => {
    const tracked = [];

    queueMicrotask(() => tracked.push(2));
    tracked.push(1);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("Synchronous operations have priority over macrotasks.", async () => {
    const tracked = [];

    setTimeout(() => tracked.push(2));
    tracked.push(1);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("Microtasks are executed in the order they are registered.", async () => {
    const tracked = [];

    Promise.resolve().then(() => tracked.push(1));
    queueMicrotask(() => tracked.push(2));

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("When a microtask occurs within a microtask, it is placed at the end of the accumulated microtasks.", async () => {
    const tracked = [];

    Promise.resolve().then(() => {
      tracked.push(1);
      queueMicrotask(() => tracked.push(3));
    });
    queueMicrotask(() => tracked.push(2));

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("If a macrotask and a microtask are registered at the same time, the microtask will take priority.", async () => {
    const tracked = [];

    setTimeout(() => tracked.push(3));
    Promise.resolve().then(() => tracked.push(1));
    queueMicrotask(() => tracked.push(2));

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("Macro tasks with different scheduled firing times are executed in the order of their scheduled firing times.", async () => {
    const tracked = [];

    setTimeout(() => tracked.push(3), 20);
    setTimeout(() => tracked.push(2), 10);
    setTimeout(() => tracked.push(1));

    await waitAndExpectSequentialNumbers(tracked, 30);
  });

  it("If a microtask occurs within a macrotask, all microtasks are executed before the next macrotask is executed.", async () => {
    const tracked = [];

    setTimeout(() => {
      tracked.push(1);
      queueMicrotask(() => tracked.push(2));
      queueMicrotask(() => tracked.push(3));
    });
    setTimeout(() => tracked.push(4));

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("When a new macrotask occurs among the macrotasks, it is placed at the end of the accumulated macrotasks.", async () => {
    const tracked = [];

    setTimeout(() => {
      tracked.push(1);
      setTimeout(() => tracked.push(3));
    });
    setTimeout(() => tracked.push(2));

    await waitAndExpectSequentialNumbers(tracked, 10);
  });
});

describe("Asynchronous Microtask Function", () => {
  it("If an asynchronous microtask function does not contain an await, it is executed entirely at the time of function registration.", async () => {
    const tracked = [];

    async function microTaskFunction() {
      tracked.push(1);
    }

    microTaskFunction();
    tracked.push(2);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("If an asynchronous microtask function contains an asynchronous operation without await, the subsequent operation will execute immediately because the asynchronous operation will not wait for completion.", async () => {
    const tracked = [];

    async function microTaskFunction() {
      Promise.resolve().then(() => tracked.push(3));
      tracked.push(1);
    }

    microTaskFunction();
    tracked.push(2);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("If an asynchronous microtask function contains an asynchronous operation with await, the subsequent operation will be executed after the asynchronous operation has finished.", async () => {
    const tracked = [];

    async function microTaskFunction() {
      await Promise.resolve().then(() => tracked.push(2));
      tracked.push(3);
    }

    microTaskFunction();
    tracked.push(1);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });
});

describe("Asynchronous Require/Import", () => {
  it("When a require is executed synchronously within the main thread, the following statements are executed immediately after the module is loaded.", async () => {
    const tracked = [];

    tracked.push(1);
    require(`${CWD}/fixtures/empty.js`);
    tracked.push(2);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("When require is placed inside a queueMicrotask, it executes after the current synchronous execution context finishes and before any scheduled macrotasks.", async () => {
    const tracked = [];

    queueMicrotask(() => {
      tracked.push(2);
      require(`${CWD}/fixtures/empty.js`);
      tracked.push(3);
    });
    queueMicrotask(() => tracked.push(4));

    tracked.push(1);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("When require is placed inside a setTimeout, it executes after all synchronous code and all microtasks have been completed, following the macrotask scheduling.", async () => {
    const tracked = [];

    setTimeout(() => {
      tracked.push(3);
      require(`${CWD}/fixtures/empty.js`);
      tracked.push(4);
    }, 0);
    queueMicrotask(() => tracked.push(2));

    tracked.push(1);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("When require is placed inside a Promise.then, it executes after the current synchronous execution context finishes but before any macrotasks, following the microtask scheduling rules.", async () => {
    const tracked = [];

    Promise.resolve().then(() => {
      tracked.push(2);
      require(`${CWD}/fixtures/empty.js`);
      tracked.push(3);
    });
    queueMicrotask(() => tracked.push(4));

    tracked.push(1);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("When multiple queueMicrotask calls contain require, they execute in the order they were scheduled, immediately after all synchronous code, according to FIFO (first-in, first-out) microtask queue behavior.", async () => {
    const tracked = [];

    queueMicrotask(() => {
      tracked.push(2);
      require(`${CWD}/fixtures/empty.js`);
      tracked.push(3);
    });
    queueMicrotask(() => {
      tracked.push(4);
      require(`${CWD}/fixtures/empty.js`);
      tracked.push(5);
    });

    tracked.push(1);

    await waitAndExpectSequentialNumbers(tracked, 10);
  });

  it("When using await import(), the code following the await resumes only after all earlier queued microtasks have been executed, as await yields execution back to the event loop.", async () => {
    const tracked = [];

    (async () => {
      tracked.push(1);
      await import(`${CWD}/fixtures/empty.js`);
      tracked.push(4);
    })();

    queueMicrotask(() => tracked.push(3));
    tracked.push(2);

    await waitAndExpectSequentialNumbers(tracked, 20);
  });
});
