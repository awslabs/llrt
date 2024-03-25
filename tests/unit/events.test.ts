import { EventEmitter } from "events";

it("should use custom EventEmitter", () => {
  let called = 0;
  const symbolA = Symbol();
  const symbolB = Symbol();
  const symbolC = Symbol();
  const callback = () => {
    called++;
  };

  class MyEmitter extends EventEmitter {}
  const myEmitter = new MyEmitter();
  const myEmitter2 = new MyEmitter();

  myEmitter.once("event", function (a, b) {
    expect(a).toEqual("a");
    expect(b).toEqual("b");
    // @ts-ignore
    expect(this instanceof MyEmitter).toBeTruthy();
    // @ts-ignore
    expect(this === myEmitter).toBeTruthy();
    // @ts-ignore
    expect(this !== myEmitter2).toBeTruthy();
    called++;
  });

  myEmitter.on(symbolA, callback);
  myEmitter.on(symbolB, callback);
  myEmitter.on(symbolC, callback);

  myEmitter.emit("event", "a", "b");
  myEmitter.emit(symbolA);
  myEmitter.emit(symbolB);
  myEmitter.emit(symbolC);

  expect(called).toEqual(4);
  expect(myEmitter.eventNames()).toEqual([symbolA, symbolB, symbolC]);

  myEmitter.off(symbolB, callback);

  myEmitter.emit("event", "a", "b");
  myEmitter.emit(symbolA);
  myEmitter.emit(symbolB);
  myEmitter.emit(symbolC);

  expect(called).toEqual(6);
  expect(myEmitter.eventNames()).toEqual([symbolA, symbolC]);
});

it("should prepend event listeners", async () => {
  const myEmitter = new EventEmitter();

  const eventsArray: string[] = [];

  myEmitter.addListener("event", () => {
    eventsArray.push("added first");
  });
  myEmitter.prependListener("event", () => {
    eventsArray.push("added to beginning");
  });
  myEmitter.addListener("event", () => {
    eventsArray.push("last");
  });
  myEmitter.prependListener("event", () => {
    eventsArray.push("even before that");
  });

  myEmitter.emit("event");

  expect(eventsArray).toEqual([
    "even before that",
    "added to beginning",
    "added first",
    "last",
  ]);
});

it("should handle crash in event handler", () => {
  const emitter = new EventEmitter();

  emitter.on("data", () => {
    throw new Error("error");
  });

  expect(() => {
    emitter.emit("data", 123);
  }).toThrow();
});

it("should handle events emitted recursively", (done) => {
  const ee = new EventEmitter();

  ee.on("test", () => {
    ee.emit("test2");
  });

  ee.on("test2", done);

  ee.emit("test");
});

it("should set abort reason on AbortSignal", () => {
  const abortController = new AbortController();
  const signal = abortController.signal;

  abortController.abort("cancelled");

  expect(signal.aborted).toEqual(true);
  expect(signal.reason).toEqual("cancelled");
});
