import * as timers from "timers";

function log(...args: any[]) {
  return (console as any).__formatPlain(...args);
}

it("should log module", () => {
  let module = log(timers);

  expect(module).toEqual(    `
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
  )
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

  expect(stringObj).toEqual(    `
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
  )

});
