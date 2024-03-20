import * as timers from "timers";
import { Console as NodeConsole } from "node:console";
import { Console } from "console";

function log(...args: any[]) {
  return (console as any).__formatPlain(...args);
}

it("should log module", () => {
  let module = log(timers);

  assert.equal(
    module,
    `
{
  clearInterval: [function: (anonymous)],
  clearTimeout: [function: (anonymous)],
  default: {
    setTimeout: [function: (anonymous)],
    clearTimeout: [function: (anonymous)],
    setInterval: [function: (anonymous)],
    clearInterval: [function: (anonymous)]
  },
  setInterval: [function: (anonymous)],
  setTimeout: [function: (anonymous)]
}
`.trim()
  );
});
it("should log using console object", () => {
  const consoleObj = new Console({
    stdout: process.stdout,
    stderr: process.stderr,
  });

  // we check if the log does not throw an exception when called
  consoleObj.log("log");
  consoleObj.debug("debug");
  consoleObj.info("info");
  consoleObj.assert(false, "text for assertion should display");
  consoleObj.assert(true, "This text should not be seen");

  consoleObj.warn("warn");
  consoleObj.error("error");
  consoleObj.trace("trace");
});

it("should log using node:console object", () => {
  const consoleObj = new NodeConsole({
    stdout: process.stdout,
    stderr: process.stderr,
  });

  // we check if the log does not throw an exception when called
  consoleObj.log("log");
  consoleObj.debug("debug");
  consoleObj.info("info");
  consoleObj.assert(false, "text for assertion should display");
  consoleObj.assert(true, "This text should not be seen");

  consoleObj.warn("warn");
  consoleObj.error("error");
  consoleObj.trace("trace");
});

it("should log complex object", () => {
  let date = new Date();

  let func = () => {};
  let ClassType = class Instance {};
  let instance = new ClassType();

  const obj = {
    a: 1,
    b: "foo",
    c: {
      d: date,
      e: [2.2, true, [], {}],
      f: {
        g: 1,
        h: 2,
        i: 3,
        j: {
          k: {
            l: "foo",
          },
          m: [1, 2, 3],
        },
      },
      n: [1, 2, 3],
    },
    o: {},
    p: new (class {})(),
    q: new (class Foo {})(),
    r: () => {},
    s: function () {},
    t: function foo() {},
    u: func,
    v: instance,
    x: ClassType,
    y: null,
    z: undefined,
    1: Symbol.for("foo"),
    2: new Promise(() => {}),
    3: {},
    4: [1, 2, 3],
    abc: 123,
  };

  // Add a circular reference
  obj.o = obj;

  const stringObj = log(obj);

  assert.equal(
    stringObj,
    `
{
  1: Symbol(foo),
  2: Promise {},
  3: {},
  4: [ 1, 2, 3 ],
  a: 1,
  b: \'foo\',
  c: {
    d: ${date.toISOString()},
    e: [ 2.2, true, [], {} ],
    f: { g: 1, h: 2, i: 3, j: { k: [Object], m: [Object] } },
    n: [ 1, 2, 3 ]
  },
  o: [Circular],
  p: {},
  q: Foo {},
  r: [function: r],
  s: [function: s],
  t: [function: foo],
  u: [function: func],
  v: Instance {},
  x: [class: Instance],
  y: null,
  z: undefined,
  abc: 123
}
`.trim()
  );
});
