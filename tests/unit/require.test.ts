globalThis._require = require;

const CWD = process.cwd();

it("should require a file", () => {
  const { hello } = _require(`${CWD}/fixtures/hello.js`);

  expect(hello).toEqual("hello world!")
});

it("should return same module when require multiple files", () => {
  const { hello: hello1 } = _require(`${CWD}/fixtures/hello.js`);
  const { hello: hello2 } = _require(`${CWD}/fixtures/hello.js`);
  const { hello: hello3 } = _require(`${CWD}/fixtures/hello.js`);

  expect(hello1).toEqual(hello2)
  expect(hello1).toEqual(hello3)
});

it("should handle cyclic requires", () => {
  const a = _require(`${CWD}/fixtures/a.js`);
  const b = _require(`${CWD}/fixtures/b.js`);

  expect(a.done).toEqual(b.done)
});

it("should handle cjs requires", () => {
  const a = _require(`${CWD}/fixtures/import.cjs`);

  expect(a.c).toEqual("c")
});
