import defaultImport from "node:console";
import legacyImport from "console";

import * as timers from "node:timers";
import util from "node:util";

it("node:console should be the same as console", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { Console } = defaultImport;

it("should format strings correctly", () => {
  expect(util.format("%s:%s", "foo", "bar")).toEqual("foo:bar");
  expect(util.format("█", "foo")).toEqual("█ foo");
  expect(util.format(1, 2, 3)).toEqual("1 2 3");
  expect(util.format("%% %s")).toEqual("%% %s");
  expect(util.format("%s:%s", "foo")).toEqual("foo:%s");
  expect(util.format("Hello %%, %s! How are you, %s?", "Alice", "Bob")).toEqual(
    "Hello %, Alice! How are you, Bob?"
  );
  expect(util.format("The %s %d %f. %i", "quick", "42", "3.14", "abc")).toEqual(
    "The quick 42 3.14. NaN"
  );
  expect(
    util.format("Unmatched placeholders: %s %x %% %q", "one", "two")
  ).toEqual("Unmatched placeholders: one %x % %q two");
  expect(
    util.format("Unmatched placeholders: %s", "one", "two", "three")
  ).toEqual("Unmatched placeholders: one two three");

  // Should not throw any exceptions
  console.log("%s:%s", "foo", "bar");
});

it("should log module", () => {
  let module = util.format(timers);

  expect(module).toEqual(
    `
{
  clearInterval: [function: (anonymous)],
  clearTimeout: [function: (anonymous)],
  default: {
    setTimeout: [function: (anonymous)],
    clearTimeout: [function: (anonymous)],
    setInterval: [function: (anonymous)],
    clearInterval: [function: (anonymous)],
    setImmediate: [function: (anonymous)],
    queueMicrotask: [function: (anonymous)]
  },
  queueMicrotask: [function: (anonymous)],
  setImmediate: [function: (anonymous)],
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
          m: new Array(1000).fill(0),
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
    [3.14]: 1,
    4: [1, 2, 3],
    5: Promise.reject(1),
    6: Promise.resolve(1),
    abc: 123,
  };

  // Add a circular reference
  obj.o = obj;

  const stringObj = util.format(obj);

  expect(stringObj).toEqual(
    `
{
  '1': Symbol(foo),
  '2': Promise { <pending> },
  '3': {},
  '4': [ 1, 2, 3 ],
  '5': Promise {
    <rejected> 1
  },
  '6': Promise {
    1
  },
  a: 1,
  b: \'foo\',
  c: {
    d: ${date.toISOString()},
    e: [ 2.2, true, [], {} ],
    f: { g: 1, h: 2, i: 3, j: { k: [Object], m: [Array] } },
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
  '3.14': 1,
  abc: 123
}
`.trim()
  );
});

it("should log Headers", () => {
  const headers = new Headers();
  headers.append("foo", "bar");
  expect(util.format(headers)).toEqual(`Headers {
  foo: 'bar'
}`);
});

it("should handle broken utf8 surrogate pairs", () => {
  const s = "🌍🌎🌏";
  expect(util.format(s)).toEqual(s);
  expect(util.format(s.slice(1))).toEqual("�🌎🌏");

  // Test single emoji
  expect(util.format("🌍")).toEqual("🌍");

  // Test broken surrogate at end
  expect(util.format("abc🌍".slice(0, 4))).toEqual("abc�");

  // Test multiple broken surrogates
  const broken = "🌍".slice(0, 1) + "🌎".slice(0, 1) + "🌏";
  expect(util.format(broken)).toEqual("��🌏");

  // Test mixing regular chars and emojis
  expect(util.format("a🌍b🌎c")).toEqual("a🌍b🌎c");
  expect(util.format("a🌍b🌎c".slice(2))).toEqual("�b🌎c");
});
